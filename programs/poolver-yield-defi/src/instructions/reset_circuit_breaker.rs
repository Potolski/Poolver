use anchor_lang::prelude::*;

use crate::constants::{DEFI_ADAPTER_SEED, TRIP_REASON_NONE};
use crate::events::CircuitBreakerReset;
use crate::state::DefiAdapterState;

/// `reset_circuit_breaker` — admin-only instruction that clears the
/// `tripped` latch after manual investigation. NOT a `core_invoker`
/// CPI; this is operator-driven (spec §4 + §5.3 — circuit-breaker
/// recovery is an out-of-band human decision).
///
/// SPEC_QUESTION-26: V1 V1 doesn't gate on a real admin pubkey
/// (hackathon scope, no admin rotation infra yet). Any signer can
/// call this in V1; in production the constraint should pin the
/// signer to `protocol_config.admin` (or a multisig — Q-25). The
/// `// SPEC_QUESTION-26:` marker below is the swap site.
#[derive(Accounts)]
pub struct ResetCircuitBreaker<'info> {
    /// SPEC_QUESTION-26: in production, constrain `admin == protocol_config.admin`.
    /// V1 leaves it open so the hackathon demo + tests don't need to
    /// thread `protocol_config` through the adapter just to clear a
    /// breaker. The breaker is itself a self-DoS mechanism, not a
    /// theft vector — clearing it can't drain funds.
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [DEFI_ADAPTER_SEED, adapter_state.pool.as_ref()],
        bump = adapter_state.bump,
    )]
    pub adapter_state: Account<'info, DefiAdapterState>,
}

pub fn handle_reset_circuit_breaker(ctx: Context<ResetCircuitBreaker>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let state = &mut ctx.accounts.adapter_state;
    let previous_reason = state.tripped_reason;

    state.tripped = false;
    state.tripped_reason = TRIP_REASON_NONE;
    state.tripped_at = 0;

    // Clear the mock-only breaker inputs as part of the reset so a
    // stale `mock_kamino_paused = true` doesn't immediately re-trip
    // the breaker on the next deposit. SPEC_QUESTION-19: in
    // production these fields don't exist; the breaker re-evaluates
    // against the live Kamino + Pyth state on the next deposit.
    state.mock_utilization_bps = 5_000;
    state.mock_oracle_deviation_bps = 0;
    state.mock_kamino_paused = false;

    emit!(CircuitBreakerReset {
        pool: state.pool,
        previous_reason,
        timestamp: now,
    });

    Ok(())
}
