"use client";

import { useEffect, useState } from "react";
import {
  fetchUserReputation,
  type UserReputationView,
} from "@poolver/client";

import { usePoolver } from "@/providers/PoolverProvider";

const POLL_MS = 15_000;

/**
 * Compact reputation badge for the connected wallet — shown in TopBar
 * next to the USDC balance. Format: "REP · J·C·D" where J = pools
 * joined, C = pools completed, D = pools defaulted. Tooltip expands.
 */
export function ReputationBadge() {
  const { client, connected, publicKey } = usePoolver();
  const [rep, setRep] = useState<UserReputationView | null>(null);

  useEffect(() => {
    if (!connected || !publicKey) {
      setRep(null);
      return;
    }
    let cancelled = false;
    const tick = async () => {
      try {
        const r = await fetchUserReputation(client, publicKey);
        if (!cancelled) setRep(r);
      } catch {
        // network errors: leave as-is
      }
    };
    void tick();
    const id = setInterval(tick, POLL_MS);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, [client, connected, publicKey]);

  if (!connected || !publicKey) return null;
  if (!rep) {
    return (
      <span className="rep-badge" title="Reputation not initialized yet">
        <span className="rep-badge-label">REP</span>
        <span className="rep-badge-value">—</span>
      </span>
    );
  }

  const completed = rep.poolsCompleted ?? 0;
  const defaulted = rep.poolsDefaulted ?? 0;
  const joined = rep.poolsJoined ?? 0;

  const tooltip =
    `Joined: ${joined} pool${joined === 1 ? "" : "s"}\n` +
    `Completed: ${completed}\n` +
    `Defaulted: ${defaulted}\n` +
    `Lifetime contributed: ${(Number(rep.totalContributedLifetime ?? 0) / 1e6).toLocaleString()} USDC\n` +
    `Lifetime received: ${(Number(rep.totalReceivedLifetime ?? 0) / 1e6).toLocaleString()} USDC`;

  return (
    <span className="rep-badge" title={tooltip}>
      <span className="rep-badge-label">REP</span>
      <span className="rep-badge-value">
        <span style={{ color: "var(--fg)" }}>{joined}</span>
        <span style={{ color: "var(--fg-4)" }}>·</span>
        <span style={{ color: "var(--ok, var(--acc))" }}>{completed}</span>
        <span style={{ color: "var(--fg-4)" }}>·</span>
        <span style={{ color: defaulted > 0 ? "var(--err)" : "var(--fg-4)" }}>
          {defaulted}
        </span>
      </span>
    </span>
  );
}
