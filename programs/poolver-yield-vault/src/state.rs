use anchor_lang::prelude::*;

/// Tier 0 yield-adapter state. Layout fixed by arch §3.8 (81 bytes total
/// including Anchor's 8-byte discriminator). The struct below contributes 73
/// bytes; Anchor adds the discriminator on top → 81. Field order MUST stay
/// stable so a future upgrade can be done without account reallocation.
///
/// `total_deposited` is the cumulative net deposit ledger; the authoritative
/// USDC balance lives in `VaultAdapterUsdc`. We never trust this field for
/// solvency checks — see INV-21 / spec §9.1 ("never trust adapter return
/// values without bounds-checking").
#[account]
#[derive(InitSpace)]
pub struct VaultAdapterState {
    pub pool: Pubkey,
    pub usdc_vault: Pubkey,
    pub total_deposited: u64,
    pub bump: u8,
}
