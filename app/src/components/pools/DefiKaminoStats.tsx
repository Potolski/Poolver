"use client";

import { useEffect, useState } from "react";
import { getAccount } from "@solana/spl-token";
import {
  findAdapterUsdc,
  microUsdcToHuman,
  type PoolView,
} from "@poolver/client";
import BN from "bn.js";

import { PoolverMark } from "@/components/brand/PoolverLogo";
import { SectionHead } from "@/components/layout/SectionHead";
import { usePoolver } from "@/providers/PoolverProvider";
import { fmtUSD } from "@/lib/format";

/**
 * Stats card visible only on Tier 1 (DeFi) pools — surfaces the Kamino
 * mock-integration's accumulated USDC and a placeholder APY. Polls every
 * 10s so the deployed balance reflects fresh contributions / slashes.
 *
 * V1 NOTE: `poolver-yield-defi` is a Kamino *mock* — the underlying
 * adapter just holds USDC in a vault PDA. The "yield" shown here is the
 * spread between the adapter's live token balance and the protocol's
 * cumulative net-contribution ledger (`pool.totalContributed × 0.97`),
 * minus the participant payouts already drawn (`pool.totalDistributed`).
 * In production the yield comes from Kamino's lending pool — same UI,
 * just real numbers.
 */
export function DefiKaminoStats({ pool }: { pool: PoolView }) {
  const { client } = usePoolver();
  const [deployed, setDeployed] = useState<BN | null>(null);

  useEffect(() => {
    let cancelled = false;
    const [adapterUsdc] = findAdapterUsdc("defi", pool.publicKey);
    const tick = async () => {
      try {
        const acct = await getAccount(client.provider.connection, adapterUsdc);
        if (!cancelled) setDeployed(new BN(acct.amount.toString()));
      } catch {
        if (!cancelled) setDeployed(new BN(0));
      }
    };
    void tick();
    const id = setInterval(tick, 10_000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, [client, pool.publicKey]);

  // Net-deposit estimate. Each contribution lands `1 - protocol_fee_bps -
  // reserve_fee_bps` in the adapter (3% total fee for DeFi tier in V1).
  // Slashes go in at FULL contribution_amount — so the on-chain ledger
  // is roughly: total_contributed × 0.97 + slashes × 0.03 ≈ slightly
  // above 0.97 × total_contributed. Approximation good enough for a
  // demo "yield = balance − expected" gauge.
  const expectedNet = pool.totalContributed.muln(9700).divn(10_000);
  const distributed = pool.totalDistributed;
  const expectedDeployed = expectedNet.sub(distributed);
  const yieldMicro =
    deployed && deployed.gt(expectedDeployed)
      ? deployed.sub(expectedDeployed)
      : new BN(0);
  const deployedHuman = deployed
    ? Number(microUsdcToHuman(deployed))
    : null;
  const yieldHuman = Number(microUsdcToHuman(yieldMicro));
  const apyEstimate =
    deployedHuman && deployedHuman > 0
      ? Math.min(99, (yieldHuman / deployedHuman) * 100 * 12)
      : 0;

  return (
    <section className="shell section">
      <SectionHead
        n="05"
        title="Kamino <em>Position</em>"
        meta="Tier 1 · DeFi yield adapter"
      />
      <div className="stats">
        <div className="stat">
          <div className="lbl">
            <PoolverMark size={11} /> Deployed in Kamino
          </div>
          <div className="v">
            {deployedHuman === null ? "…" : fmtUSD(deployedHuman)}
          </div>
          <div className="sub">live adapter USDC vault balance</div>
          <div className="mini-bar">
            <div
              className="fill"
              style={{
                width:
                  deployedHuman && deployedHuman > 0 ? "100%" : "0%",
              }}
            />
          </div>
        </div>
        <div className="stat">
          <div className="lbl">
            <PoolverMark size={11} /> Accrued yield
          </div>
          <div className="v">
            {deployedHuman === null ? "…" : fmtUSD(yieldHuman)}
          </div>
          <div className="sub">
            {apyEstimate > 0
              ? `~${apyEstimate.toFixed(2)}% APY (annualized estimate)`
              : "starts compounding once first claim reduces the float"}
          </div>
          <div className="mini-bar">
            <div
              className="fill"
              style={{
                width: `${Math.min(100, apyEstimate * 5)}%`,
              }}
            />
          </div>
        </div>
        <div className="stat">
          <div className="lbl">
            <PoolverMark size={11} /> Strategy
          </div>
          <div
            className="v"
            style={{ fontSize: 18, lineHeight: 1.2 }}
          >
            Kamino Lend
          </div>
          <div className="sub">
            <a
              href="https://app.kamino.finance/"
              target="_blank"
              rel="noopener noreferrer"
              style={{
                color: "var(--acc)",
                textDecoration: "none",
                borderBottom: "1px dashed var(--acc)",
              }}
            >
              app.kamino.finance ↗
            </a>
            <span style={{ marginLeft: 6, color: "var(--fg-4)" }}>
              · auto-compounding USDC
            </span>
          </div>
          <div
            className="mini-bar"
            title="V1 devnet uses a Kamino mock adapter; production routes to a real Kamino lending market."
          >
            <div className="fill" style={{ width: "100%" }} />
          </div>
        </div>
      </div>
      <p
        style={{
          marginTop: 12,
          fontFamily: "var(--mono)",
          fontSize: 11,
          color: "var(--fg-4)",
          letterSpacing: "0.04em",
        }}
      >
        ⚠ V1 devnet uses a Kamino mock adapter — yield is approximated from the
        gap between the adapter's live USDC balance and the on-chain
        net-contribution ledger. Production swaps in a real Kamino Lend
        market without changing this page.
      </p>
    </section>
  );
}
