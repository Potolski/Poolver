use anchor_lang::prelude::*;

/// Adapter-local errors. Names align with spec §7's catalogue so that
/// the CPI caller (`poolver-core`) can map adapter errors uniformly
/// across both tiers (INV-21).
#[error_code]
pub enum YieldDefiError {
    #[msg("Caller is not the canonical core_invoker PDA")]
    Unauthorized,
    #[msg("Adapter is in tripped state — call reset_circuit_breaker before further use")]
    CircuitBreakerTripped,
    #[msg("Adapter has insufficient liquidity to satisfy the requested withdrawal")]
    InsufficientLiquidity,
    #[msg("Arithmetic overflow")]
    MathOverflow,
    #[msg("Amount must be non-zero")]
    InvalidAmount,
    #[msg("Caller is not the protocol admin (mock + reset only)")]
    NotAdmin,
}
