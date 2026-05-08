//! `select_winner` — spec §5.1, step 7.
//!
//! ## Architectural shape (arch §9 + Q-21)
//!
//! Architecture §9 mandates a **hybrid pattern**: the bid-path resolves
//! synchronously inside one transaction; the lottery-path requests
//! Switchboard On-Demand VRF and completes asynchronously via a
//! `consume_vrf_winner` callback. **For V1 (this step), we collapse both
//! branches into a single sync instruction** because Q-21 specifies a
//! mocked VRF: the entropy source is `sha256(pool || month || slot)`,
//! not a real oracle round, so there is no async round-trip to wait on.
//!
//! Where the **production** Switchboard integration plugs in is marked
//! with `// SPEC_QUESTION-21:` throughout this file. The `WinnerSelected`
//! event shape is identical between V1 mock and production VRF — the
//! indexer sees no schema change when the oracle path swaps in.
//!
//! ## Account-flow layout
//!
//! Step-7 introduces a new `remaining_accounts` convention because
//! `select_winner` ranges over up to 12 candidates per call. The
//! convention is *self-describing*: the handler distinguishes a
//! `(bid, participant, kyc)` triple from a `(participant, kyc)` pair by
//! Anchor account discriminators (Bid vs Participant). Caller layout:
//!
//! ```text
//!   remaining_accounts = (
//!       [bid, participant, kyc] × N_committed_bids   (any order),
//!       [participant, kyc]      × M_non_bidder_candidates,
//!   )
//! ```
//!
//! - `N_committed_bids`: every `Bid` PDA whose `month == pool.current_month`
//!   should be passed. Both revealed AND committed-but-unrevealed bids:
//!   the latter need to be in the iteration so their stake is forfeit
//!   to the tier reserve (Q-3).
//! - `M_non_bidder_candidates`: only used by the lottery branch. May be
//!   empty if the caller is sure at least one revealed bid is eligible.
//!
//! Anchor's PDA seed table guarantees a Bid PDA's owner is
//! `poolver_core::ID` and its discriminator differs from Participant's,
//! so the handler can iterate by inspecting the first 8 bytes (Anchor
//! discriminator) of each account. We don't trust `key()` alone for the
//! discrimination — we deserialize and assert.
//!
//! ## `has_won = true` deferred to `claim_winning`
//!
//! `select_winner` writes the `MonthWinner` slot but **does NOT** flip
//! the winner's `Participant.has_won` flag. The flip happens in step 8
//! (`claim_winning`) so that ix can:
//!   - re-validate the winner against `pool.winners[month-1].winner`
//!   - require Full KYC at claim time (defence-in-depth — KYC may have
//!     expired between selection and claim)
//!   - bookkeep `win_month`, `bid_amount_when_won`, collateral state
//!
//! Step 8's authorization check therefore compares the claimer's pubkey
//! to the on-chain `MonthWinner.winner` value. A second call to
//! `select_winner` for the same month is structurally rejected via the
//! `MonthWinner.month != 0` sentinel, so the deferred flip is safe.
//!
//! ## Tie-break (Q-2)
//!
//! Ties on `revealed_amount` are broken by the lexicographically smallest
//! `sha256(pool || current_month_le || bidder_pubkey)`. Probability of a
//! u64-microUSDC tie inside a 12-bidder pool with 20% bid cap is < 1
//! per 2^60; the deterministic hash avoids forcing every contested month
//! into the (currently mocked) VRF flow.
//!
//! ## Pool ‹state› vs ‹token› flow
//!
//! State writes:
//!   - `pool.winners[current_month - 1] = MonthWinner { ... }`
//!   - one `bid.is_winner = true` (the bid path's winner)
//!   - on every committed-but-unrevealed bid that we drain for stake
//!     forfeit: `bid.stake_refunded = true`
//!
//! Token movement (only on stake forfeit):
//!   - `bid_stake_vault → reserve_usdc_vault` via CPI to
//!     `poolver_reserve::deposit(stake_amount)` per unrevealed bid.
//!     `core_invoker` + `bid_stake_vault` PDAs co-sign.
//!
//! No yield/payout movement happens here. Net payout is wired up to the
//! winner in step 8 (`claim_winning`).
//!
//! ## Errors
//!
//! - `BidWindowOpen` — reveal window still open
//! - `WinnerAlreadySelected` — `pool.winners[m-1].month != 0`
//! - `NoEligibleParticipants` — lottery branch with empty candidate set
//! - `SelectWinnerAccountsMalformed` — remaining_accounts shape invalid
//! - `ProtocolPaused`, `PoolComplete`, `PoolNotStarted` — standard gates

use anchor_lang::prelude::*;
use anchor_lang::Discriminator;
use anchor_spl::token::Token;
use solana_sha256_hasher::hashv;

use crate::constants::{
    BID_SEED, BID_STAKE_VAULT_SEED, BPS_DENOMINATOR, CORE_INVOKER_SEED, KYC_SEED,
    PARTICIPANT_SEED, POOL_SIZE, PROTOCOL_CONFIG_SEED, RESERVE_FUND_SEED, RESERVE_VAULT_SEED,
};
use crate::error::CoreError;
use crate::events::{BidStakeForfeited, WinnerSelected};
use crate::kyc::require_full_kyc;
use crate::state::{
    Bid, KycAttestation, MonthWinner, Participant, Pool, ProtocolConfig, SelectionMethod,
    Tier,
};

/// Anyone can call. Payer/keeper is the only signer; all candidate state
/// flows in via `remaining_accounts`.
///
/// SPEC_QUESTION-15: `Pool` is `Box`'d. `protocol_config` and the various
/// reserve/stake-vault adjacent accounts are `UncheckedAccount` so the
/// `try_accounts`-time stack frame stays under the 4 KB BPF budget. Same
/// trade-off as `commit_bid` / `reveal_bid` / `contribute`.
#[derive(Accounts)]
pub struct SelectWinner<'info> {
    /// Permissionless caller. Pays the tx fee. Not validated against any
    /// on-chain state — anyone is allowed to advance pool state by spec.
    #[account(mut)]
    pub caller: Signer<'info>,

    /// Protocol config. Manually deserialized in the handler
    /// (SPEC_QUESTION-15). CHECK: PDA seed binding here, owner +
    /// discriminator validated inside the handler.
    #[account(seeds = [PROTOCOL_CONFIG_SEED], bump)]
    pub protocol_config: UncheckedAccount<'info>,

    /// The pool. Mut because we write the winner slot.
    #[account(mut)]
    pub pool: Box<Account<'info, Pool>>,

    // ───── Stake-forfeit accounts (used only when an unrevealed bid is in
    //       remaining_accounts; otherwise pure no-op pass-through). We
    //       always require them so the account schema is fixed and
    //       indexers / ALTs can plan a single layout per pool.
    /// Per-pool bid-stake vault. PDA-owned token account; signs the
    /// transfer to reserve. CHECK: PDA seed binding ensures identity.
    #[account(
        mut,
        seeds = [BID_STAKE_VAULT_SEED, pool.key().as_ref()],
        bump,
    )]
    pub bid_stake_vault: UncheckedAccount<'info>,

    /// `core_invoker` PDA — co-signs the reserve CPI per arch §5.2.
    /// CHECK: AccountInfo only; bump validated by Anchor seeds.
    #[account(seeds = [CORE_INVOKER_SEED], bump)]
    pub core_invoker: UncheckedAccount<'info>,

    /// CHECK: validated by `poolver_reserve::deposit` (tier-encoded seed).
    /// Mut because deposit increments balance + inflows.
    #[account(mut)]
    pub reserve_fund: UncheckedAccount<'info>,

    /// CHECK: validated by `poolver_reserve::deposit`. Mut because
    /// deposit transfers tokens in.
    #[account(mut)]
    pub reserve_usdc_vault: UncheckedAccount<'info>,

    /// CHECK: hardcoded program ID.
    #[account(address = poolver_reserve::ID)]
    pub reserve_program: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

// ─── Forfeit CPI helper (split out per SPEC_QUESTION-15 stack budget) ───

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn cpi_forfeit_to_reserve<'info>(
    reserve_program: AccountInfo<'info>,
    core_invoker: AccountInfo<'info>,
    reserve_fund: AccountInfo<'info>,
    reserve_usdc_vault: AccountInfo<'info>,
    bid_stake_vault: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    pool_key: &Pubkey,
    core_invoker_bump: u8,
    bid_stake_vault_bump: u8,
    amount: u64,
) -> Result<()> {
    let cpi_accounts = poolver_reserve::cpi::accounts::ReserveDepositCtx {
        core_invoker,
        reserve_fund,
        reserve_usdc_vault,
        source_usdc: bid_stake_vault.clone(),
        source_authority: bid_stake_vault,
        token_program,
    };
    let combined_seeds: &[&[&[u8]]] = &[
        &[CORE_INVOKER_SEED, &[core_invoker_bump]],
        &[
            BID_STAKE_VAULT_SEED,
            pool_key.as_ref(),
            &[bid_stake_vault_bump],
        ],
    ];
    let cpi_ctx = CpiContext::new_with_signer(
        reserve_program.key(),
        cpi_accounts,
        combined_seeds,
    );
    poolver_reserve::cpi::deposit(cpi_ctx, amount)
}

// ─── Eligibility helpers ────────────────────────────────────────────────

/// True iff `(participant, kyc)` is currently eligible to win:
/// has not won any past month, not defaulted, not suspended, Full-KYC,
/// unexpired, sanctions-clean.
///
/// Past-win check looks at `pool.winners` directly — `participant.has_won`
/// only flips at claim time, so an unclaimed past winner would otherwise
/// pass this filter and be eligible for re-selection.
fn is_eligible(
    pool: &Pool,
    participant: &Participant,
    kyc: &KycAttestation,
    user: &Pubkey,
    now: i64,
) -> bool {
    if participant.has_won || participant.is_defaulted || participant.is_suspended {
        return false;
    }
    if pool.has_won_any_month(user) {
        return false;
    }
    require_full_kyc(kyc, user, now).is_ok()
}

/// Q-2 tiebreak: lexicographically smallest sha256(pool || month_le ||
/// bidder.to_bytes()) wins. Documented choice: smallest. Probability of
/// even a 2-way tie at u64 microUSDC granularity in a 12-bidder pool is
/// < 2^-60, so this branch is effectively never executed in practice —
/// but it MUST be deterministic for the keeper-vs-keeper race condition.
fn tiebreak_hash(pool: &Pubkey, month: u8, bidder: &Pubkey) -> [u8; 32] {
    let pool_bytes = pool.to_bytes();
    let month_le = [month];
    let bidder_bytes = bidder.to_bytes();
    hashv(&[&pool_bytes, &month_le, &bidder_bytes]).to_bytes()
}

/// Mock-VRF entropy source — V1 ONLY (Q-21).
///
/// SPEC_QUESTION-21: production swaps this for a Switchboard On-Demand
/// VRF callback. The seed input shape is intentionally orthogonal to any
/// participant pubkey so a malicious caller cannot grind candidates by
/// re-trying with different participant orders inside `remaining_accounts`
/// — the seed depends only on `pool`, `month`, and `slot`.
fn mock_vrf_seed(pool: &Pubkey, month: u8, slot: u64) -> [u8; 32] {
    let pool_bytes = pool.to_bytes();
    let month_le = [month];
    let slot_le = slot.to_le_bytes();
    hashv(&[&pool_bytes, &month_le, &slot_le]).to_bytes()
}

// ─── Internal candidate carriers (no heap; size known at compile time) ──
//
// 12 max candidates per call. Each entry stays small — pubkeys + a
// u64 + a single `usize` index back into `remaining_accounts` so we can
// flip `bid.is_winner` after the search.

#[derive(Clone, Copy)]
struct BidCandidate {
    user: Pubkey,
    revealed_amount: u64,
    /// Index into `remaining_accounts` of the underlying Bid PDA.
    bid_idx: usize,
}

#[derive(Clone, Copy)]
struct LotteryCandidate {
    user: Pubkey,
}

pub fn handle_select_winner(ctx: Context<SelectWinner>) -> Result<()> {
    // ───── 1. Pause + pool gates (INV-25, INV-31) ──────────────────────
    {
        let acct = &ctx.accounts.protocol_config;
        require_keys_eq!(*acct.owner, crate::ID, CoreError::Unauthorized);
        let mut data: &[u8] = &acct.try_borrow_data()?;
        let cfg = ProtocolConfig::try_deserialize(&mut data)?;
        require!(!cfg.paused, CoreError::ProtocolPaused);
    }

    let now = Clock::get()?.unix_timestamp;
    let slot = Clock::get()?.slot;

    let pool_key = ctx.accounts.pool.key();
    let current_month: u8;
    let pool_tier: Tier;
    let contribution_amount: u64;
    let pool_protocol_fee_bps: u64;
    let pool_reserve_fee_bps: u64;
    {
        let pool = &ctx.accounts.pool;
        require!(!pool.is_complete, CoreError::PoolComplete);
        require!(
            pool.current_month >= 1 && pool.current_month <= Pool::TOTAL_MONTHS,
            CoreError::PoolNotStarted
        );

        // Reveal window must have closed — same gate `reveal_bid` uses
        // for its UPPER bound, applied here as a LOWER bound. Distinct
        // error from `BidWindowClosed` so the client UX can render a
        // helpful message.
        require!(
            now >= pool.reveal_window_ends_at,
            CoreError::BidWindowOpen
        );

        // INV "Single winner per month": MonthWinner default is
        // `month: 0`. Anything non-zero means already filled.
        let m_idx = (pool.current_month as usize) - 1;
        require!(
            pool.winners[m_idx].month == 0,
            CoreError::WinnerAlreadySelected
        );

        current_month = pool.current_month;
        pool_tier = pool.tier;
        contribution_amount = pool.contribution_amount;

        // Re-read fee bps from protocol_config — same trick as reveal_bid.
        let acct = &ctx.accounts.protocol_config;
        let mut data: &[u8] = &acct.try_borrow_data()?;
        let cfg = ProtocolConfig::try_deserialize(&mut data)?;
        pool_protocol_fee_bps = cfg.protocol_fee_bps as u64;
        pool_reserve_fee_bps = match pool_tier {
            Tier::Vault => cfg.vault_reserve_fee_bps,
            Tier::DeFi => cfg.defi_reserve_fee_bps,
        } as u64;
    }

    // ───── 1.5 Reserve isolation (INV-4 / arch §11) — same shape as
    //          `contribute`. Even though we may not actually CPI to
    //          reserve in every call (only when stake forfeit is needed),
    //          we validate up-front so the account schema is enforced.
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

    // ───── 2. Monthly-pot math (Q-10), same formula as reveal_bid ──────
    let protocol_fee = contribution_amount
        .checked_mul(pool_protocol_fee_bps)
        .and_then(|v| v.checked_div(BPS_DENOMINATOR))
        .ok_or(CoreError::MathOverflow)?;
    let reserve_fee = contribution_amount
        .checked_mul(pool_reserve_fee_bps)
        .and_then(|v| v.checked_div(BPS_DENOMINATOR))
        .ok_or(CoreError::MathOverflow)?;
    let net_contribution = contribution_amount
        .checked_sub(protocol_fee)
        .and_then(|v| v.checked_sub(reserve_fee))
        .ok_or(CoreError::MathOverflow)?;
    let monthly_pot = (POOL_SIZE as u64)
        .checked_mul(net_contribution)
        .ok_or(CoreError::MathOverflow)?;

    // ───── 3. Walk remaining_accounts ──────────────────────────────────
    //
    // First-account discriminator picks the chunk shape:
    //   - Bid::DISCRIMINATOR  → triple (bid, participant, kyc)
    //   - Participant::DISCRIMINATOR → pair (participant, kyc)
    //
    // Everything not matching either shape is `SelectWinnerAccountsMalformed`.
    let bid_stake_vault_bump = ctx.bumps.bid_stake_vault;
    let core_invoker_bump = ctx.bumps.core_invoker;

    // Owned vectors of candidates (max 12 each).
    let mut bid_candidates: Vec<BidCandidate> = Vec::with_capacity(POOL_SIZE as usize);
    let mut lottery_candidates: Vec<LotteryCandidate> =
        Vec::with_capacity(POOL_SIZE as usize);

    // We collect forfeit work as (bid_idx, amount) tuples. We perform
    // the actual CPI + flag flip AFTER the eligibility scan to keep the
    // borrow checker happy (the CPI mutably touches `bid_stake_vault`
    // while the scan loop still borrows `remaining_accounts` read-only).
    let mut forfeit_work: Vec<(usize, u64, Pubkey)> = Vec::with_capacity(POOL_SIZE as usize);

    let remaining = ctx.remaining_accounts;
    let mut i = 0usize;
    while i < remaining.len() {
        let head = &remaining[i];

        // Inspect first 8 bytes (Anchor discriminator). Fall through to
        // the malformed error if the account isn't core-owned or has too
        // little data.
        let head_data = head.try_borrow_data()?;
        if head_data.len() < 8 {
            return err!(CoreError::SelectWinnerAccountsMalformed);
        }
        let disc = &head_data[..8];

        if disc == Bid::DISCRIMINATOR {
            // ─── Triple: (bid, participant, kyc) ──────────────────────
            require_keys_eq!(*head.owner, crate::ID, CoreError::Unauthorized);
            require!(
                i + 2 < remaining.len(),
                CoreError::SelectWinnerAccountsMalformed
            );

            let mut bid_data: &[u8] = &head_data;
            let bid: Bid = Bid::try_deserialize(&mut bid_data)?;
            drop(head_data);

            // Validate Bid PDA: must be for THIS pool + THIS month + bid.user.
            let month_seed = [bid.month];
            let (expected_bid_pda, _) = Pubkey::find_program_address(
                &[BID_SEED, pool_key.as_ref(), &month_seed, bid.user.as_ref()],
                &crate::ID,
            );
            require_keys_eq!(
                head.key(),
                expected_bid_pda,
                CoreError::SelectWinnerAccountsMalformed
            );
            require!(
                bid.month == current_month,
                CoreError::SelectWinnerAccountsMalformed
            );
            require_keys_eq!(bid.pool, pool_key, CoreError::SelectWinnerAccountsMalformed);

            // Participant + KYC follow.
            let part_acct = &remaining[i + 1];
            let kyc_acct = &remaining[i + 2];
            require_keys_eq!(*part_acct.owner, crate::ID, CoreError::Unauthorized);
            require_keys_eq!(*kyc_acct.owner, crate::ID, CoreError::Unauthorized);

            let part: Participant = {
                let mut d: &[u8] = &part_acct.try_borrow_data()?;
                Participant::try_deserialize(&mut d)?
            };
            let kyc: KycAttestation = {
                let mut d: &[u8] = &kyc_acct.try_borrow_data()?;
                KycAttestation::try_deserialize(&mut d)?
            };

            // Ensure the participant/kyc accounts match `bid.user`.
            let (expected_part, _) = Pubkey::find_program_address(
                &[PARTICIPANT_SEED, pool_key.as_ref(), bid.user.as_ref()],
                &crate::ID,
            );
            require_keys_eq!(
                part_acct.key(),
                expected_part,
                CoreError::SelectWinnerAccountsMalformed
            );
            require_keys_eq!(part.pool, pool_key, CoreError::NotAParticipant);
            require_keys_eq!(part.user, bid.user, CoreError::NotAParticipant);

            let (expected_kyc, _) =
                Pubkey::find_program_address(&[KYC_SEED, bid.user.as_ref()], &crate::ID);
            require_keys_eq!(
                kyc_acct.key(),
                expected_kyc,
                CoreError::SelectWinnerAccountsMalformed
            );

            // ───────── Eligibility filter ─────────
            //
            // Three-way outcome:
            //   (a) bid.revealed && eligible(part, kyc)
            //         → BidCandidate (will compete on revealed_amount)
            //   (b) !bid.revealed && !bid.stake_refunded
            //         → forfeit stake (Q-3) — schedule CPI later
            //   (c) anything else (unrevealed but already refunded, or
            //       revealed but ineligible) → silently skip
            if bid.revealed && is_eligible(&ctx.accounts.pool, &part, &kyc, &bid.user, now) {
                bid_candidates.push(BidCandidate {
                    user: bid.user,
                    revealed_amount: bid.revealed_amount,
                    bid_idx: i,
                });
            } else if !bid.revealed && !bid.stake_refunded && bid.stake_amount > 0 {
                // Stake forfeit (Q-3). Schedule the CPI; we'll execute it
                // after the iteration finishes. Per-bid event is emitted
                // at execute time.
                forfeit_work.push((i, bid.stake_amount, bid.user));
            }

            i += 3;
        } else if disc == Participant::DISCRIMINATOR {
            // ─── Pair: (participant, kyc) — non-bidder lottery candidate ─
            require_keys_eq!(*head.owner, crate::ID, CoreError::Unauthorized);
            require!(
                i + 1 < remaining.len(),
                CoreError::SelectWinnerAccountsMalformed
            );

            let mut part_data: &[u8] = &head_data;
            let part: Participant = Participant::try_deserialize(&mut part_data)?;
            drop(head_data);

            let (expected_part, _) = Pubkey::find_program_address(
                &[PARTICIPANT_SEED, pool_key.as_ref(), part.user.as_ref()],
                &crate::ID,
            );
            require_keys_eq!(
                head.key(),
                expected_part,
                CoreError::SelectWinnerAccountsMalformed
            );
            require_keys_eq!(part.pool, pool_key, CoreError::NotAParticipant);

            let kyc_acct = &remaining[i + 1];
            require_keys_eq!(*kyc_acct.owner, crate::ID, CoreError::Unauthorized);
            let kyc: KycAttestation = {
                let mut d: &[u8] = &kyc_acct.try_borrow_data()?;
                KycAttestation::try_deserialize(&mut d)?
            };
            let (expected_kyc, _) =
                Pubkey::find_program_address(&[KYC_SEED, part.user.as_ref()], &crate::ID);
            require_keys_eq!(
                kyc_acct.key(),
                expected_kyc,
                CoreError::SelectWinnerAccountsMalformed
            );

            if is_eligible(&ctx.accounts.pool, &part, &kyc, &part.user, now) {
                lottery_candidates.push(LotteryCandidate { user: part.user });
            }

            i += 2;
        } else {
            // Unknown discriminator. Likely caller mismatch.
            return err!(CoreError::SelectWinnerAccountsMalformed);
        }
    }

    // ───── 4. Stake-forfeit CPIs (idempotent on `bid.stake_refunded`) ──
    //
    // Done before winner selection so a partial-failure here aborts the
    // whole tx — we never end up with a `MonthWinner` written but the
    // stake forfeit half-applied. Both writes are fully atomic.
    for (bid_idx, stake_amount, user) in forfeit_work.iter().copied() {
        cpi_forfeit_to_reserve(
            ctx.accounts.reserve_program.to_account_info(),
            ctx.accounts.core_invoker.to_account_info(),
            ctx.accounts.reserve_fund.to_account_info(),
            ctx.accounts.reserve_usdc_vault.to_account_info(),
            ctx.accounts.bid_stake_vault.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            &pool_key,
            core_invoker_bump,
            bid_stake_vault_bump,
            stake_amount,
        )?;

        // Flip `bid.stake_refunded = true` on the underlying account.
        // Re-deserialize → mutate → serialize back. We can't hold a
        // mutable Anchor Account here because the account list is
        // dynamic; manual round-trip is fine.
        let bid_acct = &ctx.remaining_accounts[bid_idx];
        let mut data = bid_acct.try_borrow_mut_data()?;
        let mut bid: Bid = {
            let mut d: &[u8] = &data;
            Bid::try_deserialize(&mut d)?
        };
        bid.stake_refunded = true;
        let mut buf: Vec<u8> = Vec::with_capacity(8 + Bid::INIT_SPACE);
        bid.try_serialize(&mut buf)?;
        data[..buf.len()].copy_from_slice(&buf);
        drop(data);

        emit!(BidStakeForfeited {
            pool: pool_key,
            user,
            month: current_month,
            stake_amount,
            timestamp: now,
        });
    }

    // ───── 5. Winner selection ─────────────────────────────────────────
    let (winner_pub, winning_amount, method, winner_bid_idx): (
        Pubkey,
        u64,
        SelectionMethod,
        Option<usize>,
    ) = if !bid_candidates.is_empty() {
        // ─── Bid path ──────────────────────────────────────────────────
        // Find max revealed_amount; resolve any tie via Q-2 hash.
        let mut best_idx: usize = 0;
        for (idx, cand) in bid_candidates.iter().enumerate().skip(1) {
            let best = &bid_candidates[best_idx];
            if cand.revealed_amount > best.revealed_amount {
                best_idx = idx;
            } else if cand.revealed_amount == best.revealed_amount {
                // Tie — Q-2: lexicographically smallest hash wins.
                let h_best = tiebreak_hash(&pool_key, current_month, &best.user);
                let h_cand = tiebreak_hash(&pool_key, current_month, &cand.user);
                if h_cand < h_best {
                    best_idx = idx;
                }
            }
        }
        let w = bid_candidates[best_idx];
        (w.user, w.revealed_amount, SelectionMethod::Bid, Some(w.bid_idx))
    } else {
        // ─── Lottery path ──────────────────────────────────────────────
        //
        // SPEC_QUESTION-21: V1 mock VRF.
        //
        // Production swap-in plan (no schema change):
        //   1. Replace this whole branch's seed derivation with a CPI to
        //      Switchboard On-Demand: `swb::request_randomness(...)`,
        //      passing a callback that lands in a sibling instruction
        //      `consume_vrf_winner(pool, vrf_account)`.
        //   2. Set `pool.vrf_in_flight = true` + `pool.vrf_account = ...`,
        //      and return `Ok(())` — `MonthWinner` is written by the
        //      callback ix, not here.
        //   3. The lottery candidate set MUST persist across the async
        //      gap. Easiest path: write the candidate list into a
        //      transient PDA `[POOL_SEED, pool_key, b"vrf_request",
        //      month_le]`, drop it on the callback. Arch §9.
        //
        // For V1 we collapse all that into a single tx; the entropy
        // source is `sha256(pool || month || slot)` which is deterministic
        // but unpredictable to anyone trying to grind candidates. The
        // candidate ORDER is irrelevant because the seed doesn't depend
        // on it.
        require!(
            !lottery_candidates.is_empty(),
            CoreError::NoEligibleParticipants
        );

        let seed = mock_vrf_seed(&pool_key, current_month, slot);
        // First 8 bytes → u64 → mod candidate_count.
        let mut idx_bytes = [0u8; 8];
        idx_bytes.copy_from_slice(&seed[..8]);
        let raw = u64::from_le_bytes(idx_bytes);
        // candidate_count <= POOL_SIZE = 12; safe `as usize`.
        let pick = (raw % (lottery_candidates.len() as u64)) as usize;
        let chosen = lottery_candidates[pick];
        (chosen.user, 0u64, SelectionMethod::Lottery, None)
    };

    // ───── 6. Compute payouts and write MonthWinner ────────────────────
    let net_payout = monthly_pot
        .checked_sub(winning_amount)
        .ok_or(CoreError::MathOverflow)?;

    // ───── 7. Mutate Pool + winning Bid (if any) ───────────────────────
    {
        let pool = &mut ctx.accounts.pool;
        let m_idx = (current_month as usize) - 1;
        // INV "MonthWinner persistent": once written, we never rewrite.
        // The pre-check on `month == 0` plus the single-tx-per-month
        // guard is the structural enforcement here.
        pool.winners[m_idx] = MonthWinner {
            month: current_month,
            winner: winner_pub,
            winning_bid: winning_amount,
            gross_payout: monthly_pot,
            net_payout,
            selected_at: now,
            selection_method: method,
            claimed: false,
            _reserved: [0u8; 8],
        };
    }

    if let Some(bid_idx) = winner_bid_idx {
        let bid_acct = &ctx.remaining_accounts[bid_idx];
        let mut data = bid_acct.try_borrow_mut_data()?;
        let mut bid: Bid = {
            let mut d: &[u8] = &data;
            Bid::try_deserialize(&mut d)?
        };
        bid.is_winner = true;
        let mut buf: Vec<u8> = Vec::with_capacity(8 + Bid::INIT_SPACE);
        bid.try_serialize(&mut buf)?;
        data[..buf.len()].copy_from_slice(&buf);
    }

    emit!(WinnerSelected {
        pool: pool_key,
        month: current_month,
        winner: winner_pub,
        winning_bid: winning_amount,
        gross_payout: monthly_pot,
        net_payout,
        method,
        timestamp: now,
    });

    Ok(())
}
