"use client";

import Link from "next/link";
import { useParams, useRouter } from "next/navigation";
import { microUsdcToHuman } from "@poolver/client";

import { PoolverMark } from "@/components/brand/PoolverLogo";
import { LotterySection } from "@/components/lottery/LotterySection";
import { MonthTimeline } from "@/components/pools/MonthTimeline";
import { ParticipantRoster } from "@/components/pools/ParticipantRoster";
import { PoolActions } from "@/components/pools/PoolActions";
import { ReserveStats } from "@/components/pools/ReserveStats";
import { usePool } from "@/hooks/usePool";
import { fmtCountdown } from "@/lib/format";
import { derivePoolStatus } from "@/lib/types";
import { truncateAddress } from "@/lib/utils";

export default function PoolPage() {
  const { address } = useParams<{ address: string }>();
  const router = useRouter();
  const { pool, participant, monthState, loading, error, refetch } =
    usePool(address);

  if (loading) {
    return (
      <section className="shell section" style={{ textAlign: "center" }}>
        <div
          style={{
            padding: "96px 16px",
            color: "var(--fg-3)",
            fontFamily: "var(--mono)",
            fontSize: 12,
            letterSpacing: "0.1em",
          }}
        >
          Loading pool from devnet…
        </div>
      </section>
    );
  }

  if (error || !pool) {
    return (
      <section className="shell section" style={{ textAlign: "center" }}>
        <div style={{ padding: "80px 16px" }}>
          <div
            style={{
              fontFamily: "var(--mono)",
              fontSize: 11,
              letterSpacing: "0.14em",
              color: "var(--fg-4)",
              marginBottom: 8,
            }}
          >
            POOL NOT FOUND
          </div>
          <div style={{ color: "var(--fg-2)", fontSize: 14, marginBottom: 20 }}>
            {error
              ? error.message
              : "This address does not match a Poolver V1 pool on devnet."}
          </div>
          <Link href="/pools" className="btn">
            ← All pools
          </Link>
        </div>
      </section>
    );
  }

  const status = derivePoolStatus(pool);
  const monthlyHuman = Number(microUsdcToHuman(pool.contributionAmount));
  const lifetimeHuman = monthlyHuman * pool.totalMonths;
  const totalContributedHuman = Number(microUsdcToHuman(pool.totalContributed));
  // On-chain `current_month` ticks past total_months when month-12 advances
  // and the pool flips to is_complete. For display purposes, clamp so we
  // never render "13/12".
  const displayMonth = Math.min(pool.currentMonth, pool.totalMonths);
  const fill =
    pool.totalMonths > 0 ? (displayMonth / pool.totalMonths) * 100 : 0;

  const idShort = `PLVR-${pool.publicKey.toBase58().slice(0, 4).toUpperCase()}`;
  const tierLabel = pool.tier === "vault" ? "Tier 0 · Vault" : "Tier 1 · DeFi";

  return (
    <>
      <section className="hero">
        <div className="shell" style={{ paddingTop: 8 }}>
          <button
            className="btn ghost sm"
            onClick={() => router.push("/pools")}
            style={{ marginBottom: 14 }}
          >
            ← All pools
          </button>
        </div>
        <div className="shell hero-grid">
          <div>
            <div className="hero-kicker">
              <span className="sq" />
              {status === "forming"
                ? "FORMING"
                : status === "completed"
                  ? "COMPLETE"
                  : `LIVE · MONTH ${String(displayMonth).padStart(2, "0")}/${pool.totalMonths}`}
              {" · "}
              {idShort}
            </div>
            <h1
              className="hero-headline"
              style={{ fontSize: "clamp(36px, 4.4vw, 60px)" }}
            >
              {idShort}
              <br />
              <em>
                ${monthlyHuman.toLocaleString()}/mo · {pool.participantCount}/12
              </em>
            </h1>
            <p className="hero-deck">
              {status === "forming"
                ? `Filling. ${pool.participantCount} of 12 participants joined.`
                : status === "completed"
                  ? `Pool complete — all ${pool.totalMonths} months distributed.`
                  : `Month ${displayMonth} of ${pool.totalMonths}.`}{" "}
              Lifetime pool{" "}
              <b style={{ color: "var(--fg)" }}>
                ${lifetimeHuman.toLocaleString()}
              </b>
              {monthState && monthState.secondsUntilMonthEnd > 0 && (
                <>
                  {" "}· next deadline in{" "}
                  <b style={{ color: "var(--acc)" }}>
                    {fmtCountdown(monthState.secondsUntilMonthEnd)}
                  </b>
                </>
              )}
              .
            </p>
            <PoolActions
              pool={pool}
              participant={participant}
              monthState={monthState}
              onRefresh={refetch}
            />
            <div className="hero-byline">
              <span>{tierLabel}</span>
              <span>USDC · Solana</span>
              <span>FEE 1.50%</span>
            </div>
          </div>

          <div className="terminal">
            <div className="term-head">
              <span>
                <PoolverMark size={12} /> pool.account /{" "}
                {truncateAddress(pool.publicKey.toBase58(), 5)}
              </span>
              <div className="term-dots">
                <div className="d" />
                <div className="d" />
                <div className="d" />
              </div>
            </div>
            <div className="term-body">
              <PoolverMark size={240} className="terminal-watermark" />
              <div className="metric-label">Contributed (lifetime)</div>
              <div className="metric-value">
                ${totalContributedHuman.toLocaleString()}
                <span className="tick">_</span>
              </div>
              <div className="metric-bar">
                <div className="fill" style={{ width: `${fill}%` }} />
              </div>
              <div className="metric-sub">
                {displayMonth} of {pool.totalMonths} months
                {status === "active" && (
                  <>
                    {" · "}
                    {pool.paidCountForCurrentMonth}/12 paid current
                  </>
                )}
              </div>
              <div className="metric-kv">
                <span className="k">Monthly</span>
                <span className="v">
                  ${monthlyHuman.toLocaleString()}
                </span>
                <span className="k">Members</span>
                <span className="v">{pool.participantCount}/12</span>
                <span className="k">Tier</span>
                <span className="v">{tierLabel}</span>
                <span className="k">Bid window</span>
                <span className="v acc">
                  {monthState?.inBidWindow
                    ? `closes in ${fmtCountdown(
                        Math.max(
                          0,
                          pool.bidWindowEndsAt.toNumber() -
                            Math.floor(Date.now() / 1000)
                        )
                      )}`
                    : "—"}
                </span>
                <span className="k">Reveal window</span>
                <span className="v">
                  {monthState?.inRevealWindow
                    ? `closes in ${fmtCountdown(
                        Math.max(
                          0,
                          pool.revealWindowEndsAt.toNumber() -
                            Math.floor(Date.now() / 1000)
                        )
                      )}`
                    : "—"}
                </span>
                <span className="k">Distributed</span>
                <span className="v">
                  ${Number(microUsdcToHuman(pool.totalDistributed)).toLocaleString()}
                </span>
              </div>
            </div>
          </div>
        </div>
      </section>

      <MonthTimeline pool={pool} />
      <ParticipantRoster pool={pool} />
      <LotterySection
        pool={pool}
        participant={participant}
        monthState={monthState}
        onRefresh={refetch}
      />
      <ReserveStats tier={pool.tier} />

      <section className="shell section">
        <div className="landing-cta">
          <PoolverMark size={40} className="cta-mark" />
          <h2>Need help?</h2>
          <p>Read the protocol documentation for the full mechanic.</p>
          <div style={{ marginTop: 16 }}>
            <Link href="/docs" className="btn lg">
              Open docs →
            </Link>
          </div>
        </div>
      </section>
    </>
  );
}
