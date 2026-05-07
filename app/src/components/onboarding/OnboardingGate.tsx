"use client";

import { useState } from "react";
import { toast } from "sonner";

import { PoolverMark } from "@/components/brand/PoolverLogo";
import { useOnboarding } from "@/hooks/useOnboarding";
import { usePoolver } from "@/providers/PoolverProvider";

type Busy = "reputation" | "kyc" | "faucet" | null;

export function OnboardingGate() {
  const { connected } = usePoolver();
  const { state, ensureReputation, ensureKyc, faucet, refetch } =
    useOnboarding();
  const [busy, setBusy] = useState<Busy>(null);
  const [dismissed, setDismissed] = useState(false);

  if (!connected) return null;
  if (state === "ready" || state === "loading" || state === "disconnected") {
    return null;
  }
  if (dismissed) return null;

  const handleReputation = async () => {
    setBusy("reputation");
    const toastId = toast.loading("Initializing reputation account…");
    try {
      const sig = await ensureReputation();
      toast.success("Reputation initialized", {
        id: toastId,
        description: `sig: ${sig.slice(0, 12)}…`,
      });
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      toast.error("Failed", { id: toastId, description: msg.slice(0, 200) });
    } finally {
      setBusy(null);
    }
  };

  const handleKyc = async () => {
    setBusy("kyc");
    const toastId = toast.loading("Issuing demo KYC…");
    try {
      const { signature, idempotent } = await ensureKyc();
      toast.success(
        idempotent ? "KYC already issued" : "Demo KYC granted",
        {
          id: toastId,
          description: signature
            ? `sig: ${signature.slice(0, 12)}…`
            : undefined,
        }
      );
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      toast.error("KYC failed", {
        id: toastId,
        description: msg.slice(0, 200),
      });
    } finally {
      setBusy(null);
    }
  };

  const handleFaucet = async () => {
    setBusy("faucet");
    const toastId = toast.loading("Minting test USDC…");
    try {
      const { signature, amount } = await faucet(5000);
      toast.success(`+${amount.toLocaleString()} USDC`, {
        id: toastId,
        description: `sig: ${signature.slice(0, 12)}…`,
      });
      await refetch();
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      toast.error("Faucet failed", {
        id: toastId,
        description: msg.slice(0, 200),
      });
    } finally {
      setBusy(null);
    }
  };

  const title =
    state === "needs_reputation"
      ? "Initialize your account"
      : "Verify identity (demo KYC)";
  const subtitle =
    state === "needs_reputation"
      ? "Create your on-chain reputation PDA. Sign once with your wallet — it's a single transaction. Required before joining any pool."
      : "Devnet uses a mock KYC oracle. Click below; an admin server-side route signs `mock_issue_kyc` to grant Full KYC to your wallet. Production swaps this for Idwall/Sumsub.";

  return (
    <div className="onboarding-gate">
      <div className="onboarding-gate-card">
        <button
          className="onboarding-gate-close"
          onClick={() => setDismissed(true)}
          aria-label="Dismiss"
        >
          ×
        </button>
        <div className="onboarding-gate-head">
          <PoolverMark size={28} />
          <div>
            <div
              style={{
                fontFamily: "var(--mono)",
                fontSize: 10.5,
                color: "var(--fg-4)",
                letterSpacing: "0.18em",
                textTransform: "uppercase",
                marginBottom: 4,
              }}
            >
              ONBOARDING ·{" "}
              {state === "needs_reputation" ? "STEP 1 OF 2" : "STEP 2 OF 2"}
            </div>
            <h3 style={{ margin: 0, fontSize: 18, color: "var(--fg)" }}>
              {title}
            </h3>
          </div>
        </div>
        <p
          style={{
            color: "var(--fg-2)",
            fontSize: 13.5,
            lineHeight: 1.6,
            margin: "12px 0 18px",
          }}
        >
          {subtitle}
        </p>
        <div style={{ display: "flex", gap: 10, flexWrap: "wrap" }}>
          {state === "needs_reputation" && (
            <button
              className="btn primary"
              onClick={handleReputation}
              disabled={busy !== null}
            >
              {busy === "reputation" ? "Signing…" : "▶ Initialize account"}
            </button>
          )}
          {state === "needs_kyc" && (
            <>
              <button
                className="btn primary"
                onClick={handleKyc}
                disabled={busy !== null}
              >
                {busy === "kyc" ? "Issuing…" : "▶ Verify (demo KYC)"}
              </button>
              <button
                className="btn"
                onClick={handleFaucet}
                disabled={busy !== null}
              >
                {busy === "faucet" ? "Minting…" : "💧 Get 5,000 test USDC"}
              </button>
            </>
          )}
          <button
            className="btn ghost"
            onClick={() => setDismissed(true)}
            disabled={busy !== null}
          >
            Later
          </button>
        </div>
        <div
          style={{
            marginTop: 16,
            padding: 10,
            border: "1px dashed var(--line-2)",
            borderRadius: 3,
            fontFamily: "var(--mono)",
            fontSize: 10.5,
            color: "var(--fg-4)",
            lineHeight: 1.6,
          }}
        >
          {state === "needs_reputation"
            ? "◆ Your wallet signs `initialize_user_reputation`. Costs ≈ 0.002 SOL rent."
            : "◆ Server-side admin signs `mock_issue_kyc`. No tx fee for you."}
        </div>
      </div>
    </div>
  );
}
