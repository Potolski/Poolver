//! `liquidate_default` — spec §5.1 (step 10 default cascade, day 30+).
//!
//! Permissionless. The heaviest instruction in the protocol — touches
//! Participant, Pool, UserReputation, the collateral vault, and the
//! tier-encoded reserve via CPI. Two distinct execution paths:
//!
//! **Case A — post-win defaulter.** The participant has already won and
//! claimed their pot but stopped paying back the schedule. We:
//!   1. Liquidate `min(participant.collateral_locked, total_owed)` from
//!      the collateral vault into `pool_usdc_vault` so the remaining
//!      months' contributions are pre-funded.
//!   2. Compute the residual `shortfall = total_owed − collateral_drawn`.
//!   3. If `shortfall > 0`, draw from the tier reserve. We *pre-check*
//!      `reserve_fund.total_balance` and clamp to whatever's drawable
//!      (cleaner than try/catch on the CPI — arch §5.4).
//!   4. Record any uncovered residual as `LiquidationShortfall` for
//!      off-chain alerting. The protocol stays solvent because the
//!      shortfall is recorded, but the deficit must be made up by future
//!      reserve top-ups.
//!
//! **Case B — pre-win defaulter.** The participant joined and may have
//! contributed for some months but never won and stopped paying. Their
//! contributions have already been rotated to past winners (they live in
//! the yield adapter's pool of accumulated USDC). There is NO collateral
//! to slash and nothing physical to claw back. We just mark
//! `is_defaulted = true`, increment `pools_defaulted`, and emit the
//! event. Pool continues without them.
//!
//! ## Solvency proof (INV-1 / arch §12)
//!
//! Case A:
//! ```text
//!   Δcollateral_vault   = −liquidated_from_collateral
//!   Δpool_usdc_vault    = +(liquidated_from_collateral + drawn_from_reserve)
//!   Δreserve_usdc_vault = −drawn_from_reserve
//! ```
//! Sum = 0. Pool has exactly `total_owed − shortfall` worth of pre-funded
//! USDC to cover remaining months; the `shortfall` is recorded but not
//! created.
//!
//! Case B: zero token movement. INV-1 trivially holds.
//!
//! ## Reserve isolation (INV-4 / arch §11)
//!
//! The reserve PDA is re-derived from `pool.tier` BEFORE the CPI. A
//! caller passing the wrong-tier reserve gets `Unauthorized` long before
//! the reserve program is invoked.

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, Transfer};

use crate::constants::{
    COLLATERAL_VAULT_SEED, CORE_INVOKER_SEED, LIQUIDATION_THRESHOLD_SECS,
    PARTICIPANT_SEED, POOL_USDC_VAULT_SEED, PROTOCOL_CONFIG_SEED, REPUTATION_SEED,
    RESERVE_FUND_SEED, RESERVE_VAULT_SEED,
};
use crate::error::CoreError;
use crate::events::{DefaultLiquidated, LiquidationShortfall};
use crate::state::{Participant, Pool, ProtocolConfig, UserReputation};

#[derive(Accounts)]
pub struct LiquidateDefault<'info> {
    /// Permissionless. Pays the tx fee.
    pub caller: Signer<'info>,

    /// Protocol config — read-only (pause check).
    #[account(
        seeds = [PROTOCOL_CONFIG_SEED],
        bump = protocol_config.bump,
    )]
    pub protocol_config: Box<Account<'info, ProtocolConfig>>,

    /// The pool. Mut because we update `total_collateral_locked` on
    /// Case A (collateral leaves the protocol's collateral vault and
    /// rotates into pool_usdc_vault).
    #[account(mut)]
    pub pool: Box<Account<'info, Pool>>,

    /// The defaulting participant.
    #[account(
        mut,
        seeds = [PARTICIPANT_SEED, pool.key().as_ref(), participant.user.as_ref()],
        bump = participant.bump,
        constraint = participant.pool == pool.key() @ CoreError::NotAParticipant,
    )]
    pub participant: Box<Account<'info, Participant>>,

    /// Defaulter's reputation — `pools_defaulted` is incremented.
    /// SPEC_QUESTION-11: this is the global gate that future
    /// `join_pool` calls check; defaulting in pool A blocks new joins
    /// across the board but does NOT yank the user from other active
    /// pools.
    #[account(
        mut,
        seeds = [REPUTATION_SEED, participant.user.as_ref()],
        bump = user_reputation.bump,
        constraint = user_reputation.user == participant.user @ CoreError::Unauthorized,
    )]
    pub user_reputation: Box<Account<'info, UserReputation>>,

    /// Pool USDC vault — receives the liquidated collateral + reserve
    /// drawdown so the remaining-months contributions are pre-funded.
    /// CHECK: PDA seed binding + key equality.
    #[account(
        mut,
        seeds = [POOL_USDC_VAULT_SEED, pool.key().as_ref()],
        bump,
        constraint = pool_usdc_vault.key() == pool.pool_usdc_vault
            @ CoreError::Unauthorized,
    )]
    pub pool_usdc_vault: UncheckedAccount<'info>,

    /// Collateral vault — drained on Case A.
    /// CHECK: PDA seed binding + key equality with `pool.collateral_vault`.
    #[account(
        mut,
        seeds = [COLLATERAL_VAULT_SEED, pool.key().as_ref()],
        bump,
        constraint = collateral_vault.key() == pool.collateral_vault
            @ CoreError::Unauthorized,
    )]
    pub collateral_vault: UncheckedAccount<'info>,

    /// `core_invoker` PDA — co-signs the reserve `draw` CPI (arch §5.2).
    /// CHECK: AccountInfo only; bump validated by Anchor seeds.
    #[account(seeds = [CORE_INVOKER_SEED], bump)]
    pub core_invoker: UncheckedAccount<'info>,

    // ───── Reserve CPI accounts (validated below by tier-encoded seeds) ─
    /// CHECK: re-derived from `pool.tier` in the handler before any CPI;
    /// reserve program also enforces its own seeds.
    #[account(mut)]
    pub reserve_fund: UncheckedAccount<'info>,

    /// CHECK: re-derived from `pool.tier`; reserve program enforces.
    #[account(mut)]
    pub reserve_usdc_vault: UncheckedAccount<'info>,

    /// CHECK: hardcoded program ID.
    #[account(address = poolver_reserve::ID)]
    pub reserve_program: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

// ─── CPI helpers (SPEC_QUESTION-15: split across stack frames) ───────────

#[inline(never)]
fn cpi_collateral_to_pool<'info>(
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

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn cpi_reserve_draw<'info>(
    reserve_program: AccountInfo<'info>,
    core_invoker: AccountInfo<'info>,
    reserve_fund: AccountInfo<'info>,
    reserve_usdc_vault: AccountInfo<'info>,
    pool_usdc_vault: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    core_invoker_bump: u8,
    amount: u64,
) -> Result<()> {
    let cpi_accounts = poolver_reserve::cpi::accounts::ReserveDrawCtx {
        core_invoker,
        reserve_fund,
        reserve_usdc_vault,
        destination_usdc: pool_usdc_vault,
        token_program,
    };
    let signer_seeds: &[&[&[u8]]] = &[&[CORE_INVOKER_SEED, &[core_invoker_bump]]];
    let cpi_ctx = CpiContext::new_with_signer(
        reserve_program.key(),
        cpi_accounts,
        signer_seeds,
    );
    poolver_reserve::cpi::draw(cpi_ctx, amount)
}

pub fn handle_liquidate_default(ctx: Context<LiquidateDefault>) -> Result<()> {
    require!(
        !ctx.accounts.protocol_config.paused,
        CoreError::ProtocolPaused
    );

    let now = Clock::get()?.unix_timestamp;
    let pool_key = ctx.accounts.pool.key();

    // ───── 1. Pool gates ──────────────────────────────────────────────
    let current_month: u8;
    let contribution_amount: u64;
    let liquidation_threshold: i64;
    let pool_tier: crate::state::Tier;
    {
        let pool = &ctx.accounts.pool;
        require!(!pool.is_complete, CoreError::PoolComplete);
        require!(
            pool.current_month >= 1 && pool.current_month <= Pool::TOTAL_MONTHS,
            CoreError::PoolNotStarted
        );
        current_month = pool.current_month;
        contribution_amount = pool.contribution_amount;
        pool_tier = pool.tier;
        let month_end = pool
            .current_month_started_at
            .checked_add(pool.month_duration_seconds)
            .ok_or(CoreError::MathOverflow)?;
        liquidation_threshold = month_end
            .checked_add(LIQUIDATION_THRESHOLD_SECS)
            .ok_or(CoreError::MathOverflow)?;
    }
    require!(
        now >= liquidation_threshold,
        CoreError::DefaultThresholdNotReached
    );

    // ───── 2. Reserve isolation (INV-4 / arch §11) ────────────────────
    // Re-derive the canonical reserve PDAs from `pool.tier` BEFORE any
    // CPI. A caller passing the wrong-tier reserve hits Unauthorized
    // here, never reaching the reserve program.
    let tier_seed = [pool_tier.as_u8()];
    let (expected_reserve_fund, _) =
        Pubkey::find_program_address(&[RESERVE_FUND_SEED, &tier_seed], &poolver_reserve::ID);
    require_keys_eq!(
        ctx.accounts.reserve_fund.key(),
        expected_reserve_fund,
        CoreError::Unauthorized
    );
    let (expected_reserve_vault, _) =
        Pubkey::find_program_address(&[RESERVE_VAULT_SEED, &tier_seed], &poolver_reserve::ID);
    require_keys_eq!(
        ctx.accounts.reserve_usdc_vault.key(),
        expected_reserve_vault,
        CoreError::Unauthorized
    );

    // ───── 3. Participant gates ───────────────────────────────────────
    let was_winner: bool;
    let collateral_locked: u64;
    let win_month: u8;
    let accrued_penalty: u64;
    let user_key: Pubkey;
    {
        let participant = &ctx.accounts.participant;
        // Idempotency: never liquidate twice.
        require!(!participant.is_defaulted, CoreError::AlreadyLiquidated);
        // Defense-in-depth: liquidation only after the suspension flag
        // (set by `suspend_participant` at day 6). If the keeper bot
        // skipped that step, force them to call it first — the
        // suspension is what blocks `commit_bid` etc., so it must
        // happen before liquidation regardless.
        require!(participant.is_suspended, CoreError::NotSuspended);
        // The participant must currently owe a payment.
        require!(
            !participant.has_paid_month(current_month),
            CoreError::NotLate
        );

        was_winner = participant.has_won;
        collateral_locked = participant.collateral_locked;
        win_month = participant.win_month;
        accrued_penalty = participant.late_penalty_accrued;
        user_key = participant.user;
    }

    // ───── 4. Compute total owed (Case A) ─────────────────────────────
    //
    // For a post-win defaulter, the "owed" amount is the remaining
    // schedule of contributions they're on the hook for plus any accrued
    // late penalty. We INCLUDE the current unpaid month + every
    // subsequent month up to TOTAL_MONTHS.
    //
    // For a non-winner default (Case B), we don't touch any tokens —
    // their already-paid contributions are sunk into past distributions.

    let mut liquidated_from_collateral: u64 = 0;
    let mut drawn_from_reserve: u64 = 0;
    let mut shortfall: u64 = 0;
    let total_owed: u64;

    if was_winner {
        // win_month .. TOTAL_MONTHS  inclusive  → number of remaining
        // contribution slots is TOTAL_MONTHS - current_month + 1
        // (current month + all subsequent months).
        let months_remaining = (Pool::TOTAL_MONTHS as u64)
            .checked_sub(current_month as u64)
            .and_then(|v| v.checked_add(1))
            .ok_or(CoreError::MathOverflow)?;
        let outstanding_contributions = months_remaining
            .checked_mul(contribution_amount)
            .ok_or(CoreError::MathOverflow)?;
        total_owed = outstanding_contributions
            .checked_add(accrued_penalty)
            .ok_or(CoreError::MathOverflow)?;

        // Bound the win_month sanity-check (defensive — can't liquidate
        // a "winner" that hasn't actually won a real month).
        require!(
            win_month >= 1 && win_month <= Pool::TOTAL_MONTHS,
            CoreError::Unauthorized
        );

        // ───── 5. Liquidate collateral up to total_owed ──────────────
        liquidated_from_collateral = core::cmp::min(collateral_locked, total_owed);
        let collateral_vault_bump = ctx.bumps.collateral_vault;
        if liquidated_from_collateral > 0 {
            cpi_collateral_to_pool(
                ctx.accounts.token_program.to_account_info(),
                ctx.accounts.collateral_vault.to_account_info(),
                ctx.accounts.pool_usdc_vault.to_account_info(),
                &pool_key,
                collateral_vault_bump,
                liquidated_from_collateral,
            )?;
        }

        let residual = total_owed
            .checked_sub(liquidated_from_collateral)
            .ok_or(CoreError::MathOverflow)?;

        // ───── 6. Reserve drawdown (clamped pre-flight) ──────────────
        if residual > 0 {
            // Manual deserialize to read the reserve's `total_balance`
            // BEFORE invoking `draw`. Cleaner than try/catch on the CPI
            // (arch §5.4).
            let reserve_balance: u64 = {
                let acct = &ctx.accounts.reserve_fund;
                require_keys_eq!(
                    *acct.owner,
                    poolver_reserve::ID,
                    CoreError::Unauthorized
                );
                let mut data: &[u8] = &acct.try_borrow_data()?;
                let fund =
                    poolver_reserve::state::ReserveFund::try_deserialize(&mut data)?;
                fund.total_balance
            };
            let drawable = core::cmp::min(residual, reserve_balance);
            if drawable > 0 {
                let core_invoker_bump = ctx.bumps.core_invoker;
                cpi_reserve_draw(
                    ctx.accounts.reserve_program.to_account_info(),
                    ctx.accounts.core_invoker.to_account_info(),
                    ctx.accounts.reserve_fund.to_account_info(),
                    ctx.accounts.reserve_usdc_vault.to_account_info(),
                    ctx.accounts.pool_usdc_vault.to_account_info(),
                    ctx.accounts.token_program.to_account_info(),
                    core_invoker_bump,
                    drawable,
                )?;
                drawn_from_reserve = drawable;
            }
            shortfall = residual
                .checked_sub(drawn_from_reserve)
                .ok_or(CoreError::MathOverflow)?;
        }
    } else {
        // Case B — non-winner default. NO token movement.
        total_owed = 0;
    }

    // ───── 7. State updates ───────────────────────────────────────────
    {
        let participant = &mut ctx.accounts.participant;
        participant.is_defaulted = true;
        participant.defaulted_at = now;
        // Defense: ensure suspended too (it already is per the gate).
        participant.is_suspended = true;
        if was_winner {
            // INV-4: collateral monotonic decrease.
            participant.collateral_locked = participant
                .collateral_locked
                .saturating_sub(liquidated_from_collateral);
            // Penalty consumed (rolled into total_owed).
            participant.late_penalty_accrued = 0;
            participant.liquidation_amount = liquidated_from_collateral
                .checked_add(drawn_from_reserve)
                .ok_or(CoreError::MathOverflow)?;
        }
    }

    if was_winner && liquidated_from_collateral > 0 {
        let pool = &mut ctx.accounts.pool;
        pool.total_collateral_locked = pool
            .total_collateral_locked
            .saturating_sub(liquidated_from_collateral);
    }

    {
        let rep = &mut ctx.accounts.user_reputation;
        rep.pools_defaulted = rep
            .pools_defaulted
            .checked_add(1)
            .ok_or(CoreError::MathOverflow)?;
    }

    // ───── 8. Events ──────────────────────────────────────────────────
    emit!(DefaultLiquidated {
        pool: pool_key,
        user: user_key,
        month: current_month,
        was_winner,
        total_owed,
        liquidated_from_collateral,
        drawn_from_reserve,
        shortfall,
        timestamp: now,
    });
    if shortfall > 0 {
        emit!(LiquidationShortfall {
            pool: pool_key,
            user: user_key,
            month: current_month,
            shortfall,
            timestamp: now,
        });
    }

    Ok(())
}
