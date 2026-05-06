import { PublicKey } from "@solana/web3.js";
import BN from "bn.js";

import { PoolverClient } from "../poolver";
import { TierName } from "../constants";
import { findReserveFund } from "../pdas";

export interface ReserveFundView {
  publicKey: PublicKey;
  tier: TierName;
  totalBalance: BN;
  totalInflows: BN;
  totalOutflows: BN;
  usdcVault: PublicKey;
  raw: Record<string, unknown>;
}

export async function fetchReserveFund(
  client: PoolverClient,
  tier: TierName
): Promise<ReserveFundView | null> {
  const [pda] = findReserveFund(tier);
  const raw = (await (
    client.reserve.account as any
  ).reserveFund.fetchNullable(pda)) as Record<string, any> | null;
  if (!raw) return null;
  return {
    publicKey: pda,
    tier,
    totalBalance: raw.totalBalance as BN,
    totalInflows: raw.totalInflows as BN,
    totalOutflows: raw.totalOutflows as BN,
    usdcVault: raw.usdcVault as PublicKey,
    raw,
  };
}
