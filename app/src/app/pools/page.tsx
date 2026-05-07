"use client";

import Link from "next/link";
import { useMemo, useState } from "react";
import BN from "bn.js";
import { microUsdcToHuman, type PoolView, type TierName } from "@poolver/client";

import { PoolverMark } from "@/components/brand/PoolverLogo";
import { PoolCard } from "@/components/pools/PoolCard";
import { usePools } from "@/hooks/usePools";
import { fmtUSD } from "@/lib/format";
import { derivePoolStatus, type PoolDisplayStatus } from "@/lib/types";

type StatusFilter = "all" | PoolDisplayStatus;
type TierFilter = "all" | TierName;
type Sort = "size" | "monthly" | "soon";

export default function PoolsPage() {
  const { pools, loading, error, refetch } = usePools();
  const [statusFilter, setStatusFilter] = useState<StatusFilter>("all");
  const [tierFilter, setTierFilter] = useState<TierFilter>("all");
  const [sort, setSort] = useState<Sort>("size");

  const visible = useMemo(() => {
    const now = Math.floor(Date.now() / 1000);
    const filtered = pools.filter((p) => {
      if (statusFilter !== "all" && derivePoolStatus(p) !== statusFilter)
        return false;
      if (tierFilter !== "all" && p.tier !== tierFilter) return false;
      return true;
    });
    return [...filtered].sort((a, b) => {
      if (sort === "size") {
        const aSize = a.contributionAmount.muln(a.totalMonths);
        const bSize = b.contributionAmount.muln(b.totalMonths);
        return bSize.cmp(aSize);
      }
      if (sort === "monthly") return a.contributionAmount.cmp(b.contributionAmount);
      if (sort === "soon") {
        const aLeft = a.currentMonthStartedAt
          .add(a.monthDurationSeconds)
          .sub(new BN(now))
          .toNumber();
        const bLeft = b.currentMonthStartedAt
          .add(b.monthDurationSeconds)
          .sub(new BN(now))
          .toNumber();
        return aLeft - bLeft;
      }
      return 0;
    });
  }, [pools, statusFilter, tierFilter, sort]);

  const counts = useMemo(() => {
    const all = pools.length;
    const forming = pools.filter((p) => derivePoolStatus(p) === "forming").length;
    const active = pools.filter((p) => derivePoolStatus(p) === "active").length;
    const completed = pools.filter((p) => derivePoolStatus(p) === "completed").length;
    return { all, forming, active, completed };
  }, [pools]);

  const tvlMicro = pools.reduce(
    (s, p) => s.add(p.totalContributed),
    new BN(0)
  );
  const tvl = Number(microUsdcToHuman(tvlMicro));
  const wallets = pools.reduce((s, p) => s + p.participantCount, 0);

  return (
    <>
      <section
        style={{ borderBottom: "1px solid var(--line)", padding: "48px 0 32px" }}
      >
        <div className="shell pools-hero-grid">
          <div>
            <div className="hero-kicker">
              <span className="sq" />
              ALL POOLS · LIVE INDEX
            </div>
            <h1
              className="hero-headline"
              style={{
                fontSize: "clamp(36px, 4.4vw, 60px)",
                margin: "16px 0 14px",
              }}
            >
              Concurrent <em>Poolvers</em>.<br />Pick your ticket.
            </h1>
            <p className="hero-deck" style={{ maxWidth: "56ch" }}>
              Every pool is an isolated PDA on `poolver-core`. Tier-0 (Vault)
              and Tier-1 (DeFi) adapters route the same instruction surface to
              different yield strategies.
            </p>
          </div>
          <div className="stats" style={{ gridTemplateColumns: "1fr 1fr" }}>
            <div className="stat" style={{ padding: 18 }}>
              <div className="lbl">
                <PoolverMark size={11} /> Total contributed
              </div>
              <div className="v" style={{ fontSize: 32, margin: "10px 0 6px" }}>
                {fmtUSD(tvl)}
              </div>
              <div className="sub">Across {pools.length} pools</div>
            </div>
            <div className="stat" style={{ padding: 18 }}>
              <div className="lbl">
                <PoolverMark size={11} /> Member slots filled
              </div>
              <div className="v" style={{ fontSize: 32, margin: "10px 0 6px" }}>
                {wallets}
              </div>
              <div className="sub">Devnet · USDC test mint</div>
            </div>
          </div>
        </div>
      </section>

      <section
        className="shell"
        style={{
          padding: "24px 0 12px",
          display: "flex",
          gap: 16,
          flexWrap: "wrap",
          alignItems: "center",
          justifyContent: "space-between",
        }}
      >
        <div style={{ display: "flex", gap: 12, flexWrap: "wrap" }}>
          <div
            style={{
              display: "flex",
              gap: 4,
              padding: 3,
              background: "var(--bg-2)",
              border: "1px solid var(--line)",
              borderRadius: 2,
            }}
          >
            {(
              [
                ["all", "ALL", counts.all],
                ["active", "ACTIVE", counts.active],
                ["forming", "FORMING", counts.forming],
                ["completed", "COMPLETE", counts.completed],
              ] as const
            ).map(([k, lbl, n]) => (
              <button
                key={k}
                onClick={() => setStatusFilter(k as StatusFilter)}
                style={{
                  padding: "5px 12px",
                  fontSize: 10,
                  borderRadius: 2,
                  border: 0,
                  cursor: "pointer",
                  fontFamily: "var(--mono)",
                  letterSpacing: "0.08em",
                  background: statusFilter === k ? "var(--bg)" : "transparent",
                  boxShadow:
                    statusFilter === k ? "inset 0 0 0 1px var(--line-2)" : "none",
                  color: statusFilter === k ? "var(--acc)" : "var(--fg-3)",
                }}
              >
                {lbl}{" "}
                <span style={{ color: "var(--fg-4)", marginLeft: 4 }}>{n}</span>
              </button>
            ))}
          </div>
          <div
            style={{
              display: "flex",
              gap: 4,
              padding: 3,
              background: "var(--bg-2)",
              border: "1px solid var(--line)",
              borderRadius: 2,
            }}
          >
            {(
              [
                ["all", "ALL TIERS"],
                ["vault", "VAULT"],
                ["defi", "DEFI"],
              ] as const
            ).map(([k, lbl]) => (
              <button
                key={k}
                onClick={() => setTierFilter(k as TierFilter)}
                style={{
                  padding: "5px 12px",
                  fontSize: 10,
                  borderRadius: 2,
                  border: 0,
                  cursor: "pointer",
                  fontFamily: "var(--mono)",
                  letterSpacing: "0.08em",
                  background: tierFilter === k ? "var(--bg)" : "transparent",
                  boxShadow:
                    tierFilter === k ? "inset 0 0 0 1px var(--line-2)" : "none",
                  color: tierFilter === k ? "var(--acc)" : "var(--fg-3)",
                }}
              >
                {lbl}
              </button>
            ))}
          </div>
        </div>
        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <span
            style={{
              fontSize: 10,
              color: "var(--fg-4)",
              letterSpacing: "0.14em",
              textTransform: "uppercase",
            }}
          >
            Sort
          </span>
          <select
            value={sort}
            onChange={(e) => setSort(e.target.value as Sort)}
            className="pool-select"
          >
            <option value="size">Lifetime size</option>
            <option value="monthly">Monthly (low → high)</option>
            <option value="soon">Next deadline (soonest)</option>
          </select>
        </div>
      </section>

      <section className="shell" style={{ padding: "12px 0 48px" }}>
        {loading ? (
          <EmptyState label="Loading pools from devnet…" />
        ) : error ? (
          <ErrorState message={error.message} onRetry={refetch} />
        ) : visible.length === 0 ? (
          <EmptyPoolsState total={pools.length} />
        ) : (
          <div className="pools-grid">
            {visible.map((p: PoolView) => (
              <PoolCard key={p.publicKey.toBase58()} pool={p} />
            ))}
          </div>
        )}
      </section>
    </>
  );
}

function EmptyState({ label }: { label: string }) {
  return (
    <div
      style={{
        padding: "64px 16px",
        textAlign: "center",
        color: "var(--fg-3)",
        fontFamily: "var(--mono)",
        fontSize: 12,
        letterSpacing: "0.1em",
        border: "1px dashed var(--line)",
        borderRadius: 2,
      }}
    >
      {label}
    </div>
  );
}

function ErrorState({
  message,
  onRetry,
}: {
  message: string;
  onRetry: () => void;
}) {
  return (
    <div
      style={{
        padding: "48px 16px",
        textAlign: "center",
        border: "1px solid var(--err)",
        borderRadius: 2,
        color: "var(--err)",
      }}
    >
      <div style={{ fontFamily: "var(--mono)", fontSize: 11, marginBottom: 8 }}>
        RPC error
      </div>
      <div style={{ color: "var(--fg-2)", fontSize: 13, marginBottom: 16 }}>
        {message}
      </div>
      <button className="btn sm" onClick={onRetry}>
        Retry
      </button>
    </div>
  );
}

function EmptyPoolsState({ total }: { total: number }) {
  const none = total === 0;
  return (
    <div
      style={{
        padding: "64px 16px",
        textAlign: "center",
        border: "1px dashed var(--line)",
        borderRadius: 2,
      }}
    >
      <div
        style={{
          fontFamily: "var(--mono)",
          fontSize: 11,
          letterSpacing: "0.14em",
          color: "var(--fg-4)",
          marginBottom: 8,
        }}
      >
        {none ? "NO POOLS ON DEVNET YET" : "NO POOLS MATCH FILTER"}
      </div>
      <div style={{ color: "var(--fg-2)", fontSize: 14, marginBottom: 20 }}>
        {none
          ? "Be the first to create a Poolver on the deployed program."
          : "Try a different filter or create a new pool."}
      </div>
      <Link href="/create" className="btn primary">
        + Create a pool
      </Link>
    </div>
  );
}
