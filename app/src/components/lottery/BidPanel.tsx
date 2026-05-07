"use client";

import { useEffect, useState } from "react";
import { toast } from "sonner";
import BN from "bn.js";
import {
  BID_CAP_BPS,
  BPS_DENOMINATOR,
  commitBidIx,
  humanUsdcToMicro,
  microUsdcToHuman,
  revealBidIx,
  type ParticipantView,
  type PoolMonthState,
  type PoolView,
} from "@poolver/client";

import { usePoolver } from "@/providers/PoolverProvider";
import { sendIxs } from "@/lib/tx-helpers";
import { USDC_MINT_DEVNET_DEFAULT } from "@/lib/constants";
import {
  bidKey,
  clearBidSecret,
  computeBidCommitHash,
  generateBidNonce,
  loadBidSecret,
  saveBidSecret,
  type BidSecret,
} from "@/lib/bid-storage";

interface Props {
  pool: PoolView;
  participant: ParticipantView | null;
  monthState: PoolMonthState | null;
  onRefresh: () => Promise<void>;
}

function computeBidCapHuman(pool: PoolView): number {
  const grossMicro = pool.contributionAmount.muln(pool.totalMonths);
  // Reserve fee differs Vault/DeFi but bid cap is approximated client-side;
  // chain enforces the exact value.
  const reserveBps = pool.tier === "vault" ? 150 : 250;
  const protocolBps = 150;
  const netBps = BPS_DENOMINATOR - reserveBps - protocolBps;
  const netMicro = grossMicro.muln(netBps).divn(BPS_DENOMINATOR);
  const capMicro = netMicro.muln(BID_CAP_BPS).divn(BPS_DENOMINATOR);
  return Number(microUsdcToHuman(capMicro));
}

export function BidPanel({ pool, participant, monthState, onRefresh }: Props) {
  const { client, connected, publicKey } = usePoolver();
  const month = monthState?.currentMonth ?? pool.currentMonth;
  const inBid = monthState?.inBidWindow ?? false;
  const inReveal = monthState?.inRevealWindow ?? false;

  const [savedSecret, setSavedSecret] = useState<BidSecret | null>(null);
  const [secretLoading, setSecretLoading] = useState(true);
  const [bidAmount, setBidAmount] = useState<number>(0);
  const [busy, setBusy] = useState<"commit" | "reveal" | null>(null);

  const cap = computeBidCapHuman(pool);

  useEffect(() => {
    let cancelled = false;
    setSecretLoading(true);
    if (!publicKey || month <= 0) {
      setSavedSecret(null);
      setSecretLoading(false);
      return;
    }
    const key = bidKey(pool.publicKey, month, publicKey);
    loadBidSecret(key)
      .then((s) => {
        if (!cancelled) setSavedSecret(s);
      })
      .finally(() => {
        if (!cancelled) setSecretLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [pool.publicKey, month, publicKey]);

  const submitCommit = async () => {
    if (!publicKey) return;
    if (bidAmount <= 0) {
      toast.error("Bid amount must be > 0");
      return;
    }
    if (bidAmount > cap) {
      toast.error(`Bid above cap ($${Math.floor(cap).toLocaleString()})`);
      return;
    }
    setBusy("commit");
    const toastId = toast.loading("Preparing commit…");
    try {
      const nonce = generateBidNonce();
      const bidMicro = humanUsdcToMicro(bidAmount);
      const commitHash = await computeBidCommitHash(bidMicro, nonce, publicKey);
      const key = bidKey(pool.publicKey, month, publicKey);
      const secret: BidSecret = {
        key,
        poolAddress: pool.publicKey.toBase58(),
        month,
        userPubkey: publicKey.toBase58(),
        nonce: Array.from(nonce),
        bidAmountMicro: bidMicro.toString(),
        commitHash: Array.from(commitHash),
        committedAt: Date.now(),
      };
      // Persist BEFORE signing — handoff §9.2.
      await saveBidSecret(secret);

      const { ix } = await commitBidIx(client, {
        pool: pool.publicKey,
        month,
        usdcMint: USDC_MINT_DEVNET_DEFAULT,
        commitHash,
      });
      const sig = await sendIxs(client, [ix]);
      secret.signature = sig;
      await saveBidSecret(secret);
      setSavedSecret(secret);
      toast.success("Bid committed", {
        id: toastId,
        description: `sig: ${sig.slice(0, 12)}…`,
      });
      await onRefresh();
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      toast.error("Commit failed", { id: toastId, description: msg.slice(0, 200) });
    } finally {
      setBusy(null);
    }
  };

  const submitReveal = async () => {
    if (!publicKey || !savedSecret) return;
    setBusy("reveal");
    const toastId = toast.loading("Revealing bid…");
    try {
      const nonce = new Uint8Array(savedSecret.nonce);
      const ix = await revealBidIx(client, {
        pool: pool.publicKey,
        month,
        bidAmount: new BN(savedSecret.bidAmountMicro),
        nonce,
        usdcMint: USDC_MINT_DEVNET_DEFAULT,
      });
      const sig = await sendIxs(client, [ix]);
      await clearBidSecret(savedSecret.key);
      setSavedSecret(null);
      toast.success("Bid revealed", {
        id: toastId,
        description: `sig: ${sig.slice(0, 12)}…`,
      });
      await onRefresh();
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      toast.error("Reveal failed", { id: toastId, description: msg.slice(0, 200) });
    } finally {
      setBusy(null);
    }
  };

  // Render decisions:
  // - not connected → CTA
  // - month 0 (forming) → disabled message
  // - inBid → form
  // - inReveal AND saved → reveal button
  // - inReveal AND no saved → "No bid for this month"
  // - otherwise → "Window closed"
  const isConnected = connected && publicKey;
  const isParticipant = !!participant;

  return (
    <div className="bid-grid">
      <div className="card">
        <div className="kicker">
          LANCE · MONTH {String(month).padStart(2, "0")}
        </div>
        <h3>Sealed-bid auction</h3>
        <p>
          Commit a sha256 hash of your bid + nonce in the bid window; reveal in
          the reveal window. Highest revealed bid wins this month&apos;s pot.
          Stake (1%) refunded on successful reveal.
        </p>
        {!isConnected && (
          <div
            style={{
              padding: 14,
              border: "1px dashed var(--line-2)",
              borderRadius: 3,
              fontSize: 12,
              color: "var(--fg-3)",
              fontFamily: "var(--mono)",
            }}
          >
            Connect a wallet to bid.
          </div>
        )}
        {isConnected && !isParticipant && (
          <div
            style={{
              padding: 14,
              border: "1px dashed var(--line-2)",
              borderRadius: 3,
              fontSize: 12,
              color: "var(--fg-3)",
              fontFamily: "var(--mono)",
            }}
          >
            Join the pool first to participate in the auction.
          </div>
        )}

        {isConnected && isParticipant && month === 0 && (
          <div
            style={{
              padding: 14,
              border: "1px dashed var(--line-2)",
              borderRadius: 3,
              fontSize: 12,
              color: "var(--fg-3)",
              fontFamily: "var(--mono)",
            }}
          >
            Pool still forming. Bidding opens at month 1.
          </div>
        )}

        {isConnected && isParticipant && month > 0 && inBid && !savedSecret && (
          <>
            <div className="field">
              <label>Your bid · USDC</label>
              <input
                type="range"
                min={0}
                max={Math.max(100, Math.floor(cap))}
                step={50}
                value={bidAmount}
                onChange={(e) => setBidAmount(Number(e.target.value))}
              />
              <div className="slider-vals">
                <span>$0</span>
                <span className="mid">${bidAmount.toLocaleString()}</span>
                <span>${Math.floor(cap).toLocaleString()}</span>
              </div>
            </div>
            <div
              style={{
                fontSize: 10.5,
                color: "var(--fg-4)",
                marginBottom: 12,
                fontFamily: "var(--mono)",
              }}
            >
              cap = 20% × net pot · stake = 1% × contribution (refunded on reveal)
            </div>
            <button
              className="btn primary"
              disabled={busy !== null}
              onClick={submitCommit}
            >
              {busy === "commit" ? "Signing…" : "▶ Commit bid"}
            </button>
          </>
        )}

        {isConnected && isParticipant && month > 0 && inBid && savedSecret && (
          <div
            style={{
              padding: 14,
              border: "1px solid var(--acc)",
              borderRadius: 3,
              fontSize: 12.5,
              color: "var(--fg-2)",
              fontFamily: "var(--mono)",
              lineHeight: 1.6,
            }}
          >
            ✓ Bid committed for month {month}. Wait for reveal window.
            <br />
            <span style={{ color: "var(--fg-4)" }}>
              Amount stored locally: $
              {Number(microUsdcToHuman(new BN(savedSecret.bidAmountMicro))).toLocaleString()}
            </span>
          </div>
        )}

        {isConnected && isParticipant && month > 0 && inReveal && (
          <>
            {savedSecret ? (
              <>
                <div
                  style={{
                    padding: 14,
                    border: "1px solid var(--acc)",
                    borderRadius: 3,
                    fontSize: 12.5,
                    color: "var(--fg-2)",
                    fontFamily: "var(--mono)",
                    marginBottom: 14,
                    lineHeight: 1.6,
                  }}
                >
                  Saved bid: $
                  {Number(microUsdcToHuman(new BN(savedSecret.bidAmountMicro))).toLocaleString()}{" "}
                  USDC
                  <br />
                  Reveal before window closes or stake is forfeited.
                </div>
                <button
                  className="btn primary"
                  disabled={busy !== null}
                  onClick={submitReveal}
                >
                  {busy === "reveal" ? "Signing…" : "✶ Reveal bid"}
                </button>
              </>
            ) : (
              <div
                style={{
                  padding: 14,
                  border: "1px dashed var(--line-2)",
                  borderRadius: 3,
                  fontSize: 12,
                  color: "var(--fg-3)",
                  fontFamily: "var(--mono)",
                }}
              >
                {secretLoading
                  ? "Loading saved bid…"
                  : "No bid recorded for this month on this device."}
              </div>
            )}
          </>
        )}

        {isConnected && isParticipant && month > 0 && !inBid && !inReveal && (
          <div
            style={{
              padding: 14,
              border: "1px dashed var(--line-2)",
              borderRadius: 3,
              fontSize: 12,
              color: "var(--fg-3)",
              fontFamily: "var(--mono)",
            }}
          >
            Bid + reveal windows closed. Awaiting <code>select_winner</code>.
          </div>
        )}
      </div>

      <div className="card">
        <div className="kicker">YOUR STATUS</div>
        <h3 style={{ marginBottom: 14 }}>Month {month}</h3>
        <div className="kv-list" style={{ fontSize: 12 }}>
          <div className="kv-row">
            <span className="k">Bid window</span>
            <span className="v" style={{ color: inBid ? "var(--acc)" : "var(--fg-3)" }}>
              {inBid ? "OPEN" : "—"}
            </span>
          </div>
          <div className="kv-row">
            <span className="k">Reveal window</span>
            <span
              className="v"
              style={{ color: inReveal ? "var(--acc)" : "var(--fg-3)" }}
            >
              {inReveal ? "OPEN" : "—"}
            </span>
          </div>
          <div className="kv-row">
            <span className="k">Bid cap</span>
            <span className="v">${Math.floor(cap).toLocaleString()}</span>
          </div>
          <div className="kv-row">
            <span className="k">Saved commit</span>
            <span className="v">
              {savedSecret
                ? `$${Number(microUsdcToHuman(new BN(savedSecret.bidAmountMicro))).toLocaleString()}`
                : "—"}
            </span>
          </div>
        </div>
        <div
          style={{
            marginTop: 14,
            padding: 14,
            border: "1px dashed var(--line-2)",
            borderRadius: 3,
            background: "var(--bg-1)",
            fontSize: 11,
            color: "var(--fg-3)",
            fontFamily: "var(--mono)",
            lineHeight: 1.6,
          }}
        >
          ⚠ Reveal must happen on the same browser/profile that placed the
          commit. Nonce is stored in IndexedDB.
        </div>
      </div>
    </div>
  );
}
