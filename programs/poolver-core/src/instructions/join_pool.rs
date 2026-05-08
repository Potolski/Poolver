use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, Transfer};

use crate::constants::{
    BPS_DENOMINATOR, COLLATERAL_VAULT_SEED, CORE_INVOKER_SEED, KYC_SEED, PARTICIPANT_SEED,
    POOL_USDC_VAULT_SEED, PROTOCOL_CONFIG_SEED, PROTOCOL_FEE_VAULT_SEED, REPUTATION_SEED,
};
use crate::error::CoreError;
use crate::events::{ParticipantJoined, PoolStarted};
use crate::kyc::require_light_kyc;
use crate::state::{KycAttestation, Participant, Pool, ProtocolConfig, Tier, UserReputation};

/// `join_pool` — spec §5.1.
///
/// Flow (custody-then-route to keep token movements atomic and to
/// prove the source authority once for both downstream CPIs):
///
///   1. User → `pool_usdc_vault`: `contribution_amount`. The user is
///      the SPL signer here.
///   2. `pool_usdc_vault` → `protocol_fee_vault`: `protocol_fee`. The
///      pool USDC vault PDA signs.
///   3. CPI `poolver_reserve::deposit(reserve_fee)` with `source_usdc =
///      pool_usdc_vault` and `source_authority = pool_usdc_vault PDA`.
///      The reserve's `deposit` already accepts a generic
///      `source_authority: Signer` (see arch §5.1) so this composes.
///   4. CPI `poolver_yield_vault::deposit(net_to_pool)` with the same
///      `source_usdc` and `source_authority`. Adapter pulls the net.
///
/// The `core_invoker` PDA signs both CPI contexts to authenticate that
/// the call came from core (per arch §5.2). The `pool_usdc_vault` PDA
/// additionally signs as `source_authority` because it owns the source
/// token account.
///
/// SPEC_QUESTION-10 (Q-10): the reserve fee + protocol fee are
/// DEDUCTED from the gross contribution; the user transfers exactly
/// `contribution_amount`, and `net_to_pool = gross − protocol_fee −
/// reserve_fee`.
///
/// SPEC_QUESTION-7 (Q-7): `Participant.completed_cycles_at_join` is
/// snapshotted from `UserReputation.pools_completed` at this moment.
///
/// SPEC_QUESTION-15: the `Pool` account is wrapped in `Box`.
#[derive(Accounts)]
pub struct JoinPool<'info> {
    /// Joining user — pays for the Participant PDA rent and signs the
    /// initial USDC transfer into the pool vault.
    #[account(mut)]
    pub user: Signer<'info>,

    /// Protocol config. Manually deserialized inside the handler to
    /// keep `try_accounts`'s stack frame within the 4 KB BPF budget
    /// (SPEC_QUESTION-15). Anchor's `Account<'info, ProtocolConfig>`
    /// would normally enforce ownership + discriminator + bump in
    /// `try_accounts`; here we re-do those checks manually inside the
    /// handler (still in poolver-core's frame, so no security loss).
    /// CHECK: validated in `handle_join_pool` via PDA derivation +
    /// owner check + discriminator check.
    #[account(seeds = [PROTOCOL_CONFIG_SEED], bump)]
    pub protocol_config: UncheckedAccount<'info>,

    /// User's KYC attestation. // MOCK_KYC: V1 attestations come from
    /// `mock_issue_kyc`; production attestations come from
    /// `issue_kyc_attestation`. Verification is identical (handled by
    /// `require_light_kyc` after manual deserialization).
    /// CHECK: validated in `handle_join_pool`.
    #[account(seeds = [KYC_SEED, user.key().as_ref()], bump)]
    pub user_kyc: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [REPUTATION_SEED, user.key().as_ref()],
        bump = user_reputation.bump,
    )]
    pub user_reputation: Box<Account<'info, UserReputation>>,

    /// Pool. Box'd to keep stack pressure low (SPEC_QUESTION-15). The
    /// large `[Option<MonthWinner>; 12]` array (1200 bytes) lives on
    /// the heap; only the box pointer occupies the JoinPool stack
    /// frame.
    #[account(mut)]
    pub pool: Box<Account<'info, Pool>>,

    /// Per-(pool, user) participant record. Created here.
    #[account(
        init,
        payer = user,
        space = 8 + Participant::INIT_SPACE,
        seeds = [PARTICIPANT_SEED, pool.key().as_ref(), user.key().as_ref()],
        bump,
    )]
    pub participant: Box<Account<'info, Participant>>,

    /// User's source USDC account. Validated as `AccountInfo` to keep
    /// the JoinPool struct under the 4 KB BPF stack budget
    /// (SPEC_QUESTION-15) — TokenAccount deserialization happens inside
    /// the SPL transfer CPI, which is the canonical authority for token
    /// account semantics anyway.
    /// CHECK: SPL transfer enforces ownership and balance.
    #[account(mut)]
    pub user_usdc: UncheckedAccount<'info>,

    /// Pool USDC vault — owned by its own PDA. Bump comes from the
    /// `seeds` clause and is required to sign downstream PDA transfers.
    /// CHECK: PDA seeds + key equality with `pool.pool_usdc_vault`
    /// enforce identity.
    #[account(
        mut,
        seeds = [POOL_USDC_VAULT_SEED, pool.key().as_ref()],
        bump,
        constraint = pool_usdc_vault.key() == pool.pool_usdc_vault
            @ CoreError::Unauthorized,
    )]
    pub pool_usdc_vault: UncheckedAccount<'info>,

    /// Collateral vault — receives the join collateral (1× contribution
    /// per spec §4 demo extension). Mutable since join now transfers
    /// USDC into it.
    /// CHECK: PDA seeds + key equality with `pool.collateral_vault`
    /// enforce identity.
    #[account(
        mut,
        seeds = [COLLATERAL_VAULT_SEED, pool.key().as_ref()],
        bump,
        constraint = collateral_vault.key() == pool.collateral_vault
            @ CoreError::Unauthorized,
    )]
    pub collateral_vault: UncheckedAccount<'info>,

    /// Protocol fee SPL vault.
    /// CHECK: PDA seeds binding ensures it matches the canonical
    /// `protocol_fee_vault` PDA derived from `[PROTOCOL_FEE_VAULT_SEED]`.
    /// The handler also verifies `protocol_config.protocol_fee_vault`
    /// equality after manual deserialization.
    #[account(
        mut,
        seeds = [PROTOCOL_FEE_VAULT_SEED],
        bump,
    )]
    pub protocol_fee_vault: UncheckedAccount<'info>,

    /// `core_invoker` PDA, signs the reserve / adapter CPIs.
    /// CHECK: AccountInfo only; bump validated by Anchor seeds.
    #[account(seeds = [CORE_INVOKER_SEED], bump)]
    pub core_invoker: UncheckedAccount<'info>,

    // ───── Reserve CPI accounts ─────
    //
    // These six pass-through accounts are validated INSIDE the target
    // programs (reserve / yield-vault) via their own `seeds = [...]`
    // constraints. Re-validating them on core's side adds stack pressure
    // (Anchor's generated `try_accounts` re-derives the PDA per
    // constraint, blowing the 4 KB BPF frame). We rely on the receiving
    // programs' constraints — which is the canonical contract anyway:
    // INV-4 (reserve tier isolation) is structurally enforced by the
    // reserve's `seeds = [RESERVE_FUND_SEED, &[tier_byte]]` clause, not
    // by anything core says here. SPEC_QUESTION-15.

    /// CHECK: validated by `poolver_reserve::deposit`.
    #[account(mut)]
    pub reserve_fund: UncheckedAccount<'info>,

    /// CHECK: validated by `poolver_reserve::deposit`.
    #[account(mut)]
    pub reserve_usdc_vault: UncheckedAccount<'info>,

    /// CHECK: hardcoded program ID.
    #[account(address = poolver_reserve::ID)]
    pub reserve_program: UncheckedAccount<'info>,

    /// CHECK: validated by the chosen adapter's `deposit`.
    #[account(mut)]
    pub adapter_state: UncheckedAccount<'info>,

    /// CHECK: validated by the chosen adapter's `deposit`.
    #[account(mut)]
    pub adapter_usdc_vault: UncheckedAccount<'info>,

    /// CHECK: SPEC_QUESTION-36 — adapter program ID is validated against
    /// `pool.tier` in the handler via `cpi_adapter_deposit`. The
    /// hardcoded `address = poolver_yield_vault::ID` constraint that
    /// pre-step-13 builds carried was dropped so Tier 1 join calls can
    /// pass `poolver_yield_defi::ID`.
    pub yield_adapter_program: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

// Internal helper structs to spread CPIs across smaller stack frames.
// Anchor's BPF target has a 4 KB-per-frame stack budget; concentrating
// every CPI build + invocation into the handler frame blew it
// (SPEC_QUESTION-15). Each helper gets its own frame.

#[inline(never)]
fn cpi_user_to_pool_vault<'info>(
    token_program: AccountInfo<'info>,
    user_usdc: AccountInfo<'info>,
    pool_usdc_vault: AccountInfo<'info>,
    user: AccountInfo<'info>,
    amount: u64,
) -> Result<()> {
    let cpi_ctx = CpiContext::new(
        token_program.key(),
        Transfer {
            from: user_usdc,
            to: pool_usdc_vault,
            authority: user,
        },
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

// SPEC_QUESTION-36 (step 13): per-instruction `cpi_adapter_deposit`
// removed. Tier dispatch lives in `crate::adapter_cpi::adapter::
// cpi_adapter_deposit` and is invoked directly from the handler.

pub fn handle_join_pool<'info>(
    ctx: Context<'info, JoinPool<'info>>,
) -> Result<()> {
    // ───── 1. Pool state pre-checks ────────────────────────────────────
    {
        let pool = &ctx.accounts.pool;
        require!(!pool.is_complete, CoreError::PoolComplete);
        require!(pool.current_month == 0, CoreError::PoolAlreadyStarted);
        require!(
            pool.participant_filled() < Pool::POOL_SIZE,
            CoreError::PoolFull
        );
        require!(
            !pool.has_participant(&ctx.accounts.user.key()),
            CoreError::AlreadyParticipant
        );
    }

    // ───── 2. Manual deserialization of protocol_config + user_kyc ──
    // (We trade Anchor's automatic ownership/discriminator checks in
    //  `try_accounts` for manual ones here, in exchange for a smaller
    //  account-validation stack frame. SPEC_QUESTION-15.)
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

    let kyc: KycAttestation = {
        let acct = &ctx.accounts.user_kyc;
        require_keys_eq!(*acct.owner, crate::ID, CoreError::Unauthorized);
        let mut data: &[u8] = &acct.try_borrow_data()?;
        KycAttestation::try_deserialize(&mut data)?
    };

    // ───── 3. KYC verification (Light or better, unexpired, clean) ──
    let now = Clock::get()?.unix_timestamp;
    // MOCK_KYC: verification is mock-agnostic; the same call works for
    // both `mock_issue_kyc`-minted and `issue_kyc_attestation`-minted
    // attestations.
    require_light_kyc(&kyc, &ctx.accounts.user.key(), now)?;

    // SPEC_QUESTION-11: a user who has been liquidated in any prior pool
    // (`pools_defaulted > 0`) is blocked from joining new pools. Default
    // does NOT yank them from currently-active pools (no cascading
    // liquidation), but new joins are gated. Threshold is 0 — V1 has no
    // forgiveness; production may relax to e.g. `<= 1`.
    require!(
        ctx.accounts.user_reputation.pools_defaulted == 0,
        CoreError::ReputationDefaulted
    );

    // ───── 4. Fee math (SPEC_QUESTION-10: deducted from contribution) ─
    let contribution = ctx.accounts.pool.contribution_amount;
    let pool_tier = ctx.accounts.pool.tier;
    let reserve_fee_bps = match pool_tier {
        Tier::Vault => cfg.vault_reserve_fee_bps,
        Tier::DeFi => cfg.defi_reserve_fee_bps,
    } as u64;
    let protocol_fee_bps = cfg.protocol_fee_bps as u64;

    let protocol_fee = contribution
        .checked_mul(protocol_fee_bps)
        .and_then(|v| v.checked_div(BPS_DENOMINATOR))
        .ok_or(CoreError::MathOverflow)?;
    let reserve_fee = contribution
        .checked_mul(reserve_fee_bps)
        .and_then(|v| v.checked_div(BPS_DENOMINATOR))
        .ok_or(CoreError::MathOverflow)?;
    let net_to_pool = contribution
        .checked_sub(protocol_fee)
        .and_then(|v| v.checked_sub(reserve_fee))
        .ok_or(CoreError::MathOverflow)?;

    let pool_key = ctx.accounts.pool.key();
    let pool_usdc_vault_bump = ctx.bumps.pool_usdc_vault;
    let core_invoker_bump = ctx.bumps.core_invoker;

    // Join collateral — every participant escrows the FULL pool amount
    // (total_months × contribution) when they join. So a 12-month, $1k/mo
    // pool requires $12k collateral up-front, on top of the first
    // month's contribution. This is what the user reported they expect
    // ("the first user should have paid the whole amount in collateral")
    // and matches the trust model of traditional consórcios — collateral
    // covers the entire lifetime obligation, making post-win default
    // economically irrational.
    //
    // Held in collateral_vault for the pool duration; refunded via
    // `refund_collateral` once pool.is_complete && !is_defaulted.
    let join_collateral = (Pool::TOTAL_MONTHS as u64)
        .checked_mul(contribution)
        .ok_or(CoreError::MathOverflow)?;

    // ───── 4a. user → pool_usdc_vault (contribution) ──────────────────
    cpi_user_to_pool_vault(
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.user_usdc.to_account_info(),
        ctx.accounts.pool_usdc_vault.to_account_info(),
        ctx.accounts.user.to_account_info(),
        contribution,
    )?;

    // ───── 4b. user → collateral_vault (join collateral) ──────────────
    cpi_user_to_pool_vault(
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.user_usdc.to_account_info(),
        ctx.accounts.collateral_vault.to_account_info(),
        ctx.accounts.user.to_account_info(),
        join_collateral,
    )?;

    // ───── 5. pool_usdc_vault → protocol_fee_vault ────────────────────
    if protocol_fee > 0 {
        cpi_pool_to_fee_vault(
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.pool_usdc_vault.to_account_info(),
            ctx.accounts.protocol_fee_vault.to_account_info(),
            &pool_key,
            pool_usdc_vault_bump,
            protocol_fee,
        )?;
    }

    // ───── 6. CPI: reserve::deposit(reserve_fee) ──────────────────────
    if reserve_fee > 0 {
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
            reserve_fee,
        )?;
    }

    // ───── 7. CPI: adapter::deposit(net_to_pool) (SPEC_QUESTION-36) ────
    if net_to_pool > 0 {
        let combined_seeds: &[&[&[u8]]] = &[
            &[CORE_INVOKER_SEED, &[core_invoker_bump]],
            &[
                POOL_USDC_VAULT_SEED,
                pool_key.as_ref(),
                &[pool_usdc_vault_bump],
            ],
        ];
        crate::adapter_cpi::adapter::cpi_adapter_deposit(
            pool_tier,
            ctx.accounts.yield_adapter_program.to_account_info(),
            ctx.accounts.core_invoker.to_account_info(),
            ctx.accounts.adapter_state.to_account_info(),
            ctx.accounts.adapter_usdc_vault.to_account_info(),
            ctx.accounts.pool_usdc_vault.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.remaining_accounts,
            combined_seeds,
            net_to_pool,
        )?;
    }

    // ───── 8. Local state mutation (after all token movements succeed) ─
    let user_key = ctx.accounts.user.key();
    let snapshot_completed = ctx.accounts.user_reputation.pools_completed;
    let safe_snapshot: u8 = snapshot_completed.try_into().unwrap_or(u8::MAX);

    let participant = &mut ctx.accounts.participant;
    participant.pool = pool_key;
    participant.user = user_key;
    participant.joined_at = now;
    // bit 0 set: month 1 contribution paid via this join (spec §5.1).
    participant.paid_months = 0b1;
    participant.has_won = false;
    participant.win_month = 0;
    participant.bid_amount_when_won = 0;
    // Join collateral lives in `collateral_locked` until claim_winning
    // overwrites it (winner) or refund_collateral returns it (non-winner
    // at pool_complete) or liquidate_default slashes it.
    participant.collateral_locked = join_collateral;
    participant.collateral_initial = join_collateral;
    participant.is_defaulted = false;
    participant.is_suspended = false;
    participant.defaulted_at = 0;
    participant.late_penalty_accrued = 0;
    // Step 10: liquidation_amount replaces step-4's unused
    // `pending_credit` slot.
    participant.liquidation_amount = 0;
    participant.completed_cycles_at_join = safe_snapshot;
    participant.bump = ctx.bumps.participant;
    // SPEC_QUESTION-1: cached at win time (step 8). Always 0 at join.
    participant.collateral_release_per_month = 0;
    // Step 10 default-cascade fields — initialized clean.
    participant.is_late = false;
    participant.late_marked_at = 0;
    participant.suspended_at = 0;

    let pool = &mut ctx.accounts.pool;
    let slot_idx = pool
        .next_free_slot()
        .ok_or(CoreError::PoolFull)?;
    pool.participants[slot_idx] = Some(user_key);

    pool.total_contributed = pool
        .total_contributed
        .checked_add(net_to_pool)
        .ok_or(CoreError::MathOverflow)?;
    pool.total_collateral_locked = pool
        .total_collateral_locked
        .checked_add(join_collateral)
        .ok_or(CoreError::MathOverflow)?;

    let rep = &mut ctx.accounts.user_reputation;
    rep.pools_joined = rep
        .pools_joined
        .checked_add(1)
        .ok_or(CoreError::MathOverflow)?;
    rep.total_contributed_lifetime = rep
        .total_contributed_lifetime
        .checked_add(net_to_pool)
        .ok_or(CoreError::MathOverflow)?;
    if rep.kyc_status < kyc.level {
        rep.kyc_status = kyc.level;
        rep.kyc_attestation = ctx.accounts.user_kyc.key();
        rep.last_kyc_at = kyc.issued_at;
    }

    emit!(ParticipantJoined {
        pool: pool_key,
        user: user_key,
        slot_index: slot_idx as u8,
        gross_contribution: contribution,
        protocol_fee,
        reserve_fee,
        net_to_pool,
        completed_cycles_at_join: safe_snapshot,
        timestamp: now,
    });

    // ───── 9. Auto-start when 12 participants seated ───────────────────
    if pool.participant_filled() == Pool::POOL_SIZE {
        pool.current_month = 1;
        pool.start_timestamp = now;
        pool.current_month_started_at = now;
        pool.bid_window_ends_at = now
            .checked_add(pool.bid_window_seconds)
            .ok_or(CoreError::MathOverflow)?;
        // Same scaling as advance_month: reveal = max(60s, bid/2).
        // For demo pools with short months this keeps the auction inside
        // the month; for production-default 48h bid this gives 24h reveal.
        let reveal_secs = (pool.bid_window_seconds / 2).max(60);
        pool.reveal_window_ends_at = pool
            .bid_window_ends_at
            .checked_add(reveal_secs)
            .ok_or(CoreError::MathOverflow)?;

        emit!(PoolStarted {
            pool: pool_key,
            start_timestamp: now,
        });
    }

    Ok(())
}
