use anchor_lang::prelude::*;

// Spec §6 demands every state-changing instruction emits an
// indexer-rebuildable event. Tier 1 emits the same five-event surface
// that Tier 0 does (so a single indexer codepath consumes both
// adapters per INV-21 / arch §13), plus three Tier-1-specific events:
// circuit-breaker fired/reset and the mock-only `MockYieldInjected`
// fired by the dev-only injector.

#[event]
pub struct AdapterInitialized {
    pub pool: Pubkey,
    pub adapter_state: Pubkey,
    pub usdc_vault: Pubkey,
    pub ktoken_vault: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct AdapterDeposited {
    pub pool: Pubkey,
    pub amount: u64,
    pub deployed_to_kamino: u64,
    pub kept_liquid: u64,
    pub total_deposited: u64,
    pub timestamp: i64,
}

#[event]
pub struct AdapterWithdrew {
    pub pool: Pubkey,
    pub amount: u64,
    pub from_liquid: u64,
    pub from_kamino: u64,
    pub total_deposited: u64,
    pub timestamp: i64,
}

#[event]
pub struct AdapterHarvested {
    pub pool: Pubkey,
    /// Realized yield since the last harvest call. For Tier 1 this is
    /// `current_balance − last_recorded_balance`; in V1 with the mock
    /// it's whatever amount was injected via `mock_inject_yield`.
    pub yield_amount: u64,
    pub last_recorded_balance: u64,
    pub timestamp: i64,
}

#[event]
pub struct AdapterUnwound {
    pub pool: Pubkey,
    pub amount_unwound: u64,
    pub from_liquid: u64,
    pub from_kamino: u64,
    pub timestamp: i64,
}

#[event]
pub struct CircuitBreakerTripped {
    pub pool: Pubkey,
    pub reason: u8,
    pub timestamp: i64,
}

#[event]
pub struct CircuitBreakerReset {
    pub pool: Pubkey,
    pub previous_reason: u8,
    pub timestamp: i64,
}

// Mock-only events. Gated by the `mock-yield` Cargo feature so they
// disappear from the IDL + binary in `--no-default-features` builds.
// SPEC_QUESTION-19: indexers should ignore these in production.

#[cfg(feature = "mock-yield")]
#[event]
pub struct MockYieldInjected {
    pub pool: Pubkey,
    pub amount: u64,
    pub new_ktoken_balance: u64,
    pub timestamp: i64,
}

#[cfg(feature = "mock-yield")]
#[event]
pub struct MockUtilizationSet {
    pub pool: Pubkey,
    pub bps: u16,
    pub timestamp: i64,
}

#[cfg(feature = "mock-yield")]
#[event]
pub struct MockOracleDeviationSet {
    pub pool: Pubkey,
    pub bps: u16,
    pub timestamp: i64,
}

#[cfg(feature = "mock-yield")]
#[event]
pub struct MockKaminoPausedSet {
    pub pool: Pubkey,
    pub paused: bool,
    pub timestamp: i64,
}
