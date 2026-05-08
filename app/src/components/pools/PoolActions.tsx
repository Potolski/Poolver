"use client";

import { useState } from "react";
import { toast } from "sonner";
import { useAppKit } from "@reown/appkit/react";
import {
  claimWinningIx,
  contributeIx,
  hasPaidMonth,
  joinPoolIx,
  type ParticipantView,
  type PoolMonthState,
  type PoolView,
} from "@poolver/client";

import { usePoolver } from "@/providers/PoolverProvider";
import { useOnboarding } from "@/hooks/useOnboarding";
import { ensureAtaIx, sendIxs } from "@/lib/tx-helpers";
import { USDC_MINT_DEVNET_DEFAULT } from "@/lib/constants";

interface Props {
  pool: PoolView;
  participant: ParticipantView | null;
  monthState: PoolMonthState | null;
  onRefresh: () => Promise<void>;
}

type Busy = "join" | "contribute" | "claim" | null;

export function PoolActions({ pool, participant, monthState, onRefresh }: Props) {
  const { client, connected, publicKey } = usePoolver();
  const { state: onboardingState } = useOnboarding();
  const { open } = useAppKit();
  const [busy, setBusy] = useState<Busy>(null);

  const run = async (
    kind: Exclude<Busy, null>,
    label: string,
    fn: () => Promise<string>
  ) => {
    setBusy(kind);
    const toastId = toast.loading(`${label}…`);
    try {
      const sig = await fn();
      toast.success(`${label} confirmed`, {
        id: toastId,
        description: `sig: ${sig.slice(0, 12)}…`,
      });
      await onRefresh();
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      toast.error(`${label} failed`, {
        id: toastId,
        description: msg.slice(0, 200),
      });
    } finally {
      setBusy(null);
    }
  };

  const submitJoin = async () => {
    if (!publicKey) throw new Error("wallet not connected");
    const { preIx } = ensureAtaIx(publicKey, publicKey, USDC_MINT_DEVNET_DEFAULT);
    const ix = await joinPoolIx(client, {
      pool: pool.publicKey,
      tier: pool.tier,
      usdcMint: USDC_MINT_DEVNET_DEFAULT,
    });
    return sendIxs(client, [preIx, ix]);
  };

  const submitContribute = async () => {
    const ix = await contributeIx(client, {
      pool: pool.publicKey,
      tier: pool.tier,
      usdcMint: USDC_MINT_DEVNET_DEFAULT,
    });
    return sendIxs(client, [ix]);
  };

  const submitClaim = async (claimMonth: number) => {
    const ix = await claimWinningIx(client, {
      pool: pool.publicKey,
      tier: pool.tier,
      usdcMint: USDC_MINT_DEVNET_DEFAULT,
      claimMonth,
    });
    return sendIxs(client, [ix]);
  };

  if (!connected) {
    return (
      <div style={{ display: "flex", gap: 10, marginBottom: 24, flexWrap: "wrap" }}>
        <button className="btn primary lg" onClick={() => open()}>
          ▶ Connect wallet to join
        </button>
      </div>
    );
  }

  if (onboardingState !== "ready") {
    const label =
      onboardingState === "needs_reputation"
        ? "Initialize your account"
        : onboardingState === "needs_kyc"
          ? "Verify identity (demo KYC)"
          : "Loading onboarding…";
    return (
      <div style={{ display: "flex", gap: 10, marginBottom: 24, flexWrap: "wrap" }}>
        <button className="btn lg" disabled>
          ⚠ {label}
        </button>
      </div>
    );
  }

  // From here: connected + onboarding ready.
  const isForming = pool.currentMonth === 0 && !pool.isComplete;
  const isComplete = pool.isComplete;
  const isParticipant = !!participant;
  const isFull = pool.participantCount >= 12;

  const currentMonth = monthState?.currentMonth ?? pool.currentMonth;
  const paidThisMonth =
    participant && currentMonth > 0
      ? hasPaidMonth(participant, currentMonth)
      : false;

  // Winner status lives on `pool.winners[m-1]`, NOT on `participant.hasWon`
  // (that flag only flips INSIDE claim_winning). Scan ALL past months —
  // a winner may not have claimed before the month advanced. The on-chain
  // claim_winning ix accepts a `claim_month` arg so retroactive claims
  // work even after advance_month.
  type RawWinner = {
    winner: { toBase58: () => string };
    selectedAt: { gtn?: (n: number) => boolean };
    claimed: boolean;
  };
  const winners = (pool.raw as { winners?: RawWinner[] }).winners ?? [];
  const myUnclaimedWinMonth = (() => {
    const me = publicKey?.toBase58();
    if (!me) return null;
    for (let m = 1; m <= Math.min(currentMonth, 12); m++) {
      const w = winners[m - 1];
      if (
        w &&
        w.selectedAt?.gtn?.(0) &&
        !w.claimed &&
        w.winner.toBase58() === me
      )
        return m;
    }
    return null;
  })();

  // Current month's winner (for non-winner banner)
  const monthWinner = currentMonth > 0 ? winners[currentMonth - 1] : undefined;
  const winnerSelectedThisMonth =
    !!monthWinner?.selectedAt && monthWinner.selectedAt.gtn?.(0);
  const isWinner = myUnclaimedWinMonth !== null;

  if (isComplete) {
    return (
      <div style={{ display: "flex", gap: 10, marginBottom: 24 }}>
        <button className="btn lg" disabled>
          ◉ Pool complete
        </button>
      </div>
    );
  }

  if (isForming) {
    if (isParticipant) {
      return (
        <div style={{ display: "flex", gap: 10, marginBottom: 24, flexWrap: "wrap" }}>
          <button className="btn lg" disabled>
            ✓ Joined · waiting for {12 - pool.participantCount} more
          </button>
        </div>
      );
    }
    return (
      <div style={{ display: "flex", gap: 10, marginBottom: 24, flexWrap: "wrap" }}>
        <button
          className="btn primary lg"
          disabled={isFull || busy !== null}
          onClick={() => run("join", "Joining pool", submitJoin)}
        >
          {isFull
            ? "Pool full"
            : busy === "join"
              ? "Signing…"
              : "▶ Join pool"}
        </button>
      </div>
    );
  }

  // Active month
  if (!isParticipant) {
    return (
      <div style={{ display: "flex", gap: 10, marginBottom: 24 }}>
        <button className="btn lg" disabled>
          Pool closed to new members
        </button>
      </div>
    );
  }

  // Surface "you won, claim now" messaging even before the button.
  // Note: a winner can claim retroactively (any past month they won
  // and haven't claimed yet) — the pool no longer blocks advance on
  // unclaimed winners.
  const banner = isWinner
    ? `🎉 You won month ${myUnclaimedWinMonth}! Click "Claim winnings" to post collateral and receive the pot. (Retroactive claim — works even if the month has already advanced.)`
    : winnerSelectedThisMonth && !monthWinner?.claimed
      ? `Winner of month ${currentMonth}: ${monthWinner!.winner.toBase58().slice(0, 8)}… — they can claim now or later (the pool can advance without it).`
      : null;

  return (
    <>
      {banner && (
        <div
          style={{
            marginBottom: 14,
            padding: "10px 14px",
            border: `1px solid ${isWinner ? "var(--acc)" : "var(--line-2)"}`,
            background: isWinner ? "var(--acc-tint)" : "var(--bg-2)",
            color: isWinner ? "var(--acc)" : "var(--fg-2)",
            borderRadius: 3,
            fontFamily: "var(--mono)",
            fontSize: 12.5,
            lineHeight: 1.5,
          }}
        >
          {banner}
        </div>
      )}
      <div style={{ display: "flex", gap: 10, marginBottom: 24, flexWrap: "wrap" }}>
        <button
          className="btn primary lg"
          disabled={paidThisMonth || busy !== null}
          onClick={() => run("contribute", "Sending contribution", submitContribute)}
        >
          {paidThisMonth
            ? `✓ Paid month ${currentMonth}`
            : busy === "contribute"
              ? "Signing…"
              : `▶ Pay month ${currentMonth}`}
        </button>
        {isWinner && myUnclaimedWinMonth !== null && (
          <button
            className="btn primary lg"
            disabled={busy !== null}
            onClick={() =>
              run("claim", `Claiming month ${myUnclaimedWinMonth}`, () =>
                submitClaim(myUnclaimedWinMonth)
              )
            }
          >
            {busy === "claim"
              ? "Claiming…"
              : `✶ Claim month ${myUnclaimedWinMonth} winnings`}
          </button>
        )}
      </div>
    </>
  );
}
