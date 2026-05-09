"use client";

import BN from "bn.js";
import { PublicKey } from "@solana/web3.js";
import { microUsdcToHuman, type PoolView } from "@poolver/client";

import { PoolverMark } from "@/components/brand/PoolverLogo";
import { SectionHead } from "@/components/layout/SectionHead";
import { fmtUSD } from "@/lib/format";
import { truncateAddress } from "@/lib/utils";

interface MonthWinnerRaw {
  month: number;
  winner: PublicKey;
  winningBid: BN;
  grossPayout: BN;
  netPayout: BN;
  selectedAt: BN;
  selectionMethod: { bid?: object; lottery?: object };
  claimed: boolean;
}

function readWinners(pool: PoolView): MonthWinnerRaw[] {
  const arr = (pool.raw as { winners?: MonthWinnerRaw[] }).winners;
  return Array.isArray(arr) ? arr : [];
}

export function MonthTimeline({ pool }: { pool: PoolView }) {
  const winners = readWinners(pool);
  const cells = Array.from({ length: pool.totalMonths }, (_, i) => i + 1);
  const distributedHuman = Number(microUsdcToHuman(pool.totalDistributed));
  const contributedHuman = Number(microUsdcToHuman(pool.totalContributed));
  const completed = winners.filter((w) => w.selectedAt && w.selectedAt.gtn?.(0)).length;
  const distributedTarget =
    Number(microUsdcToHuman(pool.contributionAmount)) * pool.totalMonths;

  return (
    <section className="shell section">
      <SectionHead
        n="01"
        title="Month <em>Timeline</em>"
        meta={`${pool.totalMonths} months · ${pool.totalMonths} recipients`}
      />
      <div className="timeline">
        <div className="months">
          {cells.map((m) => {
            const w = winners[m - 1];
            const hasWinner = w && w.selectedAt && w.selectedAt.gtn?.(0);
            const cls = hasWinner
              ? "done"
              : m === pool.currentMonth
                ? "current"
                : "empty";
            const winnerLabel = hasWinner
              ? truncateAddress(w.winner.toBase58(), 4)
              : m === pool.currentMonth
                ? "pending"
                : "—";
            // Compact format ($5.8K / $50K / $1.2M) so even a 12-cell row
            // on narrow viewports doesn't overflow. Tooltip on the cell
            // shows the exact figure for anyone who needs it.
            const amount =
              hasWinner && w.netPayout
                ? fmtUSD(Number(microUsdcToHuman(w.netPayout)))
                : "";
            const fullAmount =
              hasWinner && w.netPayout
                ? `$${Number(microUsdcToHuman(w.netPayout)).toLocaleString()}`
                : "";
            const tooltip = hasWinner
              ? `Month ${m} · winner ${w.winner.toBase58()} · ${fullAmount}`
              : m === pool.currentMonth
                ? `Month ${m} (current)`
                : `Month ${m}`;
            return (
              <div key={m} className={`month ${cls}`} title={tooltip}>
                <div className="m-n">M{String(m).padStart(2, "0")}</div>
                <div className="m-w">{winnerLabel}</div>
                <div className="m-a">{amount}</div>
              </div>
            );
          })}
        </div>
        <div className="timeline-side">
          <h4>
            <PoolverMark size={11} /> Pool stats
          </h4>
          <div className="kv-list">
            <div className="kv-row">
              <span className="k">Months complete</span>
              <span className="v">
                {String(completed).padStart(2, "0")} / {pool.totalMonths}
              </span>
            </div>
            <div className="kv-row">
              <span className="k">Distributed</span>
              <span className="v">{fmtUSD(distributedHuman)}</span>
            </div>
            <div className="kv-row">
              <span className="k">Outstanding</span>
              <span className="v">
                {fmtUSD(Math.max(0, distributedTarget - distributedHuman))}
              </span>
            </div>
            <div className="kv-row">
              <span className="k">Contributed total</span>
              <span className="v">{fmtUSD(contributedHuman)}</span>
            </div>
            <div className="kv-row">
              <span className="k">Tier</span>
              <span className="v">
                {pool.tier === "vault" ? "Vault (0)" : "DeFi (1)"}
              </span>
            </div>
            <div className="kv-row">
              <span className="k">Paid this month</span>
              <span className="v acc">
                {pool.paidCountForCurrentMonth} / 12
              </span>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
