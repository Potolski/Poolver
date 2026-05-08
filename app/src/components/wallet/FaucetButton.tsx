"use client";

import { useState } from "react";
import { toast } from "sonner";
import { usePoolver } from "@/providers/PoolverProvider";

const DEFAULT_AMOUNT = 100_000;

/**
 * Small "💧 USDC" button visible in the topbar whenever a wallet is
 * connected. Mints DEFAULT_AMOUNT mock USDC to the connected wallet's
 * ATA. Different from the OnboardingGate's faucet button (which
 * disappears once KYC is done) — this one stays available for the
 * entire session so demo participants can top up.
 */
export function FaucetButton() {
  const { connected, publicKey } = usePoolver();
  const [busy, setBusy] = useState(false);

  if (!connected || !publicKey) return null;

  const handle = async () => {
    setBusy(true);
    const id = toast.loading(`Minting ${DEFAULT_AMOUNT.toLocaleString()} test USDC…`);
    try {
      const res = await fetch("/api/faucet", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          recipient: publicKey.toBase58(),
          amount: DEFAULT_AMOUNT,
        }),
      });
      const data = (await res.json()) as
        | { signature: string; amount: number }
        | { error: string; message?: string };
      if (!res.ok || !("signature" in data)) {
        const msg =
          ("message" in data && data.message) ||
          ("error" in data && data.error) ||
          "faucet_failed";
        throw new Error(msg);
      }
      toast.success(`+${data.amount.toLocaleString()} USDC`, {
        id,
        description: `sig: ${data.signature.slice(0, 12)}…`,
      });
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      toast.error("Faucet failed", { id, description: msg.slice(0, 200) });
    } finally {
      setBusy(false);
    }
  };

  return (
    <button
      type="button"
      onClick={handle}
      disabled={busy}
      className="faucet-button"
      title={`Mint ${DEFAULT_AMOUNT.toLocaleString()} test USDC to your wallet`}
      aria-label="Get test USDC"
    >
      <span className="faucet-icon">💧</span>
      <span className="faucet-label">{busy ? "Minting…" : "USDC"}</span>
    </button>
  );
}
