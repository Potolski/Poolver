"use client";

import { useCallback, useEffect, useState } from "react";
import { PublicKey } from "@solana/web3.js";
import {
  computeMonthState,
  fetchParticipant,
  fetchPool,
  type ParticipantView,
  type PoolMonthState,
  type PoolView,
} from "@poolver/client";

import { usePoolver } from "@/providers/PoolverProvider";

interface UsePoolResult {
  pool: PoolView | null;
  participant: ParticipantView | null;
  monthState: PoolMonthState | null;
  loading: boolean;
  error: Error | null;
  refetch: () => Promise<void>;
}

export function usePool(address: string | undefined): UsePoolResult {
  const { client, publicKey } = usePoolver();
  const [pool, setPool] = useState<PoolView | null>(null);
  const [participant, setParticipant] = useState<ParticipantView | null>(null);
  const [monthState, setMonthState] = useState<PoolMonthState | null>(null);
  const [loading, setLoading] = useState<boolean>(Boolean(address));
  const [error, setError] = useState<Error | null>(null);

  // `silent` skips the loading-spinner state, so background polls don't
  // make the whole page flash through its `if (loading) return …` branch
  // every 8 seconds. Initial load + manual refetch still flip loading
  // for the user-visible spinner.
  const load = useCallback(
    async (silent = false) => {
      if (!address) {
        setPool(null);
        setParticipant(null);
        setMonthState(null);
        setLoading(false);
        return;
      }
      let pubkey: PublicKey;
      try {
        pubkey = new PublicKey(address);
      } catch (err) {
        setError(err instanceof Error ? err : new Error("invalid pool address"));
        setLoading(false);
        return;
      }

      if (!silent) setLoading(true);
      setError(null);
      try {
        const [poolView, participantView] = await Promise.all([
          fetchPool(client, pubkey),
          publicKey
            ? fetchParticipant(client, pubkey, publicKey)
            : Promise.resolve(null),
        ]);
        setPool(poolView);
        setParticipant(participantView);
        setMonthState(
          poolView ? computeMonthState(poolView, Math.floor(Date.now() / 1000)) : null
        );
      } catch (err) {
        setError(err instanceof Error ? err : new Error("failed to load pool"));
      } finally {
        if (!silent) setLoading(false);
      }
    },
    [address, client, publicKey]
  );

  useEffect(() => {
    void load(false); // initial load — show spinner
    const id = setInterval(() => {
      void load(true); // background poll — silent
    }, 8_000);
    return () => clearInterval(id);
  }, [load]);

  // Public refetch (called after on-chain actions) shows the spinner.
  const refetch = useCallback(() => load(false), [load]);

  return { pool, participant, monthState, loading, error, refetch };
}
