"use client";

import { useEffect, useState } from "react";
import { Connection, PublicKey } from "@solana/web3.js";
import {
  getAssociatedTokenAddressSync,
  getAccount,
  TokenAccountNotFoundError,
} from "@solana/spl-token";
import { USDC_MINT_DEVNET_DEFAULT } from "@poolver/client";

import { usePoolver } from "@/providers/PoolverProvider";
import { DEVNET_RPC } from "@/lib/constants";

const USDC_DECIMALS = 6;
const POLL_MS = 12_000;

function fmt(amount: bigint): string {
  const human = Number(amount) / 10 ** USDC_DECIMALS;
  if (human >= 1000) return `${(human / 1000).toFixed(human >= 10_000 ? 0 : 1)}K`;
  return human.toLocaleString(undefined, { maximumFractionDigits: 2 });
}

/**
 * Live USDC balance for the connected wallet, polled every POLL_MS.
 * Sits in the topbar next to the faucet button so users always see
 * how much they have to play with.
 */
export function UsdcBalance() {
  const { connected, publicKey } = usePoolver();
  const [balance, setBalance] = useState<bigint | null>(null);

  useEffect(() => {
    if (!connected || !publicKey) {
      setBalance(null);
      return;
    }
    const conn = new Connection(DEVNET_RPC, "confirmed");
    const ata = getAssociatedTokenAddressSync(USDC_MINT_DEVNET_DEFAULT, publicKey);
    let cancelled = false;

    const tick = async () => {
      try {
        const acc = await getAccount(conn, ata);
        if (!cancelled) setBalance(acc.amount);
      } catch (e) {
        if (e instanceof TokenAccountNotFoundError) {
          if (!cancelled) setBalance(BigInt(0));
        }
        // network errors: leave balance as-is
      }
    };

    void tick();
    const id = setInterval(tick, POLL_MS);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, [connected, publicKey]);

  if (!connected || !publicKey) return null;

  return (
    <span
      className="usdc-balance"
      title={
        balance === null
          ? "USDC balance"
          : `${(Number(balance) / 10 ** USDC_DECIMALS).toLocaleString()} USDC`
      }
    >
      <span className="usdc-balance-label">USDC</span>
      <span className="usdc-balance-value">
        {balance === null ? "…" : fmt(balance)}
      </span>
    </span>
  );
}
