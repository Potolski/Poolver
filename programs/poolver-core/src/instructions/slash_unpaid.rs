//! `slash_unpaid` — post-month enforcement (V1 simplification of the
//! step-10 default cascade).
//!
//! ## Why this exists
//!
//! Each joiner posts 12 × `contribution_amount` collateral up front. The
//! *intent* of that collateral is to back the joiner's monthly obligations
//! — if a participant misses month X, the protocol slashes one month's
//! contribution from their collateral and forwards it to the yield
//! adapter so the pot for month X's winner stays whole.
//!
//! V1 collapses the spec's day-1..5 / day-6..29 / day-30+ cascade into a
//! single permissionless instruction that becomes callable as soon as
//! `now >= current_month_started_at + month_duration_seconds`. Anyone
//! (keeper bot, dapp UI, the next month's eventual winner) can call it.
//!
//! ## Token movement
//!
//! Two SPL transfers, both PDA-signed, no fees taken:
//!
//!   1. `collateral_vault → pool_usdc_vault`: `slash_amount` USDC.
//!      Authority is the `collateral_vault` PDA (seeds
//!      `[COLLATERAL_VAULT_SEED, pool]`).
//!   2. CPI `yield_adapter::deposit(slash_amount)` with
//!      `source_usdc = pool_usdc_vault`, signed by both the
//!      `pool_usdc_vault` PDA and the `core_invoker` PDA.
//!
//! Skipping protocol/reserve fees on the slashed amount is a deliberate
//! V1 simplification — the pot's gross_payout invariant holds because we
//! deposit the *full* `contribution_amount` (≥ `net_contribution`) into
//! the adapter; the surplus accrues as if it were yield. Production
//! versions can split fees the same way `contribute` does.
//!
//! ## State changes
//!
//! - `participant.paid_months[month] = 1` — closes the door for a normal
//!   `contribute` call this month, and increments the pool's
//!   `paid_count_for_current_month` so the "all 12 satisfied" gate works
//!   uniformly across paid/slashed.
//! - `participant.collateral_locked -= slash_amount`. If the slash
//!   exhausts collateral, `participant.is_defaulted = true` is set and
//!   no further slashes are possible — the participant is removed from
//!   bid/lottery candidacy via the existing eligibility filter.
//! - `pool.total_collateral_locked -= slash_amount`,
//!   `pool.paid_count_for_current_month += 1`,
//!   `pool.total_contributed += slash_amount`.
//! - `user_reputation.months_missed_lifetime += 1` — soft signal for the
//!   tier UI ("yellow" tier).
//!
//! ## Idempotency
//!
//! Single slash per `(participant, month)`: the `paid_months` bit gate
//! enforces this. A second call reverts with `NotLate`.

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, Transfer};

use crate::adapter_cpi::adapter::cpi_adapter_deposit;
use crate::constants::{
    COLLATERAL_VAULT_SEED, CORE_INVOKER_SEED, PARTICIPANT_SEED, POOL_USDC_VAULT_SEED,
    PROTOCOL_CONFIG_SEED, REPUTATION_SEED,
};
use crate::error::CoreError;
use crate::events::ParticipantSlashed;
use crate::state::{Participant, Pool, ProtocolConfig, UserReputation};

#[derive(Accounts)]
pub struct SlashUnpaid<'info> {
    /// Permissionless caller — pays tx fee.
    pub caller: Signer<'info>,

    /// Read-only — paused gate.
    #[account(
        seeds = [PROTOCOL_CONFIG_SEED],
        bump = protocol_config.bump,
    )]
    pub protocol_config: Box<Account<'info, ProtocolConfig>>,

    #[account(mut)]
    pub pool: Box<Account<'info, Pool>>,

    /// The participant being slashed.
    #[account(
        mut,
        seeds = [PARTICIPANT_SEED, pool.key().as_ref(), participant.user.as_ref()],
        bump = participant.bump,
        constraint = participant.pool == pool.key() @ CoreError::NotAParticipant,
    )]
    pub participant: Box<Account<'info, Participant>>,

    /// The participant's reputation account — bumped on slash.
    #[account(
        mut,
        seeds = [REPUTATION_SEED, participant.user.as_ref()],
        bump = user_reputation.bump,
    )]
    pub user_reputation: Box<Account<'info, UserReputation>>,

    /// Collateral vault — source of the slashed funds.
    /// CHECK: PDA seed binding + key equality with `pool.collateral_vault`.
    #[account(
        mut,
        seeds = [COLLATERAL_VAULT_SEED, pool.key().as_ref()],
        bump,
        constraint = collateral_vault.key() == pool.collateral_vault
            @ CoreError::Unauthorized,
    )]
    pub collateral_vault: UncheckedAccount<'info>,

    /// Pool USDC vault — transit account.
    /// CHECK: PDA seed binding + key equality with `pool.pool_usdc_vault`.
    #[account(
        mut,
        seeds = [POOL_USDC_VAULT_SEED, pool.key().as_ref()],
        bump,
        constraint = pool_usdc_vault.key() == pool.pool_usdc_vault
            @ CoreError::Unauthorized,
    )]
    pub pool_usdc_vault: UncheckedAccount<'info>,

    /// CHECK: AccountInfo only; bump validated by Anchor seeds. Co-signs
    /// the adapter CPI alongside `pool_usdc_vault`.
    #[account(seeds = [CORE_INVOKER_SEED], bump)]
    pub core_invoker: UncheckedAccount<'info>,

    /// CHECK: validated by the chosen adapter's `deposit`.
    #[account(mut)]
    pub adapter_state: UncheckedAccount<'info>,

    /// CHECK: validated by the chosen adapter's `deposit`.
    #[account(mut)]
    pub adapter_usdc_vault: UncheckedAccount<'info>,

    /// CHECK: program ID validated by `cpi_adapter_deposit` against `pool.tier`.
    pub yield_adapter_program: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

#[inline(never)]
fn cpi_collateral_to_pool_vault<'info>(
    token_program: AccountInfo<'info>,
    collateral_vault: AccountInfo<'info>,
    pool_usdc_vault: AccountInfo<'info>,
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
            to: pool_usdc_vault,
            authority: collateral_vault,
        },
        seeds,
    );
    token::transfer(cpi_ctx, amount)
}

pub fn handle_slash_unpaid<'info>(
    ctx: Context<'info, SlashUnpaid<'info>>,
) -> Result<()> {
    require!(
        !ctx.accounts.protocol_config.paused,
        CoreError::ProtocolPaused
    );

    let now = Clock::get()?.unix_timestamp;

    // ───── 1. Pool gates ────────────────────────────────────────────────
    let current_month: u8;
    let contribution_amount: u64;
    let pool_key = ctx.accounts.pool.key();
    let pool_tier;
    {
        let pool = &ctx.accounts.pool;
        require!(!pool.is_complete, CoreError::PoolComplete);
        require!(
            pool.current_month >= 1 && pool.current_month <= Pool::TOTAL_MONTHS,
            CoreError::PoolNotStarted
        );
        let month_end = pool
            .current_month_started_at
            .checked_add(pool.month_duration_seconds)
            .ok_or(CoreError::MathOverflow)?;
        require!(now >= month_end, CoreError::MonthNotEnded);
        current_month = pool.current_month;
        contribution_amount = pool.contribution_amount;
        pool_tier = pool.tier;
    }

    // ───── 2. Participant gates ─────────────────────────────────────────
    let collateral_locked_before;
    {
        let participant = &ctx.accounts.participant;
        require!(!participant.is_defaulted, CoreError::AlreadyLiquidated);
        require!(
            !participant.has_paid_month(current_month),
            CoreError::NotLate
        );
        collateral_locked_before = participant.collateral_locked;
        require!(collateral_locked_before > 0, CoreError::NothingToSlash);
    }

    // ───── 3. Compute slash amount (clamp to remaining collateral) ──────
    let slash_amount = contribution_amount.min(collateral_locked_before);

    // ───── 4. Token movement ────────────────────────────────────────────
    let collateral_vault_bump = ctx.bumps.collateral_vault;
    let pool_usdc_vault_bump = ctx.bumps.pool_usdc_vault;
    let core_invoker_bump = ctx.bumps.core_invoker;

    cpi_collateral_to_pool_vault(
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.collateral_vault.to_account_info(),
        ctx.accounts.pool_usdc_vault.to_account_info(),
        &pool_key,
        collateral_vault_bump,
        slash_amount,
    )?;

    let combined_seeds: &[&[&[u8]]] = &[
        &[CORE_INVOKER_SEED, &[core_invoker_bump]],
        &[
            POOL_USDC_VAULT_SEED,
            pool_key.as_ref(),
            &[pool_usdc_vault_bump],
        ],
    ];
    cpi_adapter_deposit(
        pool_tier,
        ctx.accounts.yield_adapter_program.to_account_info(),
        ctx.accounts.core_invoker.to_account_info(),
        ctx.accounts.adapter_state.to_account_info(),
        ctx.accounts.adapter_usdc_vault.to_account_info(),
        ctx.accounts.pool_usdc_vault.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        ctx.remaining_accounts,
        combined_seeds,
        slash_amount,
    )?;

    // ───── 5. State updates ─────────────────────────────────────────────
    let participant = &mut ctx.accounts.participant;
    participant.mark_month_paid(current_month);
    // Also flip the `slashed_months` bit so the UI can render the slot
    // distinct from a normal paid month. Both bitmaps are ORed onto;
    // INV-3's monotonic-bit-flip property is preserved.
    participant.mark_month_slashed(current_month);
    let collateral_locked_after =
        collateral_locked_before.saturating_sub(slash_amount);
    participant.collateral_locked = collateral_locked_after;
    let is_defaulted_after = collateral_locked_after == 0;
    if is_defaulted_after {
        participant.is_defaulted = true;
    }
    let user = participant.user;

    let pool = &mut ctx.accounts.pool;
    pool.total_collateral_locked = pool
        .total_collateral_locked
        .saturating_sub(slash_amount);
    pool.paid_count_for_current_month = pool
        .paid_count_for_current_month
        .saturating_add(1);
    pool.total_contributed = pool
        .total_contributed
        .checked_add(slash_amount)
        .ok_or(CoreError::MathOverflow)?;

    let rep = &mut ctx.accounts.user_reputation;
    rep.months_missed_lifetime = rep.months_missed_lifetime.saturating_add(1);

    emit!(ParticipantSlashed {
        pool: pool_key,
        user,
        month: current_month,
        slash_amount,
        collateral_locked_after,
        is_defaulted_after,
        timestamp: now,
    });

    Ok(())
}
