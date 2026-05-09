"use client";

import Link from "next/link";
import type { PoolView } from "@poolver/client";
import { microUsdcToHuman } from "@poolver/client";
import BN from "bn.js";

import { PoolverMark } from "@/components/brand/PoolverLogo";
import { fmtUSD, fmtCountdown } from "@/lib/format";
import { derivePoolStatus } from "@/lib/types";

const STATUS_STYLES = {
  forming: { label: "◐ FORMING", color: "var(--warn)", bg: "oklch(0.3 0.1 75)" },
  active: { label: "● ACTIVE", color: "var(--acc)", bg: "var(--acc-tint)" },
  completed: { label: "◉ COMPLETE", color: "var(--fg-2)", bg: "var(--bg-3)" },
} as const;

function StatusChip({ status }: { status: keyof typeof STATUS_STYLES }) {
  const m = STATUS_STYLES[status];
  return (
    <span
      style={{
        display: "inline-flex",
        alignItems: "center",
        fontFamily: "var(--mono)",
        fontSize: 9.5,
        letterSpacing: "0.14em",
        padding: "3px 7px",
        border: `1px solid ${m.color}`,
        color: m.color,
        background: m.bg,
        borderRadius: 2,
      }}
    >
      {m.label}
    </span>
  );
}

interface PoolCardProps {
  pool: PoolView;
  featured?: boolean;
}

export function PoolCard({ pool, featured }: PoolCardProps) {
  const status = derivePoolStatus(pool);
  const monthlyHuman = Number(microUsdcToHuman(pool.contributionAmount));
  const lifetimeMicro = pool.contributionAmount.muln(pool.totalMonths);
  const lifetimeHuman = Number(microUsdcToHuman(lifetimeMicro));

  const isForming = status === "forming";
  const isComplete = status === "completed";
  // Clamp `currentMonth` for display — on-chain it ticks past totalMonths
  // when the pool advances out of month 12 into is_complete.
  const displayMonth = Math.min(pool.currentMonth, pool.totalMonths);
  const fill =
    isForming
      ? pool.participantCount / 12
      : pool.totalMonths > 0
        ? displayMonth / pool.totalMonths
        : 0;
  const barColor = isForming ? "var(--warn)" : "var(--acc)";

  const now = Math.floor(Date.now() / 1000);
  const monthEnded = pool.currentMonthStartedAt.add(pool.monthDurationSeconds);
  const secsLeft = monthEnded.sub(new BN(now)).toNumber();
  const nextDraw =
    status === "active" && Number.isFinite(secsLeft)
      ? fmtCountdown(secsLeft)
      : isForming
        ? "—"
        : "settled";

  const addr = pool.publicKey.toBase58();
  const idShort = `PLVR-${addr.slice(0, 4).toUpperCase()}`;

  return (
    <Link
      href={`/pool/${addr}`}
      className={`pool-card ${featured ? "featured" : ""}`}
    >
      {featured && <div className="pool-featured-badge">YOUR POSITION</div>}
      <div className="pool-card-head">
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <PoolverMark size={18} />
          <div>
            <div
              style={{
                fontFamily: "var(--mono)",
                fontSize: 14,
                color: "var(--fg)",
                fontWeight: 500,
              }}
            >
              {idShort}
            </div>
            <div
              style={{
                fontSize: 10,
                color: "var(--fg-4)",
                letterSpacing: "0.1em",
                textTransform: "uppercase",
                marginTop: 2,
              }}
            >
              Solana · USDC · Tier {pool.tier === "vault" ? "0 · Vault" : "1 · Kamino"}
            </div>
          </div>
        </div>
        <StatusChip status={status} />
      </div>

      <div className="pool-hero">
        <div>
          <div
            style={{
              fontSize: 9.5,
              color: "var(--fg-4)",
              letterSpacing: "0.14em",
              textTransform: "uppercase",
            }}
          >
            Lifetime pool
          </div>
          <div
            style={{
              fontFamily: "var(--display)",
              fontSize: 32,
              lineHeight: 1,
              letterSpacing: "-0.02em",
              color: "var(--fg)",
              marginTop: 4,
              fontVariantNumeric: "tabular-nums",
            }}
          >
            {fmtUSD(lifetimeHuman)}
          </div>
        </div>
        <div style={{ textAlign: "right" }}>
          <div
            style={{
              fontSize: 9.5,
              color: "var(--fg-4)",
              letterSpacing: "0.14em",
              textTransform: "uppercase",
            }}
          >
            Monthly
          </div>
          <div
            style={{
              fontFamily: "var(--display)",
              fontSize: 18,
              color: "var(--acc)",
              marginTop: 4,
              fontVariantNumeric: "tabular-nums",
              letterSpacing: "-0.01em",
            }}
          >
            {fmtUSD(monthlyHuman)}
          </div>
        </div>
      </div>

      <div style={{ marginTop: 14 }}>
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            fontSize: 10,
            color: "var(--fg-3)",
            fontFamily: "var(--mono)",
            letterSpacing: "0.08em",
            marginBottom: 6,
          }}
        >
          <span>
            {isForming
              ? `FILLING ${pool.participantCount}/12`
              : isComplete
                ? `COMPLETE · ${pool.totalMonths}/${pool.totalMonths}`
                : `MONTH ${String(displayMonth).padStart(2, "0")} / ${pool.totalMonths}`}
          </span>
          <span style={{ color: "var(--fg-4)" }}>
            {Math.round(fill * 100)}%
          </span>
        </div>
        <div
          style={{
            height: 3,
            background: "var(--bg-3)",
            borderRadius: 2,
            overflow: "hidden",
          }}
        >
          <div
            style={{
              height: "100%",
              width: `${Math.min(100, fill * 100)}%`,
              background: barColor,
              boxShadow: `0 0 8px ${barColor}`,
            }}
          />
        </div>
      </div>

      <div className="pool-kv">
        <div className="pool-kv-row">
          <span>Next draw</span>
          <span
            className="v"
            style={{ color: status === "active" ? "var(--acc)" : "var(--fg-3)" }}
          >
            {nextDraw}
          </span>
        </div>
        <div className="pool-kv-row">
          <span>Tier</span>
          <span className="v">
            {pool.tier === "vault" ? "Vault (0)" : "DeFi (1)"}
          </span>
        </div>
        <div className="pool-kv-row">
          <span>Contributed</span>
          <span className="v">
            {fmtUSD(Number(microUsdcToHuman(pool.totalContributed)))}
          </span>
        </div>
        <div className="pool-kv-row">
          <span>Distributed</span>
          <span className="v">
            {fmtUSD(Number(microUsdcToHuman(pool.totalDistributed)))}
          </span>
        </div>
      </div>

      <div className="pool-card-foot">
        <span
          style={{
            fontSize: 10,
            color: "var(--fg-4)",
            fontFamily: "var(--mono)",
            letterSpacing: "0.1em",
          }}
        >
          {isForming
            ? `OPENS @ 12/12`
            : isComplete
              ? `Pool complete`
              : `${pool.totalMonths - displayMonth} months remaining`}
        </span>
        <span
          style={{
            fontSize: 11,
            color: "var(--acc)",
            fontFamily: "var(--mono)",
            letterSpacing: "0.1em",
          }}
        >
          {featured ? "OPEN →" : isForming ? "JOIN →" : "VIEW →"}
        </span>
      </div>
    </Link>
  );
}
