"use client";

import { useEffect, useState } from "react";
import {
  fetchReserveFund,
  microUsdcToHuman,
  type ReserveFundView,
  type TierName,
} from "@poolver/client";

import { PoolverMark } from "@/components/brand/PoolverLogo";
import { SectionHead } from "@/components/layout/SectionHead";
import { usePoolver } from "@/providers/PoolverProvider";
import { fmtUSD } from "@/lib/format";

export function ReserveStats({ tier }: { tier: TierName }) {
  const { client } = usePoolver();
  const [reserve, setReserve] = useState<ReserveFundView | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    const tick = () => {
      fetchReserveFund(client, tier)
        .then((r) => {
          if (!cancelled) {
            setReserve(r);
            setLoading(false);
          }
        })
        .catch(() => {
          if (!cancelled) {
            setReserve(null);
            setLoading(false);
          }
        });
    };
    setLoading(true);
    tick();
    // Poll every 10s so reserve flows from successive joins / contributions
    // / yield distributions / default drawdowns appear live.
    const id = setInterval(tick, 10_000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, [client, tier]);

  const total = reserve ? Number(microUsdcToHuman(reserve.totalBalance)) : 0;
  const inflows = reserve ? Number(microUsdcToHuman(reserve.totalInflows)) : 0;
  const outflows = reserve ? Number(microUsdcToHuman(reserve.totalOutflows)) : 0;
  const utilization =
    inflows > 0 ? Math.min(100, Math.round((outflows / inflows) * 100)) : 0;

  const tierLabel = tier === "vault" ? "Tier 0 · Vault" : "Tier 1 · DeFi";

  return (
    <section className="shell section">
      <SectionHead
        n="04"
        title="Reserve <em>Fund</em>"
        meta={`${tierLabel} · solvency backstop`}
      />
      <div className="stats">
        <div className="stat">
          <div className="lbl">
            <PoolverMark size={11} /> Total reserves
          </div>
          <div className="v">{loading ? "…" : fmtUSD(total)}</div>
          <div className="sub">{tierLabel}</div>
          <div className="mini-bar">
            <div
              className="fill"
              style={{ width: total > 0 ? "70%" : "0%" }}
            />
          </div>
        </div>
        <div className="stat">
          <div className="lbl">
            <PoolverMark size={11} /> Total inflows
          </div>
          <div className="v">{loading ? "…" : fmtUSD(inflows)}</div>
          <div className="sub">
            {tier === "vault" ? "1.5%" : "2.5%"} per contribution
          </div>
          <div className="mini-bar">
            <div className="fill" style={{ width: inflows > 0 ? "100%" : "0%" }} />
          </div>
        </div>
        <div className="stat">
          <div className="lbl">
            <PoolverMark size={11} /> Drawdowns (defaults)
          </div>
          <div className="v">{loading ? "…" : fmtUSD(outflows)}</div>
          <div className="sub">
            {utilization}% of inflows used to cover defaults
          </div>
          <div className="mini-bar">
            <div className="fill" style={{ width: `${utilization}%` }} />
          </div>
        </div>
      </div>
    </section>
  );
}
