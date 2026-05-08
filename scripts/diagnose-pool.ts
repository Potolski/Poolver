#!/usr/bin/env npx tsx
/**
 * Read-only diagnostic for a pool stuck on claim_winning.
 *
 * Compares the on-chain `MonthWinner.gross_payout` against the actual
 * USDC liquidity in the yield-adapter vault + pool USDC vault to pinpoint
 * the "Error processing Instruction 2: 0x1771" (= adapter
 * InsufficientLiquidity) root cause.
 *
 * Usage:
 *   npx tsx scripts/diagnose-pool.ts \
 *     --pool <pool_pubkey> \
 *     --rpc "https://devnet.helius-rpc.com/?api-key=..."
 */
import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { AnchorProvider, BN, Wallet } from "@coral-xyz/anchor";
import { getAccount } from "@solana/spl-token";
import { readFileSync } from "fs";
import { resolve } from "path";

import {
  PoolverClient,
  POOLVER_YIELD_VAULT_PROGRAM_ID,
  POOLVER_YIELD_DEFI_PROGRAM_ID,
  microUsdcToHuman,
  fetchPool,
} from "../client/src";

function get(k: string): string | undefined {
  const i = process.argv.indexOf(k);
  return i >= 0 ? process.argv[i + 1] : undefined;
}

function fmt(microBn: BN): string {
  return `${Number(microUsdcToHuman(microBn)).toLocaleString()} USDC`;
}

async function main() {
  const poolArg = get("--pool");
  if (!poolArg) {
    console.error("missing --pool <address>");
    process.exit(1);
  }
  const rpc = get("--rpc") ?? "https://api.devnet.solana.com";
  const conn = new Connection(rpc, "confirmed");

  // Read-only client (random throwaway keypair as wallet).
  const provider = new AnchorProvider(
    conn,
    new Wallet(Keypair.generate()),
    { commitment: "confirmed" }
  );
  const client = new PoolverClient(provider);

  const poolPk = new PublicKey(poolArg);
  const pool = await fetchPool(client, poolPk);

  console.log("\n══ POOL ══");
  console.log("address:           ", poolPk.toBase58());
  console.log("tier:              ", pool.tier);
  console.log("current_month:     ", pool.currentMonth, "/", pool.totalMonths);
  console.log("contribution:      ", fmt(pool.contributionAmount), "/ user / month");
  console.log("paid this month:   ", `${pool.paidCountForCurrentMonth}/12`);
  console.log("is_complete:       ", pool.isComplete);

  // Fetch USDC token accounts.
  const poolUsdcVault = (pool.raw as { poolUsdcVault: PublicKey }).poolUsdcVault;
  const collateralVault = (pool.raw as { collateralVault: PublicKey }).collateralVault;
  // adapter_usdc_vault is a child PDA of the yield-adapter program.
  // Seeds = [VAULT_ADAPTER_USDC_SEED | DEFI_ADAPTER_USDC_SEED, pool].
  const adapterProgramId =
    pool.tier === "vault"
      ? POOLVER_YIELD_VAULT_PROGRAM_ID
      : POOLVER_YIELD_DEFI_PROGRAM_ID;
  const adapterUsdcSeed = Buffer.from(
    pool.tier === "vault" ? "vault_adapter_usdc" : "defi_adapter_usdc"
  );
  const [adapterUsdcVault] = PublicKey.findProgramAddressSync(
    [adapterUsdcSeed, poolPk.toBuffer()],
    adapterProgramId
  );

  console.log("\n══ TOKEN VAULTS ══");
  for (const [label, pk] of [
    ["pool_usdc_vault   ", poolUsdcVault],
    ["collateral_vault  ", collateralVault],
    ["adapter_usdc_vault", adapterUsdcVault],
  ] as const) {
    try {
      const acct = await getAccount(conn, pk);
      console.log(
        `${label}: ${fmt(new BN(acct.amount.toString()))}  (${pk.toBase58()})`
      );
    } catch (e) {
      console.log(`${label}: <missing>  (${pk.toBase58()})`);
    }
  }

  // Walk MonthWinners to find any unclaimed and check whether the
  // adapter has enough liquidity for each gross_payout.
  console.log("\n══ MONTH WINNERS ══");
  const winnersArr = (pool.raw as {
    winners: Array<{
      month: number;
      winner: PublicKey;
      winningBid: BN;
      grossPayout: BN;
      netPayout: BN;
      selectedAt: BN;
      claimed: boolean;
      selectionMethod: { bid?: object; lottery?: object };
    }>;
  }).winners;

  let adapterBalance = new BN(0);
  try {
    const a = await getAccount(conn, adapterUsdcVault);
    adapterBalance = new BN(a.amount.toString());
  } catch {}

  for (let i = 0; i < winnersArr.length; i++) {
    const w = winnersArr[i];
    if (w.month === 0 || w.selectedAt.eqn?.(0)) {
      console.log(
        `M${String(i + 1).padStart(2, "0")}: <no winner drawn>`
      );
      continue;
    }
    const method = "bid" in w.selectionMethod ? "BID    " : "LOTTERY";
    const gross = w.grossPayout;
    const net = w.netPayout;
    const enoughInAdapter = adapterBalance.gte(gross);
    const flag = w.claimed
      ? "✓ claimed     "
      : enoughInAdapter
        ? "○ pending     "
        : "⚠ ADAPTER SHORT";
    console.log(
      `M${String(w.month).padStart(2, "0")} ${method} ${flag}  ` +
        `gross=${fmt(gross)}  net=${fmt(net)}  winner=${w.winner.toBase58().slice(0, 8)}…`
    );
    if (!w.claimed && !enoughInAdapter) {
      const gap = gross.sub(adapterBalance);
      console.log(
        `         needs ${fmt(gross)} from adapter, has ${fmt(adapterBalance)} → SHORT BY ${fmt(gap)}`
      );
    }
  }

  // Expected vs actual contributions.
  console.log("\n══ EXPECTED VS ACTUAL CONTRIBUTIONS ══");
  console.log(
    "Per-month full-pool contribution:",
    fmt(pool.contributionAmount.muln(12))
  );
  // Cumulative net (after fees) that SHOULD be in adapter if every
  // participant paid every month [1..current_month - 1] AND no claims yet.
  // Net per contribution = contribution × (10000 - fee_bps - reserve_bps) / 10000.
  const feeBps = pool.tier === "vault" ? 150 + 150 : 150 + 250;
  const netPerContribution = pool.contributionAmount
    .muln(10_000 - feeBps)
    .divn(10_000);
  const cumulativeNetIfFull = netPerContribution
    .muln(12) // 12 participants
    .muln(Math.max(0, pool.currentMonth));
  console.log(
    "Cumulative net deposit if all 12 paid each month so far:",
    fmt(cumulativeNetIfFull)
  );
  console.log("Actual adapter balance:                 ", fmt(adapterBalance));
  if (adapterBalance.lt(cumulativeNetIfFull)) {
    console.log(
      "→ shortfall:",
      fmt(cumulativeNetIfFull.sub(adapterBalance)),
      "(missing contributions or already-claimed payouts)"
    );
  }

  console.log();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
