"use client";

import { useCallback, useEffect, useState } from "react";
import BN from "bn.js";
import { PublicKey } from "@solana/web3.js";
import { countFilledParticipants, type PoolView, type TierName } from "@poolver/client";

import { usePoolver } from "@/providers/PoolverProvider";

interface UsePoolsResult {
  pools: PoolView[];
  loading: boolean;
  error: Error | null;
  refetch: () => Promise<void>;
}

function decodeTier(idlVariant: { vault?: object; defi?: object }): TierName {
  if ("vault" in idlVariant && idlVariant.vault) return "vault";
  if ("defi" in idlVariant && idlVariant.defi) return "defi";
  throw new Error("unrecognized tier variant on Pool account");
}

function decodePool(publicKey: PublicKey, raw: Record<string, unknown>): PoolView {
  const r = raw as Record<string, unknown> & {
    tier: { vault?: object; defi?: object };
  };
  return {
    publicKey,
    poolId: r.poolId as BN,
    creator: r.creator as PublicKey,
    tier: decodeTier(r.tier),
    contributionAmount: r.contributionAmount as BN,
    participantCount: countFilledParticipants(r.participants),
    maxParticipants: r.participantCount as number,
    totalMonths: r.totalMonths as number,
    currentMonth: r.currentMonth as number,
    startTimestamp: r.startTimestamp as BN,
    monthDurationSeconds: r.monthDurationSeconds as BN,
    bidWindowSeconds: r.bidWindowSeconds as BN,
    currentMonthStartedAt: r.currentMonthStartedAt as BN,
    bidWindowEndsAt: r.bidWindowEndsAt as BN,
    revealWindowEndsAt: r.revealWindowEndsAt as BN,
    totalContributed: r.totalContributed as BN,
    totalDistributed: r.totalDistributed as BN,
    totalCollateralLocked: r.totalCollateralLocked as BN,
    bidCreditBalance: r.bidCreditBalance as BN,
    isComplete: r.isComplete as boolean,
    vrfInFlight: r.vrfInFlight as boolean,
    vrfAccount: r.vrfAccount as PublicKey,
    poolUsdcVault: r.poolUsdcVault as PublicKey,
    collateralVault: r.collateralVault as PublicKey,
    adapterState: r.adapterState as PublicKey,
    bump: r.bump as number,
    version: r.version as number,
    completedAt: r.completedAt as BN,
    paidCountForCurrentMonth: r.paidCountForCurrentMonth as number,
    raw: r,
  };
}

export function usePools(): UsePoolsResult {
  const { client } = usePoolver();
  const [pools, setPools] = useState<PoolView[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<Error | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const accountClient = (
        client.core.account as unknown as {
          pool: {
            all: () => Promise<
              Array<{ publicKey: PublicKey; account: Record<string, unknown> }>
            >;
          };
        }
      ).pool;
      const accounts = await accountClient.all();
      const decoded = accounts.map(({ publicKey, account }) =>
        decodePool(publicKey, account)
      );
      setPools(decoded);
    } catch (err) {
      setError(err instanceof Error ? err : new Error("failed to list pools"));
      setPools([]);
    } finally {
      setLoading(false);
    }
  }, [client]);

  useEffect(() => {
    void load();
  }, [load]);

  return { pools, loading, error, refetch: load };
}
