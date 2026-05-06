use anchor_lang::prelude::*;

// Spec §6 demands every state-changing instruction emits an indexer-rebuildable
// event. Tier 0 emits the same event surface that Tier 1 will mirror, so a
// single indexer codepath consumes both adapters (INV-21 / arch §13).

#[event]
pub struct AdapterInitialized {
    pub pool: Pubkey,
    pub adapter_state: Pubkey,
    pub usdc_vault: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct AdapterDeposited {
    pub pool: Pubkey,
    pub amount: u64,
    pub total_deposited: u64,
    pub timestamp: i64,
}

#[event]
pub struct AdapterWithdrew {
    pub pool: Pubkey,
    pub amount: u64,
    pub total_deposited: u64,
    pub timestamp: i64,
}

#[event]
pub struct AdapterHarvested {
    pub pool: Pubkey,
    /// Tier 0 always emits 0 here; the field exists for indexer parity with
    /// Tier 1 where a non-zero realized yield is expected.
    pub yield_amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct AdapterUnwound {
    pub pool: Pubkey,
    pub amount_unwound: u64,
    pub timestamp: i64,
}
