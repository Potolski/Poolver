// PDA seed prefixes per arch §4. Strings are unique across the program;
// the per-program collision-freedom argument lives in arch §4.

/// Seed for `DefiAdapterState` PDA. Per-pool. Arch §4.
pub const DEFI_ADAPTER_SEED: &[u8] = b"defi_adapter";

/// Seed for `DefiAdapterUsdc` (the LIQUID 25%) token-account authority
/// PDA. Per-pool. Arch §4.
pub const DEFI_ADAPTER_USDC_SEED: &[u8] = b"defi_adapter_usdc";

/// Seed for `DefiAdapterKtoken` (the Kamino-deployed 75%) token-account
/// authority PDA. Per-pool. In the V1 mock this is a second USDC token
/// account that simulates the Kamino kToken position by holding the
/// "deployed" funds; SPEC_QUESTION-19 marks every callsite where the
/// real Kamino integration replaces it.
pub const DEFI_ADAPTER_KTOKEN_SEED: &[u8] = b"defi_adapter_ktoken";

/// Seed for the `core_invoker` PDA owned by `poolver-core`. Used as the
/// sole signer that proves "this CPI came from core". Arch §4 + §5.2.
pub const CORE_INVOKER_SEED: &[u8] = b"core_invoker";

/// Tier 1 capital allocation split (spec §4 + §5.3 / arch §3.9).
/// 7500 bps = 75% deployed to Kamino, the remaining 2500 bps = 25%
/// stays liquid in the adapter's USDC vault.
pub const KAMINO_DEPLOYED_BPS: u64 = 7_500;
pub const BPS_DENOMINATOR: u64 = 10_000;

/// Circuit-breaker thresholds (spec §4 + §5.3). Reproduced here so the
/// adapter is self-contained; the same values live on `poolver-core`'s
/// `ProtocolConfig` for global reference.
pub const UTILIZATION_TRIP_BPS: u16 = 9_500;
pub const ORACLE_DEVIATION_TRIP_BPS: u16 = 200;

/// Trip-reason discriminants written to `DefiAdapterState.tripped_reason`
/// when the breaker fires. Exposed as `u8` so the field stays
/// upgrade-safe (arch §3.9 reserved tail).
pub const TRIP_REASON_NONE: u8 = 0;
pub const TRIP_REASON_UTILIZATION: u8 = 1;
pub const TRIP_REASON_ORACLE_DEVIATION: u8 = 2;
pub const TRIP_REASON_PAUSED: u8 = 3;
pub const TRIP_REASON_ADMIN_TRIP: u8 = 4;
