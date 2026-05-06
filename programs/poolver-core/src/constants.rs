//! PDA seed prefixes per arch §4. Strings are unique across the program so
//! cross-account-type collisions are structurally impossible.

// ───── Account-PDA seeds ────────────────────────────────────────────────

pub const PROTOCOL_CONFIG_SEED: &[u8] = b"protocol_config";
pub const PROTOCOL_FEE_VAULT_SEED: &[u8] = b"protocol_fee_vault";
pub const POOL_SEED: &[u8] = b"pool";
pub const POOL_USDC_VAULT_SEED: &[u8] = b"pool_usdc_vault";
pub const COLLATERAL_VAULT_SEED: &[u8] = b"collateral_vault";
pub const PARTICIPANT_SEED: &[u8] = b"participant";
pub const REPUTATION_SEED: &[u8] = b"reputation";
pub const KYC_SEED: &[u8] = b"kyc";

/// Per-(pool, month, user) sealed bid PDA (arch §3.4 / §4 PDA seed table).
/// Step 6 — `commit_bid` / `reveal_bid`.
pub const BID_SEED: &[u8] = b"bid";

/// Per-pool USDC token account that escrows the 1% anti-spam bid stake
/// (Q-3). Authority is the token account itself (self-sign via seeds).
/// Step 6 — created in `create_pool`, drained on `reveal_bid` refund.
pub const BID_STAKE_VAULT_SEED: &[u8] = b"bid_stake_vault";

/// Seed for the `core_invoker` PDA — the canonical signer core uses to
/// authenticate CPIs into `poolver-reserve` and the yield adapters
/// (arch §5.2). Both peer programs verify with
/// `seeds::program = poolver_core::ID`, so no other program can mint a
/// matching signature.
pub const CORE_INVOKER_SEED: &[u8] = b"core_invoker";

// ───── Cross-program PDA seeds (mirrored from peer programs) ────────────
//
// We re-import the seed bytes via the peer crates so a future drift on
// either side fails to compile here, not silently at runtime. These are
// the seeds the peer programs publish in their `constants` modules.

pub use poolver_reserve::constants::{RESERVE_FUND_SEED, RESERVE_VAULT_SEED};
pub use poolver_yield_vault::constants::{VAULT_ADAPTER_SEED, VAULT_ADAPTER_USDC_SEED};
// SPEC_QUESTION-36: step 13 — Tier 1 (DeFi) adapter seeds. Imported via
// the peer crate so a future drift on the adapter side is a compile
// error here instead of a silent runtime drift.
pub use poolver_yield_defi::constants::{
    DEFI_ADAPTER_KTOKEN_SEED, DEFI_ADAPTER_SEED, DEFI_ADAPTER_USDC_SEED,
};

// ───── Protocol fee defaults (basis points, spec §4) ────────────────────

pub const DEFAULT_PROTOCOL_FEE_BPS: u16 = 150;
pub const DEFAULT_VAULT_RESERVE_FEE_BPS: u16 = 150;
pub const DEFAULT_DEFI_RESERVE_FEE_BPS: u16 = 250;

/// Basis-points denominator. 10_000 bps = 100%.
pub const BPS_DENOMINATOR: u64 = 10_000;

// ───── Pool config bounds (spec §3) ─────────────────────────────────────

/// Min pool contribution: 100 USDC (6 decimals).
pub const MIN_CONTRIBUTION: u64 = 100_000_000;
/// Max pool contribution: 10,000 USDC (6 decimals).
pub const MAX_CONTRIBUTION: u64 = 10_000_000_000;

/// Fixed pool size (spec §3, arch §3.2).
pub const POOL_SIZE: u8 = 12;
/// Fixed pool length in months (spec §3, arch §3.2).
pub const TOTAL_MONTHS: u8 = 12;

/// Default month duration: 30 days.
pub const DEFAULT_MONTH_DURATION_SECS: i64 = 2_592_000;
/// Default bid window: 48 hours.
pub const DEFAULT_BID_WINDOW_SECS: i64 = 172_800;
/// Default reveal window: 24 hours (SPEC_QUESTION-4 default).
pub const DEFAULT_REVEAL_WINDOW_SECS: i64 = 86_400;

/// Anti-spam bid stake (basis points of `pool.contribution_amount`).
/// SPEC_QUESTION-3: 100 bps = 1% of contribution. Locked at `commit_bid`,
/// refunded on successful `reveal_bid`, forfeit to tier reserve on
/// no-reveal (forfeit path lands in step 7).
pub const BID_STAKE_BPS: u64 = 100;

/// Bid cap as a fraction of monthly pot (basis points). Spec §4 + Q-10.
pub const BID_CAP_BPS: u64 = 2_000;

// ───── KYC defaults ─────────────────────────────────────────────────────

/// Default KYC validity: 12 × 30 days.
pub const DEFAULT_KYC_VALIDITY_SECS: i64 = 12 * DEFAULT_MONTH_DURATION_SECS;

// ───── Default cascade timings (spec §4 + step 10) ──────────────────────

/// One day in seconds. Reused below for grace + liquidation timers.
pub const ONE_DAY_SECS: i64 = 86_400;

/// Last day of the grace period (inclusive). `mark_late_payment` accepts
/// calls in `[month_end, month_end + GRACE_PERIOD_SECS)`. Spec §4 day
/// 1..=5 of unpaid status.
pub const GRACE_PERIOD_SECS: i64 = 5 * ONE_DAY_SECS;

/// Threshold past `month_end` after which `suspend_participant` is the
/// correct call. Spec §4 day 6+.
pub const SUSPENSION_THRESHOLD_SECS: i64 = 6 * ONE_DAY_SECS;

/// Threshold past `month_end` after which `liquidate_default` is the
/// correct call. Spec §4 day 30. `contribute`'s cure-path window
/// terminates at this same boundary — once liquidation is allowed,
/// curing by payment is no longer accepted.
pub const LIQUIDATION_THRESHOLD_SECS: i64 = 30 * ONE_DAY_SECS;

/// Late-payment penalty in basis points. Spec §4 ("200 bps (2%) penalty
/// accrues"). Routed to `pool.bid_credit_balance` per
/// SPEC_QUESTION-6.
pub const LATE_PENALTY_BPS: u64 = 200;
