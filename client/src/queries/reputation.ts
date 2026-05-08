import { PublicKey } from "@solana/web3.js";
import BN from "bn.js";

import { PoolverClient } from "../poolver";
import { findUserReputation } from "../pdas";
import { KycLevelName } from "../constants";

export interface UserReputationView {
  publicKey: PublicKey;
  user: PublicKey;
  poolsJoined: number;
  poolsCompleted: number;
  poolsDefaulted: number;
  totalContributedLifetime: BN;
  totalReceivedLifetime: BN;
  kycStatus: KycLevelName;
  kycAttestation: PublicKey;
  lastKycAt: BN;
  /** Lifetime count of (pool, month) slashes for missing a contribution. */
  monthsMissedLifetime: number;
  raw: Record<string, unknown>;
}

function decodeKycStatus(byte: number): KycLevelName {
  switch (byte) {
    case 0:
      return "none";
    case 1:
      return "light";
    case 2:
      return "full";
    default:
      throw new Error(`unrecognized kyc status byte: ${byte}`);
  }
}

export async function fetchUserReputation(
  client: PoolverClient,
  user: PublicKey
): Promise<UserReputationView | null> {
  const [pda] = findUserReputation(user);
  const raw = (await (
    client.core.account as any
  ).userReputation.fetchNullable(pda)) as Record<string, any> | null;
  if (!raw) return null;
  return {
    publicKey: pda,
    user: raw.user as PublicKey,
    poolsJoined: raw.poolsJoined as number,
    poolsCompleted: raw.poolsCompleted as number,
    poolsDefaulted: raw.poolsDefaulted as number,
    totalContributedLifetime: raw.totalContributedLifetime as BN,
    totalReceivedLifetime: raw.totalReceivedLifetime as BN,
    kycStatus: decodeKycStatus(raw.kycStatus as number),
    kycAttestation: raw.kycAttestation as PublicKey,
    lastKycAt: raw.lastKycAt as BN,
    monthsMissedLifetime: (raw.monthsMissedLifetime as number) ?? 0,
    raw,
  };
}
