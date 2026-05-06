import { PublicKey } from "@solana/web3.js";
import BN from "bn.js";

import { PoolverClient } from "../poolver";
import { findParticipant } from "../pdas";

export interface ParticipantView {
  publicKey: PublicKey;
  pool: PublicKey;
  user: PublicKey;
  paidMonthsBitmap: number;
  isDefaulted: boolean;
  isSuspended: boolean;
  hasWon: boolean;
  winMonth: number;
  collateralPosted: BN;
  raw: Record<string, unknown>;
}

export async function fetchParticipant(
  client: PoolverClient,
  pool: PublicKey,
  user: PublicKey
): Promise<ParticipantView | null> {
  const [pda] = findParticipant(pool, user);
  const raw = (await (client.core.account as any).participant.fetchNullable(
    pda
  )) as Record<string, any> | null;
  if (!raw) return null;
  return {
    publicKey: pda,
    pool: raw.pool as PublicKey,
    user: raw.user as PublicKey,
    paidMonthsBitmap: raw.paidMonths as number,
    isDefaulted: raw.isDefaulted as boolean,
    isSuspended: raw.isSuspended as boolean,
    hasWon: raw.hasWon as boolean,
    winMonth: (raw.winMonth as number) ?? 0,
    collateralPosted: (raw.collateralPosted as BN) ?? new BN(0),
    raw,
  };
}

/** True iff the participant has marked the given month as paid. */
export function hasPaidMonth(
  participant: ParticipantView,
  month: number
): boolean {
  if (month < 1 || month > 16) return false;
  return (participant.paidMonthsBitmap & (1 << (month - 1))) !== 0;
}
