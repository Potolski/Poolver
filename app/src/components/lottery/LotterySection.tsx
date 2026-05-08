"use client";

import { useState } from "react";
import { toast } from "sonner";
import {
  advanceMonthIx,
  type ParticipantView,
  type PoolMonthState,
  type PoolView,
} from "@poolver/client";

import { PoolverMark } from "@/components/brand/PoolverLogo";
import { SectionHead } from "@/components/layout/SectionHead";
import { usePoolver } from "@/providers/PoolverProvider";
import { sendIxs } from "@/lib/tx-helpers";
import { fmtCountdown } from "@/lib/format";
import { BidPanel } from "./BidPanel";

interface Props {
  pool: PoolView;
  participant: ParticipantView | null;
  monthState: PoolMonthState | null;
  onRefresh: () => Promise<void>;
}

export function LotterySection({
  pool,
  participant,
  monthState,
  onRefresh,
}: Props) {
  const { client, connected } = usePoolver();
  const [advancing, setAdvancing] = useState(false);

  const month = monthState?.currentMonth ?? pool.currentMonth;
  const secsLeft = monthState?.secondsUntilMonthEnd ?? 0;
  const monthEnded = secsLeft <= 0;
  const stage = monthState?.inBidWindow
    ? { label: "BID OPEN", color: "var(--acc)" }
    : monthState?.inRevealWindow
      ? { label: "REVEAL OPEN", color: "var(--acc-2, var(--acc))" }
      : monthEnded
        ? { label: "MONTH ENDED", color: "var(--warn)" }
        : { label: "—", color: "var(--fg-3)" };

  const handleAdvance = async () => {
    if (!connected) {
      toast.error("Connect a wallet to advance the month");
      return;
    }
    setAdvancing(true);
    const toastId = toast.loading("Advancing month…");
    try {
      const ix = await advanceMonthIx(client, { pool: pool.publicKey });
      const sig = await sendIxs(client, [ix]);
      toast.success("Month advanced", {
        id: toastId,
        description: `sig: ${sig.slice(0, 12)}…`,
      });
      await onRefresh();
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      toast.error("Advance failed", {
        id: toastId,
        description: msg.slice(0, 200),
      });
    } finally {
      setAdvancing(false);
    }
  };

  return (
    <section className="shell section">
      <SectionHead
        n="03"
        title="Sealed-bid <em>Auction</em>"
        meta="Commit-reveal · winner selection"
      />
      <div className="vrf-grid">
        <div className="console">
          <div className="console-head">
            <span>
              <PoolverMark size={11} /> auction.month{String(month).padStart(2, "0")}
            </span>
            <span style={{ color: stage.color }}>● {stage.label}</span>
          </div>
          <div className="console-body">
            <div className="console-line">
              <span className="prompt">$</span>
              <span className="txt">
                month <b>{month}</b> / {pool.totalMonths} ·{" "}
                {pool.paidCountForCurrentMonth}/12 paid this month
              </span>
            </div>
            <div className="console-line">
              <span className="prompt">$</span>
              <span className="txt">
                stage: <b style={{ color: stage.color }}>{stage.label}</b> ·
                ends in <b>{fmtCountdown(secsLeft)}</b>
              </span>
            </div>
            <div className="console-line">
              <span className="prompt">&gt;</span>
              <span className="txt">
                bid_window_ends_at ·{" "}
                <span className="dim">
                  {new Date(
                    pool.bidWindowEndsAt.toNumber() * 1000
                  ).toISOString()}
                </span>
              </span>
            </div>
            <div className="console-line">
              <span className="prompt">&gt;</span>
              <span className="txt">
                reveal_window_ends_at ·{" "}
                <span className="dim">
                  {new Date(
                    pool.revealWindowEndsAt.toNumber() * 1000
                  ).toISOString()}
                </span>
              </span>
            </div>

            <div
              style={{
                display: "flex",
                gap: 10,
                marginTop: 16,
                flexWrap: "wrap",
                alignItems: "center",
              }}
            >
              {monthEnded && month > 0 && (
                <button
                  className="btn primary"
                  disabled={advancing}
                  onClick={handleAdvance}
                >
                  {advancing ? "Advancing…" : "↯ Advance month"}
                </button>
              )}
              {!monthEnded && (
                <span
                  style={{
                    fontFamily: "var(--mono)",
                    fontSize: 11,
                    color: "var(--fg-3)",
                  }}
                >
                  month auto-advances after duration elapses
                </span>
              )}
            </div>
          </div>
        </div>

        <div className="outcome">
          <PoolverMark size={280} className="outcome-watermark" />
          <div className="outcome-label">SELECTION MECHANIC</div>
          <div className="outcome-name" style={{ color: "var(--fg-2)" }}>
            sealed-bid &middot; commit-reveal
          </div>
          <div className="outcome-handle">
            INV-14: commit_hash = sha256(amount_le ‖ nonce_16 ‖ user_32)
          </div>
          <hr className="rule-dashed" />
          <div className="outcome-row">
            <span>Bid stake</span>
            <span className="v">1% (refundable)</span>
          </div>
          <div className="outcome-row">
            <span>Bid cap</span>
            <span className="v">20% of net pot</span>
          </div>
          <div className="outcome-row">
            <span>Tie-break</span>
            <span className="v">VRF lottery</span>
          </div>
          <div className="outcome-row">
            <span>Reveal nonce</span>
            <span className="v">stored in IndexedDB</span>
          </div>
          <pre className="ascii" style={{ marginTop: 16 }}>
{`   ┌── AUCTION FLOW ──────┐
   │  commit  ->  reveal  │
   │  reveal  ->  select  │
   │  select  ->  claim   │
   └──────────────────────┘`}
          </pre>
        </div>
      </div>
      <BidPanel
        pool={pool}
        participant={participant}
        monthState={monthState}
        onRefresh={onRefresh}
      />
    </section>
  );
}
