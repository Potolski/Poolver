/**
 * All PDA derivations for Poolver V1.
 *
 * Each helper returns `[address, bump]`. Bumps are returned for diagnostic
 * use; the SDK does NOT pass bumps over the wire — Anchor recovers the
 * canonical bump from the seeds, and the on-chain handlers use the bump
 * stored on each account.
 */
import { PublicKey } from "@solana/web3.js";
import BN from "bn.js";

import {
  BID_SEED,
  BID_STAKE_VAULT_SEED,
  COLLATERAL_VAULT_SEED,
  CORE_INVOKER_SEED,
  DEFI_ADAPTER_KTOKEN_SEED,
  DEFI_ADAPTER_SEED,
  DEFI_ADAPTER_USDC_SEED,
  KYC_SEED,
  PARTICIPANT_SEED,
  POOLVER_CORE_PROGRAM_ID,
  POOLVER_RESERVE_PROGRAM_ID,
  POOLVER_YIELD_DEFI_PROGRAM_ID,
  POOLVER_YIELD_VAULT_PROGRAM_ID,
  POOL_SEED,
  POOL_USDC_VAULT_SEED,
  PROTOCOL_CONFIG_SEED,
  PROTOCOL_FEE_VAULT_SEED,
  REPUTATION_SEED,
  RESERVE_FUND_SEED,
  RESERVE_VAULT_SEED,
  TierName,
  VAULT_ADAPTER_SEED,
  VAULT_ADAPTER_USDC_SEED,
  tierAsU8,
} from "./constants";

type Pda = [PublicKey, number];

function pda(seeds: (Buffer | Uint8Array)[], programId: PublicKey): Pda {
  return PublicKey.findProgramAddressSync(seeds, programId);
}

// ─────────────────────────── Singletons ───────────────────────────────

export function findProtocolConfig(): Pda {
  return pda([PROTOCOL_CONFIG_SEED], POOLVER_CORE_PROGRAM_ID);
}

export function findProtocolFeeVault(): Pda {
  return pda([PROTOCOL_FEE_VAULT_SEED], POOLVER_CORE_PROGRAM_ID);
}

export function findCoreInvoker(): Pda {
  return pda([CORE_INVOKER_SEED], POOLVER_CORE_PROGRAM_ID);
}

// ─────────────────────────── Reserve (per tier) ───────────────────────

export function findReserveFund(tier: TierName): Pda {
  const tierByte = Buffer.from([tierAsU8(tier)]);
  return pda([RESERVE_FUND_SEED, tierByte], POOLVER_RESERVE_PROGRAM_ID);
}

export function findReserveVault(tier: TierName): Pda {
  const tierByte = Buffer.from([tierAsU8(tier)]);
  return pda([RESERVE_VAULT_SEED, tierByte], POOLVER_RESERVE_PROGRAM_ID);
}

// ─────────────────────────── User-scoped ──────────────────────────────

export function findUserReputation(user: PublicKey): Pda {
  return pda([REPUTATION_SEED, user.toBuffer()], POOLVER_CORE_PROGRAM_ID);
}

export function findKycAttestation(user: PublicKey): Pda {
  return pda([KYC_SEED, user.toBuffer()], POOLVER_CORE_PROGRAM_ID);
}

// ─────────────────────────── Pool & members ───────────────────────────

/** `pool_id` is u64; encode as 8 little-endian bytes. */
export function findPool(creator: PublicKey, poolId: BN): Pda {
  const idBytes = Buffer.from(poolId.toArray("le", 8));
  return pda(
    [POOL_SEED, creator.toBuffer(), idBytes],
    POOLVER_CORE_PROGRAM_ID
  );
}

export function findPoolUsdcVault(pool: PublicKey): Pda {
  return pda(
    [POOL_USDC_VAULT_SEED, pool.toBuffer()],
    POOLVER_CORE_PROGRAM_ID
  );
}

export function findCollateralVault(pool: PublicKey): Pda {
  return pda(
    [COLLATERAL_VAULT_SEED, pool.toBuffer()],
    POOLVER_CORE_PROGRAM_ID
  );
}

export function findBidStakeVault(pool: PublicKey): Pda {
  return pda(
    [BID_STAKE_VAULT_SEED, pool.toBuffer()],
    POOLVER_CORE_PROGRAM_ID
  );
}

export function findParticipant(pool: PublicKey, user: PublicKey): Pda {
  return pda(
    [PARTICIPANT_SEED, pool.toBuffer(), user.toBuffer()],
    POOLVER_CORE_PROGRAM_ID
  );
}

/** Per-(pool, month, user) sealed bid. `month` is u8 (1..=12). */
export function findBid(pool: PublicKey, month: number, user: PublicKey): Pda {
  if (month < 1 || month > 12) {
    throw new Error(`bid month out of range (1..=12): ${month}`);
  }
  return pda(
    [BID_SEED, pool.toBuffer(), Buffer.from([month]), user.toBuffer()],
    POOLVER_CORE_PROGRAM_ID
  );
}

// ─────────────────────────── Adapter PDAs ─────────────────────────────

export function findVaultAdapterState(pool: PublicKey): Pda {
  return pda(
    [VAULT_ADAPTER_SEED, pool.toBuffer()],
    POOLVER_YIELD_VAULT_PROGRAM_ID
  );
}

export function findVaultAdapterUsdc(pool: PublicKey): Pda {
  return pda(
    [VAULT_ADAPTER_USDC_SEED, pool.toBuffer()],
    POOLVER_YIELD_VAULT_PROGRAM_ID
  );
}

export function findDefiAdapterState(pool: PublicKey): Pda {
  return pda(
    [DEFI_ADAPTER_SEED, pool.toBuffer()],
    POOLVER_YIELD_DEFI_PROGRAM_ID
  );
}

export function findDefiAdapterUsdc(pool: PublicKey): Pda {
  return pda(
    [DEFI_ADAPTER_USDC_SEED, pool.toBuffer()],
    POOLVER_YIELD_DEFI_PROGRAM_ID
  );
}

export function findDefiAdapterKtoken(pool: PublicKey): Pda {
  return pda(
    [DEFI_ADAPTER_KTOKEN_SEED, pool.toBuffer()],
    POOLVER_YIELD_DEFI_PROGRAM_ID
  );
}

/**
 * Tier-aware dispatch: returns the per-pool adapter state PDA on whichever
 * adapter program owns the given tier.
 */
export function findAdapterState(tier: TierName, pool: PublicKey): Pda {
  return tier === "vault"
    ? findVaultAdapterState(pool)
    : findDefiAdapterState(pool);
}

export function findAdapterUsdc(tier: TierName, pool: PublicKey): Pda {
  return tier === "vault"
    ? findVaultAdapterUsdc(pool)
    : findDefiAdapterUsdc(pool);
}

/** The adapter program ID for a given tier. */
export function adapterProgramId(tier: TierName): PublicKey {
  return tier === "vault"
    ? POOLVER_YIELD_VAULT_PROGRAM_ID
    : POOLVER_YIELD_DEFI_PROGRAM_ID;
}
