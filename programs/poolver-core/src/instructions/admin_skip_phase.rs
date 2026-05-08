// Admin-only fast-forward for testing + demo. Detects the pool's
// current phase and skips its time gate forward to "now":
//
//   - bid window open  → close it (sets bid_window_ends_at = now,
//                        starts a fresh reveal window of the per-pool
//                        derived length so reveal_bid still works)
//   - reveal window open → close it (sets reveal_window_ends_at = now)
//   - past reveal window → roll current_month_started_at backwards
//                          enough that advance_month's time check
//                          passes immediately
//
// Does NOT bypass any other invariant (still requires the appropriate
// instructions to actually advance the pool — select_winner, claim,
// advance_month). This is a time-only skip so demos don't have to
// wait through the auction timers.
//
// Admin-gated — signer must equal `protocol_config.admin`. Production
// deploys should NEVER call this; it exists for devnet demo / dev
// convenience only.

use anchor_lang::prelude::*;

use crate::constants::{POOL_SEED, PROTOCOL_CONFIG_SEED};
use crate::error::CoreError;
use crate::events::PhaseSkipped;
use crate::state::{Pool, ProtocolConfig};

#[derive(Accounts)]
pub struct AdminSkipPhase<'info> {
    pub admin: Signer<'info>,

    #[account(
        seeds = [PROTOCOL_CONFIG_SEED],
        bump = protocol_config.bump,
        constraint = protocol_config.admin == admin.key() @ CoreError::Unauthorized,
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,

    #[account(
        mut,
        seeds = [POOL_SEED, pool.creator.as_ref(), &pool.pool_id.to_le_bytes()],
        bump = pool.bump,
    )]
    pub pool: Box<Account<'info, Pool>>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum SkippedPhase {
    BidWindow,
    RevealWindow,
    MonthDuration,
}

pub fn handle_admin_skip_phase(ctx: Context<AdminSkipPhase>) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    let now = Clock::get()?.unix_timestamp;

    require!(pool.current_month >= 1, CoreError::PoolNotStarted);
    require!(!pool.is_complete, CoreError::PoolComplete);

    let phase = if now < pool.bid_window_ends_at {
        // We're still in the bid window. Close it; open the reveal window
        // immediately for the per-pool reveal duration (half of bid_window
        // per the scaling rule, min 60s).
        let reveal_secs = (pool.bid_window_seconds / 2).max(60);
        pool.bid_window_ends_at = now;
        pool.reveal_window_ends_at = now
            .checked_add(reveal_secs)
            .ok_or(CoreError::MathOverflow)?;
        SkippedPhase::BidWindow
    } else if now < pool.reveal_window_ends_at {
        // We're in the reveal window. Close it.
        pool.reveal_window_ends_at = now;
        SkippedPhase::RevealWindow
    } else {
        // Past the reveal window. Make the month appear to have elapsed
        // so advance_month's time check passes. We push
        // current_month_started_at backwards by the month duration plus
        // a 1-second safety margin.
        let backshift = pool
            .month_duration_seconds
            .checked_add(1)
            .ok_or(CoreError::MathOverflow)?;
        pool.current_month_started_at = now
            .checked_sub(backshift)
            .ok_or(CoreError::MathOverflow)?;
        SkippedPhase::MonthDuration
    };

    emit!(PhaseSkipped {
        pool: pool.key(),
        month: pool.current_month,
        phase: phase as u8,
        timestamp: now,
    });

    Ok(())
}
