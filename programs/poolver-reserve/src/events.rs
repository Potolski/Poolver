use anchor_lang::prelude::*;

use crate::state::Tier;

// Spec §6 demands every state-changing instruction emits an indexer-
// rebuildable event. Reserve emits one event per mutation type so an
// indexer can distinguish admin top-ups (`ReserveSeeded`) from core-driven
// inflows (`ReserveDeposit`) and outflows (`ReserveDraw`).
//
// All events carry `tier` so the indexer can attribute the mutation to the
// correct reserve without re-reading account state (INV-36 / INV-3).

#[event]
pub struct ReserveInitialized {
    pub tier: Tier,
    pub reserve_fund: Pubkey,
    pub usdc_vault: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct ReserveDeposit {
    pub tier: Tier,
    pub amount: u64,
    pub total_balance: u64,
    pub total_inflows: u64,
    pub timestamp: i64,
}

#[event]
pub struct ReserveDraw {
    pub tier: Tier,
    pub amount: u64,
    pub total_balance: u64,
    pub total_outflows: u64,
    pub timestamp: i64,
}

#[event]
pub struct ReserveSeeded {
    pub tier: Tier,
    pub amount: u64,
    pub total_balance: u64,
    pub total_inflows: u64,
    pub timestamp: i64,
}
