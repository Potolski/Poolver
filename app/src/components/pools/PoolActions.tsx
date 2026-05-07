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

  const submitClaim = async () => {
    const ix = await claimWinningIx(client, {
      pool: pool.publicKey,
      tier: pool.tier,
      usdcMint: USDC_MINT_DEVNET_DEFAULT,
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
  const isWinner =
    participant?.hasWon &&
    participant.winMonth === currentMonth &&
    !(participant.raw as { claimed?: boolean })?.claimed;

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

  return (
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
      {isWinner && (
        <button
          className="btn primary lg"
          disabled={busy !== null}
          onClick={() => run("claim", "Claiming winnings", submitClaim)}
        >
          {busy === "claim" ? "Claiming…" : "✶ Claim winnings"}
        </button>
      )}
    </div>
  );
}
