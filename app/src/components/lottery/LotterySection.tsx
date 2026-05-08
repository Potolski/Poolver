"use client";

import { useEffect, useState } from "react";
import { ComputeBudgetProgram, PublicKey } from "@solana/web3.js";
import BN from "bn.js";
import { toast } from "sonner";
import {
  POOLVER_ALT_DEVNET,
  advanceMonthIx,
  selectWinnerIx,
  type ParticipantView,
  type PoolMonthState,
  type PoolView,
} from "@poolver/client";

import { PoolverMark } from "@/components/brand/PoolverLogo";
import { SectionHead } from "@/components/layout/SectionHead";
import { usePoolver } from "@/providers/PoolverProvider";
import { sendIxs, sendIxsV0 } from "@/lib/tx-helpers";
import { fmtCountdown } from "@/lib/format";
import { BidPanel } from "./BidPanel";

interface BidAccountRaw {
  pool: PublicKey;
  user: PublicKey;
  month: number;
  revealed: boolean;
  isWinner: boolean;
}

interface BidAccountClient {
  all: (filters?: unknown[]) => Promise<
    Array<{ publicKey: PublicKey; account: BidAccountRaw }>
  >;
}

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
  const { client, connected, publicKey } = usePoolver();
  const [advancing, setAdvancing] = useState(false);
  const [selecting, setSelecting] = useState(false);
  const [bidStats, setBidStats] = useState<{
    committed: number;
    revealed: number;
    /** All wallets that called commit_bid this month (revealed or not). */
    bidders: PublicKey[];
    winnerSelected: boolean;
  } | null>(null);

  const month = monthState?.currentMonth ?? pool.currentMonth;
  const secsLeft = monthState?.secondsUntilMonthEnd ?? 0;
  const monthEnded = secsLeft <= 0;
  const revealWindowClosed =
    pool.revealWindowEndsAt.gtn(0) &&
    Date.now() / 1000 >= pool.revealWindowEndsAt.toNumber();

  // Authoritative "winner has been drawn for this month" check. We
  // can't rely on `bidStats.winnerSelected` (which derives from the
  // `Bid.is_winner` flag) because lottery winners have no Bid PDA —
  // their MonthWinner entry is the only signal. This mirrors how
  // MonthTimeline + ParticipantRoster already read winners.
  const winners =
    ((pool.raw as {
      winners?: Array<{
        month: number;
        winner: PublicKey;
        winningBid: BN;
        netPayout: BN;
        selectedAt: BN;
        selectionMethod: { bid?: object; lottery?: object };
      }>;
    }).winners) ?? [];
  const monthWinnerSelected =
    month > 0 && month <= winners.length
      ? Boolean(winners[month - 1]?.selectedAt?.gtn?.(0))
      : false;

  const stage = monthState?.inBidWindow
    ? { label: "BID OPEN", color: "var(--acc)" }
    : monthState?.inRevealWindow
      ? { label: "REVEAL OPEN", color: "var(--acc-2, var(--acc))" }
      : revealWindowClosed && month > 0 && !monthWinnerSelected
        ? { label: "READY TO DRAW", color: "var(--warn)" }
        : monthEnded
          ? { label: "MONTH ENDED", color: "var(--warn)" }
          : { label: "—", color: "var(--fg-3)" };

  // Pull current-month bid stats so the UI can show "X committed / Y revealed"
  // and decide whether to surface the "Run draw" button.
  useEffect(() => {
    if (!month || month < 1) return;
    let cancelled = false;
    const bidClient = (
      client.core.account as unknown as { bid: BidAccountClient }
    ).bid;
    bidClient
      .all([
        { memcmp: { offset: 8, bytes: pool.publicKey.toBase58() } },
      ])
      .then((accounts) => {
        if (cancelled) return;
        const thisMonth = accounts.filter((a) => a.account.month === month);
        const revealed = thisMonth.filter((a) => a.account.revealed);
        const winnerSelected = thisMonth.some((a) => a.account.isWinner);
        setBidStats({
          committed: thisMonth.length,
          revealed: revealed.length,
          bidders: thisMonth.map((a) => a.account.user),
          winnerSelected,
        });
      })
      .catch(() => {
        if (cancelled) return;
        setBidStats({ committed: 0, revealed: 0, bidders: [], winnerSelected: false });
      });
    return () => {
      cancelled = true;
    };
  }, [client, pool.publicKey, month, pool.currentMonth]);

  const handleSelect = async () => {
    if (!connected) {
      toast.error("Connect a wallet to run the draw");
      return;
    }
    if (!bidStats) {
      toast.error("Loading bid state — try again in a moment");
      return;
    }
    setSelecting(true);
    const toastId = toast.loading("Selecting winner…");
    try {
      // Build the candidate sets:
      //   bidders     = every wallet that called commit_bid this month
      //                 (revealed or not — unrevealed gets stake forfeit)
      //   nonBidders  = active participants who didn't bid (lottery pool)
      //
      // TX-SIZE: each non-bidder costs 2 accounts (~64 bytes) in the tx.
      // 12 non-bidders + 9 base accounts puts us right at the 1232-byte
      // wire limit. Two optimizations to stay under:
      //   (1) skip non-bidders entirely when at least one bid revealed —
      //       the handler enters the bid branch and never touches lottery
      //       candidates.
      //   (2) skip non-bidders who already won a past month — the handler
      //       filters them via `is_eligible` anyway, so we just save bytes.
      const bidderSet = new Set(bidStats.bidders.map((p) => p.toBase58()));
      const winnerSet = new Set<string>();
      const winners =
        ((pool.raw as { winners?: Array<{ winner: PublicKey; selectedAt: { gtn?: (n: number) => boolean } }> })
          .winners) ?? [];
      for (const w of winners) {
        if (w.selectedAt && w.selectedAt.gtn?.(0)) {
          winnerSet.add(w.winner.toBase58());
        }
      }
      const allParticipants = (
        pool.raw as { participants?: Array<PublicKey | null> }
      ).participants ?? [];
      const nonBidders: PublicKey[] = [];
      if (bidStats.revealed === 0) {
        for (const p of allParticipants) {
          if (!p) continue;
          const k = p.toBase58();
          if (bidderSet.has(k)) continue;
          if (winnerSet.has(k)) continue;
          nonBidders.push(p);
        }
      }
      const ix = await selectWinnerIx(client, {
        pool: pool.publicKey,
        tier: pool.tier,
        month,
        bidders: bidStats.bidders,
        nonBidders,
      });
      // Bump CU limit above the 200k default. select_winner walks up
      // to 12 (Participant, Kyc) pairs, sha256-hashes each tiebreak,
      // and may CPI into reserve to forfeit unrevealed stakes — the
      // 200k default is too tight and the wallet's static simulator
      // raises a "transaction may fail" warning. 400k leaves headroom
      // and silences the warning without affecting fee meaningfully.
      const cuIx = ComputeBudgetProgram.setComputeUnitLimit({
        units: 400_000,
      });
      // v0 + ALT: the protocol-static ALT holds 8 addresses
      // (protocol_config, core_invoker, reserve_fund × tier,
      // reserve_vault × tier, reserve_program, token_program), which
      // saves ~248 bytes — enough headroom for a 12-non-bidder lottery
      // draw to fit under the 1232-byte legacy-tx cap.
      const sig = await sendIxsV0(client, [cuIx, ix], [POOLVER_ALT_DEVNET]);
      toast.success("Winner selected", {
        id: toastId,
        description: `sig: ${sig.slice(0, 12)}…`,
      });
      await onRefresh();
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      toast.error("Select failed", {
        id: toastId,
        description: msg.slice(0, 200),
      });
    } finally {
      setSelecting(false);
    }
  };

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
              {revealWindowClosed && month > 0 && !monthWinnerSelected && (
                <button
                  className="btn primary"
                  disabled={selecting}
                  onClick={handleSelect}
                  title={
                    bidStats?.revealed
                      ? `Pick the highest of ${bidStats.revealed} revealed bid(s)`
                      : "No revealed bids — fall back to lottery (mock VRF)"
                  }
                >
                  {selecting ? "Selecting…" : "▶ Run draw / select winner"}
                </button>
              )}
              {monthEnded && month > 0 && monthWinnerSelected && (
                <button
                  className="btn primary"
                  disabled={advancing}
                  onClick={handleAdvance}
                  title="Advance to the next month"
                >
                  {advancing ? "Advancing…" : "↯ Advance month"}
                </button>
              )}
              {monthEnded && month > 0 && !monthWinnerSelected && (
                <span
                  style={{
                    fontFamily: "var(--mono)",
                    fontSize: 11,
                    color: "var(--warn)",
                  }}
                >
                  ⚠ draw the month winner before advancing
                </span>
              )}
              {!monthEnded && !revealWindowClosed && (
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
          <hr className="rule-dashed" />
          <div className="outcome-label" style={{ marginBottom: 8 }}>
            MONTH {String(month).padStart(2, "0")} BIDS
          </div>
          <div className="outcome-row">
            <span>Committed</span>
            <span className="v">{bidStats?.committed ?? "…"}</span>
          </div>
          <div className="outcome-row">
            <span>Revealed</span>
            <span className="v">{bidStats?.revealed ?? "…"}</span>
          </div>
          <div className="outcome-row">
            <span>Winner</span>
            <span className="v">
              {monthWinnerSelected
                ? "✓ selected"
                : revealWindowClosed
                  ? "pending — click Run draw"
                  : "—"}
            </span>
          </div>
        </div>
      </div>
      {monthWinnerSelected && (() => {
        const w = winners[month - 1];
        const isBid = w?.selectionMethod && "bid" in w.selectionMethod;
        return (
          <div
            className="card"
            style={{
              marginTop: 18,
              padding: 18,
              background: "var(--bg-1)",
              border: "1px solid var(--line)",
              fontFamily: "var(--mono)",
              fontSize: 12,
              color: "var(--fg-2)",
              lineHeight: 1.7,
            }}
          >
            <div
              className="kicker"
              style={{ marginBottom: 10, color: "var(--fg-3)" }}
            >
              AUDIT · MONTH {String(month).padStart(2, "0")} SELECTION
            </div>
            {isBid ? (
              <>
                <div>
                  <b style={{ color: "var(--acc)" }}>Method:</b> sealed-bid
                  auction (deterministic, no randomness involved).
                </div>
                <div style={{ marginTop: 6, color: "var(--fg-3)" }}>
                  All bids were committed as a sha256 hash during the bid
                  window, then revealed in the reveal window. The handler
                  picks the highest revealed amount; ties are broken by
                  the lexicographically smallest{" "}
                  <code>sha256(pool ‖ month ‖ user)</code>. The full bid
                  list and winner are public on-chain — verifiable from
                  the Bid PDAs for this pool + month.
                </div>
              </>
            ) : (
              <>
                <div>
                  <b style={{ color: "var(--acc)" }}>Method:</b> VRF lottery
                  (no revealed bids → uniform random pick).
                </div>
                <div style={{ marginTop: 6, color: "var(--fg-3)" }}>
                  Seed ={" "}
                  <code>sha256(pool ‖ month_le ‖ slot)</code>. The slot is
                  the Solana slot at draw time — public and immutable, but
                  unpredictable to anyone trying to grind the candidate
                  list (slots advance ~400ms apart). First 8 bytes of the
                  seed → u64 → mod candidate_count → winner index.
                </div>
                <div style={{ marginTop: 6, color: "var(--fg-3)" }}>
                  Candidate list is the on-chain{" "}
                  <code>pool.participants</code> filtered for: not
                  defaulted, not suspended, Full-KYC, no prior win. That
                  filter and the seed inputs are all derivable from
                  on-chain state, so anyone can reproduce the pick.
                </div>
                <div
                  style={{
                    marginTop: 10,
                    padding: 10,
                    border: "1px dashed var(--line-2)",
                    borderRadius: 3,
                    color: "var(--fg-4)",
                    fontSize: 11,
                  }}
                >
                  V1 uses a deterministic mock VRF for demo simplicity;
                  production swaps in Switchboard On-Demand without any
                  state-shape change (see select_winner.rs §SPEC-21).
                </div>
              </>
            )}
            <hr className="rule-dashed" style={{ margin: "12px 0" }} />
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                gap: 18,
                flexWrap: "wrap",
              }}
            >
              <div>
                <div style={{ color: "var(--fg-4)", fontSize: 11 }}>
                  Winner
                </div>
                <div style={{ color: "var(--fg-1)" }}>
                  {w.winner.toBase58()}
                </div>
              </div>
              <div>
                <div style={{ color: "var(--fg-4)", fontSize: 11 }}>
                  Selected at
                </div>
                <div>
                  {new Date(w.selectedAt.toNumber() * 1000).toISOString()}
                </div>
              </div>
            </div>
          </div>
        );
      })()}
      <BidPanel
        pool={pool}
        participant={participant}
        monthState={monthState}
        onRefresh={onRefresh}
      />
    </section>
  );
}
