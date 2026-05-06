//! `mark_late_payment` — spec §5.1 (step 10 default cascade).
//!
//! Permissionless keeper instruction. Flags a participant as late once
//! the strict in-window contribution window has elapsed AND we are still
//! inside the 5-day grace period. Side-effects:
//!
//!   - `participant.is_late = true`
//!   - `participant.late_marked_at = now`
//!   - `participant.late_penalty_accrued += 2% × pool.contribution_amount`
//!     (SPEC_QUESTION-6 — penalty is added on next `contribute`, routed
//!     to `pool.bid_credit_balance`)
//!
//! No CPIs, no token movement. Pure state change. Emits `LatePayment`.
//!
//! ## Timing windows (spec §4)
//!
//! ```text
//!   day 0           → strict in-window (`contribute` happy path)
//!   day 1..=5       → grace period; mark_late_payment accepts
//!   day 6..=29      → suspension window; suspend_participant accepts
//!   day 30+         → liquidation; liquidate_default accepts
//! ```
//!
//! `mark_late_payment` rejects outside the day 1..=5 grace bucket.
//!
//! ## Idempotency
//!
//! Single mark per (participant, month). A second call within the same
//! month reverts with `AlreadyMarkedLate`. The `late_marked_at` timestamp
//! is checked against `pool.current_month_started_at` — if the participant
//! was marked late this month already, `late_marked_at` will be ≥
//! `current_month_started_at`. After `advance_month`, the field is stale
//! and a new mark is allowed (in case a participant misses a subsequent
//! month after curing a previous one).

use anchor_lang::prelude::*;

use crate::constants::{
    BPS_DENOMINATOR, GRACE_PERIOD_SECS, LATE_PENALTY_BPS, PARTICIPANT_SEED,
    PROTOCOL_CONFIG_SEED,
};
use crate::error::CoreError;
use crate::events::LatePayment;
use crate::state::{Participant, Pool, ProtocolConfig};

#[derive(Accounts)]
pub struct MarkLatePayment<'info> {
    /// Permissionless keeper. Pays the tx fee only.
    pub caller: Signer<'info>,

    /// Read-only protocol config — pause check (INV-25).
    #[account(
        seeds = [PROTOCOL_CONFIG_SEED],
        bump = protocol_config.bump,
    )]
    pub protocol_config: Box<Account<'info, ProtocolConfig>>,

    /// Pool. Read-only — defaults touch only Participant state.
    pub pool: Box<Account<'info, Pool>>,

    /// The participant being marked late.
    #[account(
        mut,
        seeds = [PARTICIPANT_SEED, pool.key().as_ref(), participant.user.as_ref()],
        bump = participant.bump,
        constraint = participant.pool == pool.key() @ CoreError::NotAParticipant,
    )]
    pub participant: Box<Account<'info, Participant>>,
}

pub fn handle_mark_late_payment(ctx: Context<MarkLatePayment>) -> Result<()> {
    require!(
        !ctx.accounts.protocol_config.paused,
        CoreError::ProtocolPaused
    );

    let now = Clock::get()?.unix_timestamp;

    // ───── 1. Pool gates ──────────────────────────────────────────────
    let current_month: u8;
    let contribution_amount: u64;
    let month_end: i64;
    let current_month_started_at: i64;
    {
        let pool = &ctx.accounts.pool;
        require!(!pool.is_complete, CoreError::PoolComplete);
        require!(
            pool.current_month >= 1 && pool.current_month <= Pool::TOTAL_MONTHS,
            CoreError::PoolNotStarted
        );
        current_month = pool.current_month;
        contribution_amount = pool.contribution_amount;
        current_month_started_at = pool.current_month_started_at;
        month_end = pool
            .current_month_started_at
            .checked_add(pool.month_duration_seconds)
            .ok_or(CoreError::MathOverflow)?;
    }

    // ───── 2. Time-window gate (spec §4 day 1..=5) ────────────────────
    require!(now >= month_end, CoreError::GracePeriodNotElapsed);
    let grace_end = month_end
        .checked_add(GRACE_PERIOD_SECS)
        .ok_or(CoreError::MathOverflow)?;
    require!(now < grace_end, CoreError::GracePeriodElapsed);

    // ───── 3. Participant gates ───────────────────────────────────────
    let participant = &mut ctx.accounts.participant;
    require!(!participant.is_defaulted, CoreError::AlreadyLiquidated);
    // Must owe a payment for the current month.
    require!(
        !participant.has_paid_month(current_month),
        CoreError::NotLate
    );
    // Single mark per month: if already marked AND the mark was within
    // the current-month window, reject.
    require!(
        !(participant.is_late && participant.late_marked_at >= current_month_started_at),
        CoreError::AlreadyMarkedLate
    );

    // ───── 4. Compute penalty (Q-6: 200 bps of contribution) ──────────
    let penalty_added = contribution_amount
        .checked_mul(LATE_PENALTY_BPS)
        .and_then(|v| v.checked_div(BPS_DENOMINATOR))
        .ok_or(CoreError::MathOverflow)?;

    // ───── 5. State updates ───────────────────────────────────────────
    participant.is_late = true;
    participant.late_marked_at = now;
    participant.late_penalty_accrued = participant
        .late_penalty_accrued
        .checked_add(penalty_added)
        .ok_or(CoreError::MathOverflow)?;

    let accrued_penalty = participant.late_penalty_accrued;
    let user = participant.user;

    emit!(LatePayment {
        pool: ctx.accounts.pool.key(),
        user,
        month: current_month,
        penalty_added,
        accrued_penalty,
        timestamp: now,
    });

    Ok(())
}
