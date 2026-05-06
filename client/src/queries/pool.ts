import { PublicKey } from "@solana/web3.js";
import BN from "bn.js";

import { PoolverClient } from "../poolver";
import { findPool } from "../pdas";
import { TierName } from "../constants";

/**
 * Decoded `Pool` account. Field names match the Rust state struct
 * (camelCased by Anchor codegen). Only the fields needed for SDK
 * derivations are typed strictly here; everything else passes through
 * as `any`.
 */
export interface PoolView {
  publicKey: PublicKey;
  poolId: BN;
  creator: PublicKey;
  tier: TierName;
  contributionAmount: BN;
  participantCount: number;
  totalMonths: number;
  currentMonth: number;
  startTimestamp: BN;
  monthDurationSeconds: BN;
  bidWindowSeconds: BN;
  currentMonthStartedAt: BN;
  bidWindowEndsAt: BN;
  revealWindowEndsAt: BN;
  totalContributed: BN;
  totalDistributed: BN;
  totalCollateralLocked: BN;
  bidCreditBalance: BN;
  isComplete: boolean;
  vrfInFlight: boolean;
  vrfAccount: PublicKey;
  poolUsdcVault: PublicKey;
  collateralVault: PublicKey;
  adapterState: PublicKey;
  bump: number;
  version: number;
  completedAt: BN;
  paidCountForCurrentMonth: number;
  /** Untyped passthrough for fields not enumerated above. */
  raw: Record<string, unknown>;
}

function decodeTier(idlVariant: { vault?: object; defi?: object }): TierName {
  if ("vault" in idlVariant && idlVariant.vault) return "vault";
  if ("defi" in idlVariant && idlVariant.defi) return "defi";
  throw new Error("unrecognized tier variant from on-chain Pool account");
}

/** Fetch a Pool by its address. */
export async function fetchPool(
  client: PoolverClient,
  pool: PublicKey
): Promise<PoolView | null> {
  // The IDL is generic; we cast through `any` here because the SDK ships
  // with a generic Idl-typed Program (see PoolverClient).
  const raw = (await (client.core.account as any).pool.fetchNullable(
    pool
  )) as Record<string, any> | null;
  if (!raw) return null;
  return {
    publicKey: pool,
    poolId: raw.poolId as BN,
    creator: raw.creator as PublicKey,
    tier: decodeTier(raw.tier),
    contributionAmount: raw.contributionAmount as BN,
    participantCount: raw.participantCount as number,
    totalMonths: raw.totalMonths as number,
    currentMonth: raw.currentMonth as number,
    startTimestamp: raw.startTimestamp as BN,
    monthDurationSeconds: raw.monthDurationSeconds as BN,
    bidWindowSeconds: raw.bidWindowSeconds as BN,
    currentMonthStartedAt: raw.currentMonthStartedAt as BN,
    bidWindowEndsAt: raw.bidWindowEndsAt as BN,
    revealWindowEndsAt: raw.revealWindowEndsAt as BN,
    totalContributed: raw.totalContributed as BN,
    totalDistributed: raw.totalDistributed as BN,
    totalCollateralLocked: raw.totalCollateralLocked as BN,
    bidCreditBalance: raw.bidCreditBalance as BN,
    isComplete: raw.isComplete as boolean,
    vrfInFlight: raw.vrfInFlight as boolean,
    vrfAccount: raw.vrfAccount as PublicKey,
    poolUsdcVault: raw.poolUsdcVault as PublicKey,
    collateralVault: raw.collateralVault as PublicKey,
    adapterState: raw.adapterState as PublicKey,
    bump: raw.bump as number,
    version: raw.version as number,
    completedAt: raw.completedAt as BN,
    paidCountForCurrentMonth: raw.paidCountForCurrentMonth as number,
    raw,
  };
}

/** Fetch a Pool by `(creator, poolId)`. */
export async function fetchPoolByCreatorAndId(
  client: PoolverClient,
  creator: PublicKey,
  poolId: BN
): Promise<PoolView | null> {
  const [pool] = findPool(creator, poolId);
  return fetchPool(client, pool);
}

/**
 * Compute the current month state from a Pool snapshot. Useful for UI
 * gating ("can I bid right now?", "is reveal window open?").
 */
export interface PoolMonthState {
  currentMonth: number;
  inBidWindow: boolean;
  inRevealWindow: boolean;
  monthEndedAt: BN;
  /** Seconds until `month_end`; negative when overdue. */
  secondsUntilMonthEnd: number;
}

export function computeMonthState(
  pool: PoolView,
  nowUnixSecs: number
): PoolMonthState {
  const now = new BN(nowUnixSecs);
  const inBidWindow =
    pool.bidWindowEndsAt.gtn(0) && now.lt(pool.bidWindowEndsAt);
  const inRevealWindow =
    !inBidWindow &&
    pool.revealWindowEndsAt.gtn(0) &&
    now.lt(pool.revealWindowEndsAt);
  const monthEndedAt = pool.currentMonthStartedAt.add(pool.monthDurationSeconds);
  return {
    currentMonth: pool.currentMonth,
    inBidWindow,
    inRevealWindow,
    monthEndedAt,
    secondsUntilMonthEnd: monthEndedAt.sub(now).toNumber(),
  };
}
