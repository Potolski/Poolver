use anchor_lang::prelude::*;

use crate::constants::PROTOCOL_CONFIG_SEED;
use crate::error::CoreError;
use crate::events::{MonthAdvanced, PoolCompleted};
use crate::state::{Pool, ProtocolConfig};

/// `advance_month` — spec §5.1.
///
/// **Permissionless.** Anyone may call once the current-month duration
/// has elapsed. Increments `pool.current_month`, resets the bid + reveal
/// windows for the next month, and (on month 12 → 13) marks the pool
/// complete.
///
/// SPEC_QUESTION-5: spec calls for a 24-hour winner-claim window gate
/// before a month can advance. Step 5 only checks that the month
/// duration has elapsed; the winner-claim gate lands when step 8
/// (`claim_winning`) ships. Re-enable the assertion below at that time.
///
/// SPEC_QUESTION-15: Pool is `Box`'d to keep the BPF stack frame lean.
#[derive(Accounts)]
pub struct AdvanceMonth<'info> {
    /// Anyone — permissionless. The signer just pays the tx fee.
    pub caller: Signer<'info>,

    /// Read-only protocol config; the only thing we need from it is the
    /// `paused` flag. Box'd to match the rest of the program.
    #[account(
        seeds = [PROTOCOL_CONFIG_SEED],
        bump = protocol_config.bump,
    )]
    pub protocol_config: Box<Account<'info, ProtocolConfig>>,

    #[account(mut)]
    pub pool: Box<Account<'info, Pool>>,
}

pub fn handle_advance_month(ctx: Context<AdvanceMonth>) -> Result<()> {
    require!(
        !ctx.accounts.protocol_config.paused,
        CoreError::ProtocolPaused
    );

    let pool = &mut ctx.accounts.pool;
    require!(!pool.is_complete, CoreError::PoolComplete);
    require!(
        pool.current_month >= 1 && pool.current_month <= Pool::TOTAL_MONTHS,
        CoreError::PoolNotStarted
    );

    let now = Clock::get()?.unix_timestamp;
    let month_end = pool
        .current_month_started_at
        .checked_add(pool.month_duration_seconds)
        .ok_or(CoreError::MathOverflow)?;
    // SPEC_QUESTION-5: step 8 will additionally require that the month
    // winner has claimed (or the 24h claim window expired).
    require!(now >= month_end, CoreError::MonthDurationNotElapsed);

    let next_month = pool
        .current_month
        .checked_add(1)
        .ok_or(CoreError::MathOverflow)?;
    pool.current_month = next_month;

    if next_month > Pool::TOTAL_MONTHS {
        // Pool just rolled past month 12 → complete. Indexers can mark
        // the pool archived from `PoolCompleted`. Cycle finalization
        // (collateral fully released, reputation `pools_completed`
        // incremented for non-defaulters) happens in step 11
        // (`finalize_pool`). For step 5 we only flip `is_complete` and
        // stamp `completed_at` so subsequent instructions
        // (incl. `contribute`) can hard-reject.
        pool.is_complete = true;
        pool.completed_at = now;
        emit!(PoolCompleted {
            pool: pool.key(),
            total_contributed: pool.total_contributed,
            total_distributed: pool.total_distributed,
            completed_at: now,
        });
    } else {
        pool.current_month_started_at = now;
        pool.bid_window_ends_at = now
            .checked_add(pool.bid_window_seconds)
            .ok_or(CoreError::MathOverflow)?;
        // SPEC_QUESTION-4: derive reveal window from the per-pool bid
        // window (half of bid, min 60s). For production-default 48h bid
        // this gives a 24h reveal — same as the original hardcoded
        // constant. For demo pools with short months this scales down
        // proportionally (a 5-min bid yields a 2.5-min reveal).
        let reveal_secs = (pool.bid_window_seconds / 2).max(60);
        pool.reveal_window_ends_at = pool
            .bid_window_ends_at
            .checked_add(reveal_secs)
            .ok_or(CoreError::MathOverflow)?;

        // SPEC_QUESTION-1 (step 8): roll the bid-credit denominator over
        // for the new month. Any unspent `bid_credit_balance` carries
        // forward (no token movement here — it's a virtual ledger), but
        // the per-month "how many have paid" counter resets so the next
        // month's contributors get a fresh pro-rata share.
        pool.paid_count_for_current_month = 0;

        emit!(MonthAdvanced {
            pool: pool.key(),
            new_month: next_month,
            timestamp: now,
        });
    }

    Ok(())
}
