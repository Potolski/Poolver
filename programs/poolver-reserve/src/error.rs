use anchor_lang::prelude::*;

/// Reserve-local errors. Names align with spec §7's catalogue so the CPI
/// caller (`poolver-core`) can surface them uniformly during the default
/// liquidation cascade (INV-2 / INV-3 / INV-4).
#[error_code]
pub enum ReserveError {
    #[msg("Caller is not the canonical core_invoker PDA")]
    Unauthorized,
    /// INV-2 — `total_balance` must never go negative. `draw` checks this
    /// before performing the SPL transfer; the `checked_sub` is the second
    /// line of defence.
    #[msg("Reserve has insufficient balance to satisfy the draw amount")]
    ReserveInsufficient,
    #[msg("Arithmetic overflow")]
    MathOverflow,
    #[msg("Amount must be non-zero")]
    InvalidAmount,
}
