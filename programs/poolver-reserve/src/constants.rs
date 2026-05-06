// PDA seed prefixes per arch §4. Strings unique across program; collision-freedom argued there.

/// Seed for `ReserveFund` PDA. Tier-encoded — see arch §4 + §11. The full
/// seed list is `[RESERVE_FUND_SEED, &(tier as u8).to_le_bytes()]`.
pub const RESERVE_FUND_SEED: &[u8] = b"reserve_fund";

/// Seed for `ReserveVault` token-account authority PDA. Tier-encoded. Arch §4.
pub const RESERVE_VAULT_SEED: &[u8] = b"reserve_vault";

/// Seed for the `core_invoker` PDA owned by `poolver-core`. Used as the sole
/// signer that proves "this CPI came from core". Arch §4 + §5.2.
pub const CORE_INVOKER_SEED: &[u8] = b"core_invoker";
