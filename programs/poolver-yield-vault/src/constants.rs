// PDA seed prefixes per arch §4. Strings unique across program; collision-freedom argued there.

/// Seed for `VaultAdapterState` PDA. Per-pool. Arch §4.
pub const VAULT_ADAPTER_SEED: &[u8] = b"vault_adapter";

/// Seed for `VaultAdapterUsdc` token-account authority PDA. Per-pool. Arch §4.
pub const VAULT_ADAPTER_USDC_SEED: &[u8] = b"vault_adapter_usdc";

/// Seed for the `core_invoker` PDA owned by `poolver-core`. Used as the sole
/// signer that proves "this CPI came from core". Arch §4 + §5.2.
pub const CORE_INVOKER_SEED: &[u8] = b"core_invoker";
