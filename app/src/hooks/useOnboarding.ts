"use client";

import { useCallback, useEffect, useState } from "react";
import {
  fetchUserReputation,
  initializeUserReputationIx,
  type UserReputationView,
} from "@poolver/client";

import { usePoolver } from "@/providers/PoolverProvider";
import { sendIxs } from "@/lib/tx-helpers";

export type OnboardingState =
  | "loading"
  | "disconnected"
  | "needs_reputation"
  | "needs_kyc"
  | "ready";

interface UseOnboardingResult {
  state: OnboardingState;
  reputation: UserReputationView | null;
  refetch: () => Promise<void>;
  ensureReputation: () => Promise<string>;
  ensureKyc: () => Promise<{ signature?: string; idempotent?: boolean }>;
  faucet: (amount?: number) => Promise<{ signature: string; amount: number }>;
}

export function useOnboarding(): UseOnboardingResult {
  const { client, publicKey, connected } = usePoolver();
  const [state, setState] = useState<OnboardingState>("loading");
  const [reputation, setReputation] = useState<UserReputationView | null>(null);

  const load = useCallback(async () => {
    if (!connected || !publicKey) {
      setState("disconnected");
      setReputation(null);
      return;
    }
    setState("loading");
    try {
      const rep = await fetchUserReputation(client, publicKey);
      setReputation(rep);
      if (!rep) {
        setState("needs_reputation");
        return;
      }
      if (rep.kycStatus === "none") {
        setState("needs_kyc");
        return;
      }
      setState("ready");
    } catch {
      setState("needs_reputation");
      setReputation(null);
    }
  }, [client, connected, publicKey]);

  useEffect(() => {
    void load();
  }, [load]);

  const ensureReputation = useCallback(async (): Promise<string> => {
    if (!connected || !publicKey) {
      throw new Error("wallet not connected");
    }
    const ix = await initializeUserReputationIx(client);
    const sig = await sendIxs(client, [ix]);
    await load();
    return sig;
  }, [client, connected, publicKey, load]);

  const ensureKyc = useCallback(async (): Promise<{
    signature?: string;
    idempotent?: boolean;
  }> => {
    if (!publicKey) throw new Error("wallet not connected");
    const res = await fetch("/api/kyc/issue", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ user: publicKey.toBase58(), level: "full" }),
    });
    const data = (await res.json()) as {
      signature?: string;
      idempotent?: boolean;
      error?: string;
      message?: string;
    };
    if (!res.ok) {
      throw new Error(data.message ?? data.error ?? "kyc_failed");
    }
    await load();
    return { signature: data.signature, idempotent: data.idempotent };
  }, [publicKey, load]);

  const faucet = useCallback(
    async (amount = 5_000): Promise<{ signature: string; amount: number }> => {
      if (!publicKey) throw new Error("wallet not connected");
      const res = await fetch("/api/faucet", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ recipient: publicKey.toBase58(), amount }),
      });
      const data = (await res.json()) as {
        signature?: string;
        amount?: number;
        error?: string;
        message?: string;
      };
      if (!res.ok || !data.signature) {
        throw new Error(data.message ?? data.error ?? "faucet_failed");
      }
      return { signature: data.signature, amount: data.amount ?? amount };
    },
    [publicKey]
  );

  return {
    state,
    reputation,
    refetch: load,
    ensureReputation,
    ensureKyc,
    faucet,
  };
}
