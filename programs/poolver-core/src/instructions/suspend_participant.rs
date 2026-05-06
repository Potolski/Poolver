//! `suspend_participant` — spec §5.1 (step 10 default cascade).
//!
//! Permissionless keeper instruction. Flips `participant.is_suspended =
//! true` once we are past day 6 of the unpaid window. Side-effects:
//!
//!   - `participant.is_suspended = true`
//!   - `participant.suspended_at = now`
//!   - `participant.is_late = true` (defense-in-depth — caller may have
//!     skipped `mark_late_payment` entirely)
//!
//! No CPIs, no token movement. Pure state change. Emits
//! `ParticipantSuspended`.
//!
//! ## Effect on other instructions
//!
//! - `commit_bid`: rejects (already wired in step 6 — `Suspended` error)
//! - `contribute`: STILL accepts (cure path — paying clears suspension)
//! - `liquidate_default`: requires `is_suspended == true` as a pre-flight
//!
//! ## Timing windows (spec §4)
//!
//! ```text
//!   day 6..=29  → suspend_participant accepts
//!   day 30+     → liquidate_default takes over (suspension is implied)
//! ```
//!
//! ## Idempotency
//!
//! `is_suspended == true` blocks further `suspend_participant` calls
//! within the same suspension episode. After cure (via `contribute`),
//! the flag clears and a future month's missed payment can re-trigger
//! the cascade.

use anchor_lang::prelude::*;

use crate::constants::{
    PARTICIPANT_SEED, PROTOCOL_CONFIG_SEED, SUSPENSION_THRESHOLD_SECS,
};
use crate::error::CoreError;
use crate::events::ParticipantSuspended;
use crate::state::{Participant, Pool, ProtocolConfig};

#[derive(Accounts)]
pub struct SuspendParticipant<'info> {
    pub caller: Signer<'info>,

    #[account(
        seeds = [PROTOCOL_CONFIG_SEED],
        bump = protocol_config.bump,
    )]
    pub protocol_config: Box<Account<'info, ProtocolConfig>>,

    pub pool: Box<Account<'info, Pool>>,

    #[account(
        mut,
        seeds = [PARTICIPANT_SEED, pool.key().as_ref(), participant.user.as_ref()],
        bump = participant.bump,
        constraint = participant.pool == pool.key() @ CoreError::NotAParticipant,
    )]
    pub participant: Box<Account<'info, Participant>>,
}

pub fn handle_suspend_participant(ctx: Context<SuspendParticipant>) -> Result<()> {
    require!(
        !ctx.accounts.protocol_config.paused,
        CoreError::ProtocolPaused
    );

    let now = Clock::get()?.unix_timestamp;

    // ───── 1. Pool gates ──────────────────────────────────────────────
    let current_month: u8;
    let suspension_threshold: i64;
    {
        let pool = &ctx.accounts.pool;
        require!(!pool.is_complete, CoreError::PoolComplete);
        require!(
            pool.current_month >= 1 && pool.current_month <= Pool::TOTAL_MONTHS,
            CoreError::PoolNotStarted
        );
        current_month = pool.current_month;
        let month_end = pool
            .current_month_started_at
            .checked_add(pool.month_duration_seconds)
            .ok_or(CoreError::MathOverflow)?;
        suspension_threshold = month_end
            .checked_add(SUSPENSION_THRESHOLD_SECS)
            .ok_or(CoreError::MathOverflow)?;
    }

    // ───── 2. Time gate (spec §4 day 6+) ──────────────────────────────
    require!(now >= suspension_threshold, CoreError::GracePeriodNotElapsed);

    // ───── 3. Participant gates ───────────────────────────────────────
    let participant = &mut ctx.accounts.participant;
    require!(!participant.is_defaulted, CoreError::AlreadyLiquidated);
    require!(!participant.is_suspended, CoreError::Suspended);
    require!(
        !participant.has_paid_month(current_month),
        CoreError::NotLate
    );

    // ───── 4. State updates ───────────────────────────────────────────
    participant.is_suspended = true;
    participant.suspended_at = now;
    // Defense-in-depth: ensure `is_late` is also set even if
    // `mark_late_payment` was skipped by the keeper bot.
    participant.is_late = true;

    let user = participant.user;

    emit!(ParticipantSuspended {
        pool: ctx.accounts.pool.key(),
        user,
        month: current_month,
        timestamp: now,
    });

    Ok(())
}
