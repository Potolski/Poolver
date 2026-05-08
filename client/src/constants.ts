/**
 * Program IDs, PDA seed prefixes, and protocol-wide numeric constants.
 *
 * MUST stay byte-identical with the per-program constants.rs files.
 * If a Rust constant changes there, change it here too — there is no
 * automated drift detector.
 */
import { PublicKey } from "@solana/web3.js";
import BN from "bn.js";

// ─────────────────────────── Program IDs ───────────────────────────────

export const POOLVER_CORE_PROGRAM_ID = new PublicKey(
  "2SsxJqMCYKCYesfzfXASgAPPz153j8tYMXpMKKmt2QXk"
);
export const POOLVER_RESERVE_PROGRAM_ID = new PublicKey(
  "CfxRT3jsXWQZRev67ztqaNKCrHaKF6ieW9a1E8NDPvnx"
);
export const POOLVER_YIELD_VAULT_PROGRAM_ID = new PublicKey(
  "A3ERUDLAdqdwgqgAoYLftxA6F1QtxSHZYu8DpNDXyyUp"
);
export const POOLVER_YIELD_DEFI_PROGRAM_ID = new PublicKey(
  "DAitPF7KHzRDVWcV4XM3J7dYGrKJkH332dQHPYUiP7UP"
);

// ─────────────────────────── USDC mint ─────────────────────────────────
// USDC has 6 decimals on every Solana cluster.

export const USDC_DECIMALS = 6;

// Mainnet beta USDC (Circle).
export const USDC_MINT_MAINNET = new PublicKey(
  "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
);

// Devnet mock USDC. Created by `spl-token create-token --decimals 6` on
// 2026-05-07; mint authority = deploy wallet (FFwSGSnHwBkJve7dYhdKcq2JtpMxmA2rT7fJ9i2zxNFq).
// The live devnet ProtocolConfig + ReserveFunds were rotated to use this
// mint via admin_close_protocol/reserve + re-init on 2026-05-07.
// Faucet via `scripts/faucet.ts` (admin only) or the SDK's mintTo helper.
export const USDC_MINT_DEVNET_DEFAULT = new PublicKey(
  "B6dnuZtKH7FsSK6tySfWkk6ReW2LdKpmnfGAoMKsv8w8"
);

// Devnet admin / deploy wallet. Used as the gate for admin-only
// instructions (admin_close_protocol, mock_issue_kyc, admin_skip_phase).
// In production this is replaced by a multisig (Squads); the SDK only
// uses it for client-side UI gating ("show admin button if connected
// wallet matches this pubkey"). The on-chain enforcement lives in the
// `has_one = admin` / `protocol_config.admin == admin.key()` constraints
// on each admin instruction.
export const ADMIN_PUBKEY_DEVNET = new PublicKey(
  "FFwSGSnHwBkJve7dYhdKcq2JtpMxmA2rT7fJ9i2zxNFq"
);

// ─────────────────────────── Address Lookup Table ─────────────────────
// Holds 8 protocol-static addresses (protocol_config, core_invoker,
// reserve_fund × tier, reserve_vault × tier, reserve_program,
// token_program). Used by `select_winner` to fit a 12-non-bidder lottery
// draw under the 1232-byte legacy-tx wire cap. Bootstrapped once via
// `scripts/setup-alt.ts` and immutable thereafter (authority = admin).
export const POOLVER_ALT_DEVNET = new PublicKey(
  "8hvEVzjkh8hnr4qfqYNbaj4AL33F4aFbd1DyMQ1x6JeW"
);

// ─────────────────────────── PDA seeds (core) ─────────────────────────
// Mirror of programs/poolver-core/src/constants.rs.

export const PROTOCOL_CONFIG_SEED = Buffer.from("protocol_config");
export const PROTOCOL_FEE_VAULT_SEED = Buffer.from("protocol_fee_vault");
export const POOL_SEED = Buffer.from("pool");
export const POOL_USDC_VAULT_SEED = Buffer.from("pool_usdc_vault");
export const COLLATERAL_VAULT_SEED = Buffer.from("collateral_vault");
export const PARTICIPANT_SEED = Buffer.from("participant");
export const REPUTATION_SEED = Buffer.from("reputation");
export const KYC_SEED = Buffer.from("kyc");
export const BID_SEED = Buffer.from("bid");
export const BID_STAKE_VAULT_SEED = Buffer.from("bid_stake_vault");
export const CORE_INVOKER_SEED = Buffer.from("core_invoker");

// ─────────────────────────── PDA seeds (reserve) ──────────────────────
// Mirror of programs/poolver-reserve/src/constants.rs.

export const RESERVE_FUND_SEED = Buffer.from("reserve_fund");
export const RESERVE_VAULT_SEED = Buffer.from("reserve_vault");

// ─────────────────────────── PDA seeds (adapters) ─────────────────────

export const VAULT_ADAPTER_SEED = Buffer.from("vault_adapter");
export const VAULT_ADAPTER_USDC_SEED = Buffer.from("vault_adapter_usdc");
export const DEFI_ADAPTER_SEED = Buffer.from("defi_adapter");
export const DEFI_ADAPTER_USDC_SEED = Buffer.from("defi_adapter_usdc");
export const DEFI_ADAPTER_KTOKEN_SEED = Buffer.from("defi_adapter_ktoken");

// ─────────────────────────── Protocol numerics ────────────────────────
// Mirror of programs/poolver-core/src/constants.rs (basis points).

export const BPS_DENOMINATOR = 10_000;
export const DEFAULT_PROTOCOL_FEE_BPS = 150;
export const DEFAULT_VAULT_RESERVE_FEE_BPS = 150;
export const DEFAULT_DEFI_RESERVE_FEE_BPS = 250;
export const BID_STAKE_BPS = 100;
export const BID_CAP_BPS = 2_000;
export const LATE_PENALTY_BPS = 200;

// ─────────────────────────── Pool config bounds ───────────────────────

/** Minimum pool contribution: 100 USDC = 100_000_000 microUSDC. */
export const MIN_CONTRIBUTION = new BN("100000000");
/** Maximum pool contribution: 10,000 USDC = 10_000_000_000 microUSDC. */
export const MAX_CONTRIBUTION = new BN("10000000000");

export const POOL_SIZE = 12;
export const TOTAL_MONTHS = 12;

/** Default month duration: 30 days. */
export const DEFAULT_MONTH_DURATION_SECS = 2_592_000;
/** Default bid window: 48 hours. */
export const DEFAULT_BID_WINDOW_SECS = 172_800;
/** Default reveal window: 24 hours. */
export const DEFAULT_REVEAL_WINDOW_SECS = 86_400;

export const ONE_DAY_SECS = 86_400;
export const GRACE_PERIOD_SECS = 5 * ONE_DAY_SECS;
export const SUSPENSION_THRESHOLD_SECS = 6 * ONE_DAY_SECS;
export const LIQUIDATION_THRESHOLD_SECS = 30 * ONE_DAY_SECS;

// ─────────────────────────── Tier enum (matches Anchor IDL) ──────────

export const TIER_VAULT = { vault: {} } as const;
export const TIER_DEFI = { defi: {} } as const;
export type TierIdl = typeof TIER_VAULT | typeof TIER_DEFI;
export type TierName = "vault" | "defi";

/** Numeric value matching the Rust Tier enum (Vault = 0, DeFi = 1). */
export function tierAsU8(tier: TierName): number {
  return tier === "vault" ? 0 : 1;
}

export function tierToIdl(tier: TierName): TierIdl {
  return tier === "vault" ? TIER_VAULT : TIER_DEFI;
}

// ─────────────────────────── KYC level enum ──────────────────────────

export const KYC_NONE = { none: {} } as const;
export const KYC_LIGHT = { light: {} } as const;
export const KYC_FULL = { full: {} } as const;
export type KycLevelIdl =
  | typeof KYC_NONE
  | typeof KYC_LIGHT
  | typeof KYC_FULL;
export type KycLevelName = "none" | "light" | "full";

export function kycLevelToIdl(level: KycLevelName): KycLevelIdl {
  if (level === "none") return KYC_NONE;
  if (level === "light") return KYC_LIGHT;
  return KYC_FULL;
}
