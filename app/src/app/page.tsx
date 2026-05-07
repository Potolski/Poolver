"use client";

import Link from "next/link";
import BN from "bn.js";
import { microUsdcToHuman } from "@poolver/client";
import { PoolverMark } from "@/components/brand/PoolverLogo";
import { Ticker } from "@/components/layout/Ticker";
import { SectionHead } from "@/components/layout/SectionHead";
import { usePools } from "@/hooks/usePools";
import { POOLVER_CORE_PROGRAM_ID } from "@/lib/constants";
import { fmtUSD } from "@/lib/format";
import { truncateAddress } from "@/lib/utils";

export default function Home() {
  const { pools, loading } = usePools();

  const totalContributedMicro = pools.reduce(
    (sum, p) => sum.add(p.totalContributed),
    new BN(0)
  );
  const tvl = Number(microUsdcToHuman(totalContributedMicro));
  const activeCount = pools.filter((p) => !p.isComplete && p.currentMonth > 0).length;
  const formingCount = pools.filter((p) => p.currentMonth === 0 && !p.isComplete).length;
  const completedCount = pools.filter((p) => p.isComplete).length;

  const programIdShort = truncateAddress(POOLVER_CORE_PROGRAM_ID.toBase58());

  return (
    <>
      <Ticker />

      <section className="hero">
        <div className="shell hero-grid">
          <div>
            <div className="hero-kicker">
              <span className="sq" />
              POOLVER PROTOCOL · v1.0-devnet
            </div>
            <h1 className="hero-headline">
              Pool <em>savings</em>,<br />
              not risk.
            </h1>
            <p className="hero-deck">
              An on-chain rotating savings + credit (consórcio / ROSCA) protocol.
              12 wallets pool monthly USDC; commit-reveal sealed bids select who
              receives each month. No administrator, no custodian — just four
              composable Solana programs running 24/7.
            </p>
            <div
              style={{
                display: "flex",
                gap: 10,
                marginBottom: 24,
                flexWrap: "wrap",
              }}
            >
              <Link href="/pools" className="btn primary lg">
                ▶ Browse pools
              </Link>
              <Link href="/create" className="btn lg">
                + Create a pool
              </Link>
              <Link href="/docs" className="btn ghost lg">
                Read the docs →
              </Link>
            </div>
            <div className="hero-byline">
              <span>PROGRAM {programIdShort}</span>
              <span>v1.0-devnet</span>
              <span>AUDIT PENDING</span>
            </div>
          </div>

          <div className="terminal">
            <div className="term-head">
              <span>
                <PoolverMark size={12} /> protocol.summary
              </span>
              <div className="term-dots">
                <div className="d" />
                <div className="d" />
                <div className="d" />
              </div>
            </div>
            <div className="term-body">
              <PoolverMark size={240} className="terminal-watermark" />
              <div className="metric-label">Total value locked</div>
              <div className="metric-value">
                {loading ? "…" : fmtUSD(tvl)}
                <span className="tick">_</span>
              </div>
              <div className="metric-bar">
                <div className="fill" />
              </div>
              <div className="metric-sub">
                Across {pools.length} pool{pools.length === 1 ? "" : "s"} ·{" "}
                {activeCount} active · {formingCount} forming
              </div>
              <div className="metric-kv">
                <span className="k">Protocol fee</span>
                <span className="v">1.50%</span>
                <span className="k">Reserve fee (Vault)</span>
                <span className="v">1.50%</span>
                <span className="k">Reserve fee (DeFi)</span>
                <span className="v">2.50%</span>
                <span className="k">Bid cap</span>
                <span className="v">20%</span>
                <span className="k">Network</span>
                <span className="v">Solana · 400ms</span>
                <span className="k">Settlement asset</span>
                <span className="v">SPL · USDC</span>
              </div>
            </div>
          </div>
        </div>
      </section>

      <section className="shell section">
        <SectionHead n="01" title="Why <em>Poolver</em>" meta="THE THESIS" />
        <div className="landing-cards">
          <div className="landing-card">
            <div className="lc-icon">
              <PoolverMark size={28} />
            </div>
            <h3>Rotation, not interest</h3>
            <p>
              ROSCAs have funded families for 500+ years. Each of 12 members pays
              a fixed monthly amount; one round you receive the whole pool. Over
              12 months everyone gets exactly what they put in — earlier
              liquidity for whoever bids most aggressively.
            </p>
          </div>
          <div className="landing-card">
            <div className="lc-icon">◈</div>
            <h3>Trustless by construction</h3>
            <p>
              Traditional consórcios charge 10–20% intermediary fees and gate by
              country. Poolver replaces the operator with four open-source
              Anchor programs. Protocol fee: 1.5%. No country lock-in.
            </p>
          </div>
          <div className="landing-card">
            <div className="lc-icon">◆</div>
            <h3>Reputation + reserves</h3>
            <p>
              Reputation-graduated collateral, tier-segregated reserve funds,
              commit-reveal sealed bids, and a default cascade with 30-day cure.
              Four layers of enforcement instead of one.
            </p>
          </div>
        </div>
      </section>

      <section className="shell section">
        <SectionHead n="02" title="Three moves" meta="MECHANIC" />
        <div className="landing-how">
          <div className="lh-step">
            <div className="lh-n">01</div>
            <div className="lh-k">JOIN</div>
            <div className="lh-t">
              KYC + reputation init. Your slot activates when the pool fills to
              12 participants.
            </div>
          </div>
          <div className="lh-step">
            <div className="lh-n">02</div>
            <div className="lh-k">CONTRIBUTE & BID</div>
            <div className="lh-t">
              Pay your monthly USDC. Optionally place a sealed bid (1% stake)
              for the round&apos;s pot.
            </div>
          </div>
          <div className="lh-step">
            <div className="lh-n">03</div>
            <div className="lh-k">REVEAL & RECEIVE</div>
            <div className="lh-t">
              Reveal your bid; highest wins (or VRF lottery if no bids). Post
              collateral, claim payout.
            </div>
          </div>
        </div>
      </section>

      <section className="shell section">
        <SectionHead n="03" title="Protocol at a glance" meta="LIVE DATA" />
        <div className="stats">
          <div className="stat">
            <div className="lbl">
              <PoolverMark size={11} /> Active pools
            </div>
            <div className="v">{loading ? "…" : pools.length}</div>
            <div className="sub">
              {formingCount} forming · {activeCount} active · {completedCount}{" "}
              complete
            </div>
            <div className="mini-bar">
              <div
                className="fill"
                style={{
                  width: `${pools.length === 0 ? 0 : Math.min(100, (activeCount / pools.length) * 100)}%`,
                }}
              />
            </div>
          </div>
          <div className="stat">
            <div className="lbl">
              <PoolverMark size={11} /> Total contributed
            </div>
            <div className="v">{loading ? "…" : fmtUSD(tvl)}</div>
            <div className="sub">USDC · cumulative across pools</div>
            <div className="mini-bar">
              <div className="fill" style={{ width: tvl > 0 ? "70%" : "0%" }} />
            </div>
          </div>
          <div className="stat">
            <div className="lbl">
              <PoolverMark size={11} /> Programs
            </div>
            <div className="v">4</div>
            <div className="sub">core · reserve · vault · defi</div>
            <div className="mini-bar">
              <div className="fill" style={{ width: "100%" }} />
            </div>
          </div>
        </div>
      </section>

      <section className="shell section">
        <SectionHead n="04" title="Poolvers <em>now forming</em>" meta="FEATURED" />
        <div
          style={{
            padding: "48px 16px",
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
            {loading ? "LOADING…" : `${pools.length} POOL${pools.length === 1 ? "" : "S"} ON DEVNET`}
          </div>
          <div style={{ color: "var(--fg-2)", fontSize: 14, marginBottom: 16 }}>
            {pools.length === 0
              ? "Be the first to create a Poolver."
              : "Browse the live index to see all pools and their state."}
          </div>
          <div style={{ display: "flex", gap: 10, justifyContent: "center", flexWrap: "wrap" }}>
            <Link href="/pools" className="btn primary">
              ▶ Browse pools
            </Link>
            <Link href="/create" className="btn">
              + Create a pool
            </Link>
          </div>
        </div>
      </section>

      <section className="shell section">
        <div className="landing-cta">
          <PoolverMark size={56} className="cta-mark" />
          <h2>Ready to join a Poolver?</h2>
          <p>
            Browse live devnet pools, or configure your own in under 5 minutes.
          </p>
          <div
            style={{
              display: "flex",
              gap: 10,
              justifyContent: "center",
              flexWrap: "wrap",
              marginTop: 20,
            }}
          >
            <Link href="/pools" className="btn primary lg">
              ▶ Browse pools
            </Link>
            <Link href="/create" className="btn lg">
              + Create a pool
            </Link>
          </div>
        </div>
      </section>
    </>
  );
}
