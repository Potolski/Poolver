//! `claim_winning` — spec §5.1, step 8.
//!
//! ## Architectural shape (arch §2.2 + §4 + §5)
//!
//! `select_winner` (step 7) writes the `MonthWinner` slot but defers the
//! `Participant.has_won` flip and the actual token movements to this
//! instruction. The split exists so we can re-validate the winner against
//! `pool.winners[month-1].winner` AND require a fresh Full-KYC check at
//! claim time (defence-in-depth — KYC may have expired between selection
//! and claim).
//!
//! ## Token-flow contract (INV-1 / arch §12 solvency proof)
//!
//! Six token-touching steps; ordered so a partial failure leaves no
//! invariant violated (the ix is atomic — Solana reverts the whole tx —
//! but the *order* still matters for the in-tx accounting):
//!
//!   a) `yield_vault::withdraw(net_payout)`: pull `net_payout` USDC out
//!      of the Tier-0 yield adapter into `pool_usdc_vault`. Step 5's
//!      `contribute` deposits each net contribution into the adapter, so
//!      the funds physically live there until claim time.
//!   b) winner_usdc → collateral_vault: `total_collateral_required`. The
//!      winner signs this transfer directly (their own ATA authority).
//!   c) pool_usdc_vault → winner_usdc: `net_payout`. Pool USDC vault PDA
//!      signs.
//!   d) pool_usdc_vault → protocol_fee_vault: `protocol_share` (5% of
//!      `winning_bid`).
//!   e) reserve::deposit(`reserve_share` = 20% of winning_bid),
//!      source = pool_usdc_vault, signed by core_invoker + pool vault PDA.
//!   f) Virtual: credit `participant_share` (75% of winning_bid) into
//!      `pool.bid_credit_balance`. NO token movement — the funds stay in
//!      `pool_usdc_vault` to be drawn down via the Q-1 pro-rata formula
//!      in subsequent `contribute` calls.
//!
//! Solvency check after the ix:
//!
//! ```text
//!   Δpool_usdc_vault       = +net_payout (a) − net_payout (c)
//!                            − protocol_share (d) − reserve_share (e)
//!                          = −(protocol_share + reserve_share)
//!                          = −(25% × winning_bid)
//!   Δcollateral_vault      = +total_collateral_required (b)
//!   Δprotocol_fee_vault    = +protocol_share (d)
//!   Δreserve_usdc_vault    = +reserve_share (e)
//!   Δwinner_usdc           = +net_payout (c) − total_collateral_required (b)
//!   Δbid_credit_balance    = +participant_share (75% × winning_bid)
//!   Δyield_adapter_balance = −net_payout (a)
//! ```
//!
//! Sum across all USDC custody endpoints (yield_adapter + pool_vault +
//! collateral + protocol_fee + reserve_vault + winner_ata) is invariant
//! to within the (75% × winning_bid) virtual credit, which is *not* a
//! token move — the credit becomes `actual_paid_by_user` reductions in
//! later `contribute` calls. INV-1 holds.
//!
//! ## Collateral math (spec §4 + Q-7)
//!
//! ```text
//!   baseline             = (TOTAL_MONTHS − win_month) × contribution_amount
//!   reputation_bps       = match completed_cycles_at_join {
//!                              0 => 10_000,
//!                              1 =>  7_000,
//!                              _ =>  5_000,
//!                          }
//!   adjusted_baseline    = baseline × reputation_bps / 10_000
//!   bid_premium          = winning_bid × 2
//!   total_collateral     = adjusted_baseline + bid_premium
//! ```
//!
//! Q-7 architect default: reputation snapshot is read from
//! `participant.completed_cycles_at_join` — NOT live `user_reputation`.
//! Snapshotting at join time prevents grinding (the user could otherwise
//! finish a parallel pool first to lower their multiplier).
//!
//! ## Release schedule cache
//!
//! `participant.collateral_release_per_month = total_collateral_required
//! / months_remaining_at_win`. Cached on Participant so step 5's
//! `contribute` post-win release branch reads it without recomputing.
//!
//! ### Edge case: month-12 winner
//!
//! `months_remaining_at_win = TOTAL_MONTHS − 12 = 0`. The baseline term
//! collapses to 0, leaving only `bid_premium` collateral. There are no
//! future contributions to enforce against, so we **immediately refund**
//! the bid_premium back to the winner inside this same ix and zero out
//! the lock. SPEC_QUESTION-34: documented inline; QUESTIONS.md updated
//! with the architect default.
//!
//! ## Bid distribution (spec §4)
//!
//!   - 5%  → protocol_fee_vault (token move)
//!   - 20% → tier reserve via reserve::deposit (token move)
//!   - 75% → pool.bid_credit_balance (virtual credit; tokens stay in
//!           pool_usdc_vault and are drawn down via Q-1 pro-rata in
//!           future `contribute` calls)
//!
//! Computed via `winning_bid − protocol_share − reserve_share` so any
//! rounding error stays solvent (rounding goes INTO the participant
//! share, never out).
//!
//! ## Errors
//!
//!   - `NotWinner`              — caller != winners[m-1].winner OR not yet selected
//!   - `AlreadyClaimed`         — winners[m-1].claimed already true
//!   - `AlreadyWon`             — defense-in-depth: participant.has_won already
//!   - `CollateralInsufficient` — winner ATA balance < total_collateral_required
//!   - `KycInsufficientLevel` / `KycExpired` / `KycSanctionsHit` — Full-KYC gate
//!   - `Defaulted` / `Suspended` — participant lifecycle blockers
//!   - `ProtocolPaused` / `PoolComplete` / `PoolNotStarted` / `TierNotYetSupported`

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, Transfer};

use crate::constants::{
    BPS_DENOMINATOR, COLLATERAL_VAULT_SEED, CORE_INVOKER_SEED, KYC_SEED, PARTICIPANT_SEED,
    POOL_USDC_VAULT_SEED, PROTOCOL_CONFIG_SEED, PROTOCOL_FEE_VAULT_SEED, REPUTATION_SEED,
    RESERVE_FUND_SEED, RESERVE_VAULT_SEED,
};
use crate::error::CoreError;
use crate::events::{BidDistributed, WinningClaimed};
use crate::kyc::require_full_kyc;
use crate::state::{KycAttestation, Participant, Pool, ProtocolConfig, Tier, UserReputation};

/// Bid distribution constants (spec §4).
const PROTOCOL_BID_SHARE_BPS: u64 = 500;   // 5%
const RESERVE_BID_SHARE_BPS: u64 = 2_000;  // 20%
// Participant share (75%) is computed by subtraction so rounding never
// inflates the on-chain credit ledger.

/// Reputation multiplier table (spec §4 + Q-7).
#[inline]
fn reputation_multiplier_bps(completed_cycles: u8) -> u64 {
    match completed_cycles {
        0 => 10_000,
        1 => 7_000,
        _ => 5_000,
    }
}

/// SPEC_QUESTION-15: `Pool` is `Box`'d. `protocol_config` and `user_kyc`
/// are `UncheckedAccount` and manually deserialized in the handler to
/// keep the `try_accounts`-time stack frame under the 4 KB BPF budget.
/// Same trade-off as `join_pool`, `contribute`, `commit_bid`, `select_winner`.
#[derive(Accounts)]
pub struct ClaimWinning<'info> {
    /// The selected winner. Signs:
    ///   - the SPL transfer from their ATA into `collateral_vault`
    ///   - the tx fee
    /// Authorization: `winner.key() == pool.winners[current_month-1].winner`,
    /// enforced inside the handler.
    #[account(mut)]
    pub winner: Signer<'info>,

    /// Protocol config — manually deserialized (SPEC_QUESTION-15).
    /// CHECK: PDA seed binding here, owner+discriminator validated in
    /// handler.
    #[account(seeds = [PROTOCOL_CONFIG_SEED], bump)]
    pub protocol_config: UncheckedAccount<'info>,

    /// The pool. Mut because we write `winners[m-1].claimed`,
    /// `bid_credit_balance`, `total_distributed`.
    #[account(mut)]
    pub pool: Box<Account<'info, Pool>>,

    /// Per-(pool, winner) participant record. PDA seed binding doubles as
    /// the "is the caller a participant?" check.
    #[account(
        mut,
        seeds = [PARTICIPANT_SEED, pool.key().as_ref(), winner.key().as_ref()],
        bump = participant.bump,
        constraint = participant.pool == pool.key() @ CoreError::NotAParticipant,
        constraint = participant.user == winner.key() @ CoreError::NotAParticipant,
    )]
    pub participant: Box<Account<'info, Participant>>,

    /// Winner's reputation — `total_received_lifetime` is incremented.
    #[account(
        mut,
        seeds = [REPUTATION_SEED, winner.key().as_ref()],
        bump = user_reputation.bump,
    )]
    pub user_reputation: Box<Account<'info, UserReputation>>,

    /// Winner's KYC attestation — Full level required at claim time
    /// (defence-in-depth; `select_winner` already gated this but the
    /// attestation may have expired between selection and claim).
    /// CHECK: PDA seed binding; owner+discriminator+content validated in
    /// handler via manual deserialization (SPEC_QUESTION-15).
    #[account(seeds = [KYC_SEED, winner.key().as_ref()], bump)]
    pub user_kyc: UncheckedAccount<'info>,

    /// Winner's USDC ATA. Receives `net_payout`, sources `total_collateral_required`.
    /// CHECK: SPL transfer enforces ownership + balance. Validated as
    /// token account in CPI helper.
    #[account(mut)]
    pub winner_usdc: UncheckedAccount<'info>,

    /// Pool USDC vault. PDA-owned token account; signs both the payout
    /// transfer and the protocol-fee + reserve-deposit transfers.
    /// CHECK: PDA seed binding + key equality with `pool.pool_usdc_vault`.
    #[account(
        mut,
        seeds = [POOL_USDC_VAULT_SEED, pool.key().as_ref()],
        bump,
        constraint = pool_usdc_vault.key() == pool.pool_usdc_vault
            @ CoreError::Unauthorized,
    )]
    pub pool_usdc_vault: UncheckedAccount<'info>,

    /// Collateral vault. Receives `total_collateral_required`.
    /// CHECK: PDA seed binding + key equality with `pool.collateral_vault`.
    #[account(
        mut,
        seeds = [COLLATERAL_VAULT_SEED, pool.key().as_ref()],
        bump,
        constraint = collateral_vault.key() == pool.collateral_vault
            @ CoreError::Unauthorized,
    )]
    pub collateral_vault: UncheckedAccount<'info>,

    /// Protocol fee SPL vault. Receives 5% of `winning_bid`.
    /// CHECK: PDA seed binding; equality with `protocol_config.protocol_fee_vault`
    /// validated in handler.
    #[account(mut, seeds = [PROTOCOL_FEE_VAULT_SEED], bump)]
    pub protocol_fee_vault: UncheckedAccount<'info>,

    /// `core_invoker` PDA — co-signs reserve + yield-vault CPIs (arch §5.2).
    /// CHECK: AccountInfo only; bump validated by Anchor seeds.
    #[account(seeds = [CORE_INVOKER_SEED], bump)]
    pub core_invoker: UncheckedAccount<'info>,

    // ───── Reserve CPI accounts (validated by reserve via tier seed) ─────
    /// CHECK: validated by `poolver_reserve::deposit`; we additionally
    /// re-derive in the handler against `pool.tier` for INV-4.
    #[account(mut)]
    pub reserve_fund: UncheckedAccount<'info>,

    /// CHECK: validated by `poolver_reserve::deposit`.
    #[account(mut)]
    pub reserve_usdc_vault: UncheckedAccount<'info>,

    /// CHECK: hardcoded program ID.
    #[account(address = poolver_reserve::ID)]
    pub reserve_program: UncheckedAccount<'info>,

    // ───── Yield-adapter CPI accounts (SPEC_QUESTION-36) ─────
    /// CHECK: validated by the chosen adapter's `withdraw`.
    #[account(mut)]
    pub adapter_state: UncheckedAccount<'info>,

    /// CHECK: validated by the chosen adapter's `withdraw`.
    #[account(mut)]
    pub adapter_usdc_vault: UncheckedAccount<'info>,

    /// CHECK: SPEC_QUESTION-36 — adapter program ID validated against
    /// `pool.tier` in the handler via `cpi_adapter_withdraw`.
    pub yield_adapter_program: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

// ─── CPI helper frames (SPEC_QUESTION-15: split CPIs across stack frames) ─

// SPEC_QUESTION-36: per-instruction `cpi_yield_withdraw` removed in
// step 13. Tier dispatch lives in `crate::adapter_cpi::adapter::
// cpi_adapter_withdraw` and is invoked directly from the handler.

#[inline(never)]
fn cpi_winner_to_collateral<'info>(
    token_program: AccountInfo<'info>,
    winner_usdc: AccountInfo<'info>,
    collateral_vault: AccountInfo<'info>,
    winner: AccountInfo<'info>,
    amount: u64,
) -> Result<()> {
    let cpi_ctx = CpiContext::new(
        token_program.key(),
        Transfer {
            from: winner_usdc,
            to: collateral_vault,
            authority: winner,
        },
    );
    token::transfer(cpi_ctx, amount)
}

#[inline(never)]
fn cpi_pool_to_winner<'info>(
    token_program: AccountInfo<'info>,
    pool_usdc_vault: AccountInfo<'info>,
    winner_usdc: AccountInfo<'info>,
    pool_key: &Pubkey,
    pool_usdc_vault_bump: u8,
    amount: u64,
) -> Result<()> {
    let seeds: &[&[&[u8]]] = &[&[
        POOL_USDC_VAULT_SEED,
        pool_key.as_ref(),
        &[pool_usdc_vault_bump],
    ]];
    let cpi_ctx = CpiContext::new_with_signer(
        token_program.key(),
        Transfer {
            from: pool_usdc_vault.clone(),
            to: winner_usdc,
            authority: pool_usdc_vault,
        },
        seeds,
    );
    token::transfer(cpi_ctx, amount)
}

#[inline(never)]
fn cpi_pool_to_fee_vault<'info>(
    token_program: AccountInfo<'info>,
    pool_usdc_vault: AccountInfo<'info>,
    protocol_fee_vault: AccountInfo<'info>,
    pool_key: &Pubkey,
    pool_usdc_vault_bump: u8,
    amount: u64,
) -> Result<()> {
    let seeds: &[&[&[u8]]] = &[&[
        POOL_USDC_VAULT_SEED,
        pool_key.as_ref(),
        &[pool_usdc_vault_bump],
    ]];
    let cpi_ctx = CpiContext::new_with_signer(
        token_program.key(),
        Transfer {
            from: pool_usdc_vault.clone(),
            to: protocol_fee_vault,
            authority: pool_usdc_vault,
        },
        seeds,
    );
    token::transfer(cpi_ctx, amount)
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn cpi_reserve_deposit<'info>(
    reserve_program: AccountInfo<'info>,
    core_invoker: AccountInfo<'info>,
    reserve_fund: AccountInfo<'info>,
    reserve_usdc_vault: AccountInfo<'info>,
    pool_usdc_vault: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    pool_key: &Pubkey,
    core_invoker_bump: u8,
    pool_usdc_vault_bump: u8,
    amount: u64,
) -> Result<()> {
    let cpi_accounts = poolver_reserve::cpi::accounts::ReserveDepositCtx {
        core_invoker,
        reserve_fund,
        reserve_usdc_vault,
        source_usdc: pool_usdc_vault.clone(),
        source_authority: pool_usdc_vault,
        token_program,
    };
    let combined_seeds: &[&[&[u8]]] = &[
        &[CORE_INVOKER_SEED, &[core_invoker_bump]],
        &[
            POOL_USDC_VAULT_SEED,
            pool_key.as_ref(),
            &[pool_usdc_vault_bump],
        ],
    ];
    let cpi_ctx = CpiContext::new_with_signer(
        reserve_program.key(),
        cpi_accounts,
        combined_seeds,
    );
    poolver_reserve::cpi::deposit(cpi_ctx, amount)
}

/// Refund collateral from `collateral_vault` back to the winner. Used by
/// the month-12 immediate-refund edge case (SPEC_QUESTION-34).
#[inline(never)]
fn cpi_collateral_refund<'info>(
    token_program: AccountInfo<'info>,
    collateral_vault: AccountInfo<'info>,
    winner_usdc: AccountInfo<'info>,
    pool_key: &Pubkey,
    collateral_vault_bump: u8,
    amount: u64,
) -> Result<()> {
    let seeds: &[&[&[u8]]] = &[&[
        COLLATERAL_VAULT_SEED,
        pool_key.as_ref(),
        &[collateral_vault_bump],
    ]];
    let cpi_ctx = CpiContext::new_with_signer(
        token_program.key(),
        Transfer {
            from: collateral_vault.clone(),
            to: winner_usdc,
            authority: collateral_vault,
        },
        seeds,
    );
    token::transfer(cpi_ctx, amount)
}

pub fn handle_claim_winning<'info>(
    ctx: Context<'info, ClaimWinning<'info>>,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let pool_key = ctx.accounts.pool.key();
    let winner_key = ctx.accounts.winner.key();

    // ───── 1. Pool gates ───────────────────────────────────────────────
    let current_month: u8;
    let pool_tier: Tier;
    let winning_bid: u64;
    let stored_net_payout: u64;
    let stored_gross_payout: u64;
    let contribution_amount: u64;
    {
        let pool = &ctx.accounts.pool;
        require!(!pool.is_complete, CoreError::PoolComplete);
        require!(
            pool.current_month >= 1 && pool.current_month <= Pool::TOTAL_MONTHS,
            CoreError::PoolNotStarted
        );
        // SPEC_QUESTION-36: step 13 — Tier dispatch handled in CPI helper
        // below; this handler accepts both tiers.

        let m_idx = (pool.current_month as usize) - 1;
        let mw = pool.winners[m_idx];
        // Winner must have been selected: month != 0 sentinel + selected_at != 0.
        require!(mw.month != 0 && mw.selected_at != 0, CoreError::NotWinner);
        // INV "Single winner per month": once `claimed`, this gate is permanent.
        require!(!mw.claimed, CoreError::AlreadyClaimed);
        // Authorization: caller must be the selected winner.
        require_keys_eq!(mw.winner, winner_key, CoreError::NotWinner);

        current_month = pool.current_month;
        pool_tier = pool.tier;
        winning_bid = mw.winning_bid;
        stored_net_payout = mw.net_payout;
        stored_gross_payout = mw.gross_payout;
        contribution_amount = pool.contribution_amount;
    }

    // ───── 2. Participant gates (defence-in-depth) ─────────────────────
    let completed_cycles_at_join: u8;
    {
        let participant = &ctx.accounts.participant;
        // INV "Single claim per winner": belt + suspenders against
        // forged `MonthWinner` state from a malicious `select_winner`.
        require!(!participant.has_won, CoreError::AlreadyWon);
        require!(!participant.is_defaulted, CoreError::Defaulted);
        require!(!participant.is_suspended, CoreError::Suspended);
        completed_cycles_at_join = participant.completed_cycles_at_join;
    }

    // ───── 3. Manual-deserialize protocol_config + user_kyc (Q-15) ─────
    let cfg: ProtocolConfig = {
        let acct = &ctx.accounts.protocol_config;
        require_keys_eq!(*acct.owner, crate::ID, CoreError::Unauthorized);
        let mut data: &[u8] = &acct.try_borrow_data()?;
        ProtocolConfig::try_deserialize(&mut data)?
    };
    require!(!cfg.paused, CoreError::ProtocolPaused);
    require_keys_eq!(
        ctx.accounts.protocol_fee_vault.key(),
        cfg.protocol_fee_vault,
        CoreError::Unauthorized
    );

    // KYC: Full level, non-expired, sanctions clean. Re-checked here even
    // though `select_winner` already validated — the attestation may have
    // expired between selection and claim.
    {
        let acct = &ctx.accounts.user_kyc;
        require_keys_eq!(*acct.owner, crate::ID, CoreError::Unauthorized);
        let mut data: &[u8] = &acct.try_borrow_data()?;
        let kyc = KycAttestation::try_deserialize(&mut data)?;
        require_full_kyc(&kyc, &winner_key, now)?;
    }

    // ───── 3.5 Reserve isolation (INV-4) ───────────────────────────────
    let tier_seed = [pool_tier.as_u8()];
    let (expected_reserve_fund, _) = Pubkey::find_program_address(
        &[RESERVE_FUND_SEED, &tier_seed],
        &poolver_reserve::ID,
    );
    require_keys_eq!(
        ctx.accounts.reserve_fund.key(),
        expected_reserve_fund,
        CoreError::Unauthorized
    );
    let (expected_reserve_vault, _) = Pubkey::find_program_address(
        &[RESERVE_VAULT_SEED, &tier_seed],
        &poolver_reserve::ID,
    );
    require_keys_eq!(
        ctx.accounts.reserve_usdc_vault.key(),
        expected_reserve_vault,
        CoreError::Unauthorized
    );

    // ───── 4. Compute total collateral required (spec §4 + Q-7) ────────
    let total_months = Pool::TOTAL_MONTHS as u64;
    let win_month = current_month as u64;
    let months_remaining_at_win = total_months
        .checked_sub(win_month)
        .ok_or(CoreError::MathOverflow)?;

    let baseline = months_remaining_at_win
        .checked_mul(contribution_amount)
        .ok_or(CoreError::MathOverflow)?;
    let rep_bps = reputation_multiplier_bps(completed_cycles_at_join);
    let adjusted_baseline = baseline
        .checked_mul(rep_bps)
        .and_then(|v| v.checked_div(BPS_DENOMINATOR))
        .ok_or(CoreError::MathOverflow)?;
    let bid_premium = winning_bid
        .checked_mul(2)
        .ok_or(CoreError::MathOverflow)?;
    let total_collateral_required = adjusted_baseline
        .checked_add(bid_premium)
        .ok_or(CoreError::MathOverflow)?;

    // ───── 5. Compute payouts and bid distribution ─────────────────────
    // Defence-in-depth: recompute `net_payout` and assert it matches the
    // stored MonthWinner.net_payout. If select_winner had a bug or the
    // pool was tampered with, this catches it. We use the stored value
    // for the actual transfer (one source of truth).
    let recomputed_net_payout = stored_gross_payout
        .checked_sub(winning_bid)
        .ok_or(CoreError::MathOverflow)?;
    require!(
        recomputed_net_payout == stored_net_payout,
        CoreError::Unauthorized
    );
    let net_payout = stored_net_payout;

    // 75/20/5 split. Subtraction-based for the participant share so any
    // BPS rounding error stays solvent (rounding goes INTO the pool, not
    // OUT of it).
    let protocol_share = winning_bid
        .checked_mul(PROTOCOL_BID_SHARE_BPS)
        .and_then(|v| v.checked_div(BPS_DENOMINATOR))
        .ok_or(CoreError::MathOverflow)?;
    let reserve_share = winning_bid
        .checked_mul(RESERVE_BID_SHARE_BPS)
        .and_then(|v| v.checked_div(BPS_DENOMINATOR))
        .ok_or(CoreError::MathOverflow)?;
    let participant_share = winning_bid
        .checked_sub(protocol_share)
        .and_then(|v| v.checked_sub(reserve_share))
        .ok_or(CoreError::MathOverflow)?;

    // ───── 6. Verify winner's USDC balance covers collateral ───────────
    // SPL transfer would catch this at CPI time, but failing early gives
    // the client a clear `CollateralInsufficient` error code.
    {
        use anchor_spl::token::TokenAccount;
        let acct = &ctx.accounts.winner_usdc;
        let mut data: &[u8] = &acct.try_borrow_data()?;
        let ta = TokenAccount::try_deserialize(&mut data)?;
        require!(
            ta.amount >= total_collateral_required,
            CoreError::CollateralInsufficient
        );
    }

    let pool_usdc_vault_bump = ctx.bumps.pool_usdc_vault;
    let collateral_vault_bump = ctx.bumps.collateral_vault;
    let core_invoker_bump = ctx.bumps.core_invoker;

    // ───── 7. Token movements (INV-1 solvency proof) ───────────────────

    // (a) yield_vault::withdraw(gross_payout) → pool_usdc_vault.
    // Step 5's `contribute` deposits the net per-month contributions
    // into the yield adapter, so the funds physically live there until
    // claim time. We must withdraw the FULL `gross_payout` (= monthly_pot
    // = net_payout + winning_bid):
    //   - `net_payout` will go to the winner (step c)
    //   - `protocol_share` (5% of winning_bid) goes to the fee vault (d)
    //   - `reserve_share` (20% of winning_bid) goes to the reserve (e)
    //   - `participant_share` (75% of winning_bid) STAYS in pool_usdc_vault
    //     as the on-chain backing for `bid_credit_balance` — drawn down
    //     in subsequent `contribute` calls.
    if stored_gross_payout > 0 {
        // SPEC_QUESTION-36: tier dispatch — Tier 0 → poolver-yield-vault,
        // Tier 1 → poolver-yield-defi.
        crate::adapter_cpi::adapter::cpi_adapter_withdraw(
            pool_tier,
            ctx.accounts.yield_adapter_program.to_account_info(),
            ctx.accounts.core_invoker.to_account_info(),
            ctx.accounts.adapter_state.to_account_info(),
            ctx.accounts.adapter_usdc_vault.to_account_info(),
            ctx.accounts.pool_usdc_vault.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.remaining_accounts,
            core_invoker_bump,
            stored_gross_payout,
        )?;
    }

    // (b) winner → collateral_vault: total_collateral_required.
    if total_collateral_required > 0 {
        cpi_winner_to_collateral(
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.winner_usdc.to_account_info(),
            ctx.accounts.collateral_vault.to_account_info(),
            ctx.accounts.winner.to_account_info(),
            total_collateral_required,
        )?;
    }

    // (c) pool_usdc_vault → winner: net_payout.
    if net_payout > 0 {
        cpi_pool_to_winner(
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.pool_usdc_vault.to_account_info(),
            ctx.accounts.winner_usdc.to_account_info(),
            &pool_key,
            pool_usdc_vault_bump,
            net_payout,
        )?;
    }

    // (d) pool_usdc_vault → protocol_fee_vault: protocol_share.
    if protocol_share > 0 {
        cpi_pool_to_fee_vault(
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.pool_usdc_vault.to_account_info(),
            ctx.accounts.protocol_fee_vault.to_account_info(),
            &pool_key,
            pool_usdc_vault_bump,
            protocol_share,
        )?;
    }

    // (e) reserve::deposit(reserve_share) source = pool_usdc_vault.
    if reserve_share > 0 {
        cpi_reserve_deposit(
            ctx.accounts.reserve_program.to_account_info(),
            ctx.accounts.core_invoker.to_account_info(),
            ctx.accounts.reserve_fund.to_account_info(),
            ctx.accounts.reserve_usdc_vault.to_account_info(),
            ctx.accounts.pool_usdc_vault.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            &pool_key,
            core_invoker_bump,
            pool_usdc_vault_bump,
            reserve_share,
        )?;
    }

    // (f) participant_share — virtual credit. No on-chain token movement.
    // Tokens stay in `pool_usdc_vault` and get drawn down via the Q-1
    // pro-rata formula in subsequent `contribute` calls. We just bump
    // the ledger field below in step 9.

    // ───── 8. Cache release schedule (and handle month-12 edge case) ──
    let collateral_release_per_month: u64;
    let mut immediate_refund: u64 = 0;

    if months_remaining_at_win == 0 {
        // SPEC_QUESTION-34: month-12 winner edge case.
        // baseline = 0 → adjusted_baseline = 0 → total_collateral =
        // bid_premium. There are no future contributions to enforce
        // against, so we immediately refund the bid_premium back to the
        // winner and zero out the lock. This keeps `collateral_locked`'s
        // monotonic-decrease invariant intact (it goes 0 → bid_premium
        // → 0 within the same ix; net zero — but we don't bump
        // `collateral_initial` since the initial value IS the refund
        // amount, so the post-state is `locked = 0, initial = bid_premium`).
        immediate_refund = total_collateral_required;
        collateral_release_per_month = total_collateral_required;
    } else {
        // Normal case: release `collateral_initial / months_remaining_at_win`
        // per on-time payment (spec §4). Final-month true-up handled in
        // `contribute`.
        collateral_release_per_month = total_collateral_required
            .checked_div(months_remaining_at_win)
            .ok_or(CoreError::MathOverflow)?;
    }

    if immediate_refund > 0 {
        cpi_collateral_refund(
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.collateral_vault.to_account_info(),
            ctx.accounts.winner_usdc.to_account_info(),
            &pool_key,
            collateral_vault_bump,
            immediate_refund,
        )?;
    }

    // ───── 9. State updates (after token movements succeed) ────────────
    let post_claim_locked = if immediate_refund > 0 {
        0u64
    } else {
        total_collateral_required
    };

    {
        let participant = &mut ctx.accounts.participant;
        participant.has_won = true;
        participant.win_month = current_month;
        participant.bid_amount_when_won = winning_bid;
        participant.collateral_initial = total_collateral_required;
        participant.collateral_locked = post_claim_locked;
        participant.collateral_release_per_month = collateral_release_per_month;
    }

    {
        let pool = &mut ctx.accounts.pool;
        let m_idx = (current_month as usize) - 1;
        pool.winners[m_idx].claimed = true;
        pool.total_distributed = pool
            .total_distributed
            .checked_add(net_payout)
            .ok_or(CoreError::MathOverflow)?;
        pool.bid_credit_balance = pool
            .bid_credit_balance
            .checked_add(participant_share)
            .ok_or(CoreError::MathOverflow)?;
        pool.total_collateral_locked = pool
            .total_collateral_locked
            .checked_add(post_claim_locked)
            .ok_or(CoreError::MathOverflow)?;
    }

    {
        let rep = &mut ctx.accounts.user_reputation;
        rep.total_received_lifetime = rep
            .total_received_lifetime
            .checked_add(net_payout)
            .ok_or(CoreError::MathOverflow)?;
    }

    // ───── 10. Events ──────────────────────────────────────────────────
    emit!(WinningClaimed {
        pool: pool_key,
        month: current_month,
        winner: winner_key,
        winning_bid,
        net_payout,
        total_collateral_required,
        collateral_release_per_month,
        timestamp: now,
    });

    let bid_credit_balance_after = ctx.accounts.pool.bid_credit_balance;
    emit!(BidDistributed {
        pool: pool_key,
        month: current_month,
        total_bid: winning_bid,
        participant_share,
        reserve_share,
        protocol_share,
        bid_credit_balance_after,
        timestamp: now,
    });

    Ok(())
}
