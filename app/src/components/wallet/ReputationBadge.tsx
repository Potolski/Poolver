"use client";

import { useEffect, useState } from "react";
import {
  fetchUserReputation,
  repTier,
  type UserReputationView,
} from "@poolver/client";

import { usePoolver } from "@/providers/PoolverProvider";

const POLL_MS = 15_000;

/**
 * Compact reputation tier dot for the connected wallet — shown in
 * TopBar next to the USDC balance. Color encodes trust:
 *   gray   = new (no history)
 *   green  = trusted (completed pools, never defaulted)
 *   yellow = mixed (some completed, some defaulted)
 *   red    = risky (defaulted, no completed history)
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

  const t = repTier(rep);
  const tooltip = rep
    ? `${t.label} · ${t.description}\nJoined: ${rep.poolsJoined ?? 0} · Completed: ${rep.poolsCompleted ?? 0} · Defaulted: ${rep.poolsDefaulted ?? 0}`
    : "Reputation account not initialized";

  return (
    <span className="rep-badge" title={tooltip}>
      <span className="rep-badge-label">REP</span>
      <span
        className="rep-badge-dot"
        style={{
          background: t.color,
          boxShadow: `0 0 6px ${t.color}`,
        }}
        aria-label={t.label}
      />
      <span className="rep-badge-tier" style={{ color: t.color }}>
        {t.label}
      </span>
    </span>
  );
}
