#!/usr/bin/env npx tsx
/**
 * Resume a pool that's already running and walk it through to completion
 * with the same partial-pay pattern as `simulate-pool.ts`. Call this on
 * any pool whose `current_month >= 1 && !is_complete` to fast-forward.
 *
 * Usage:
 *   npx tsx scripts/continue-pool.ts \
 *     --pool <pubkey> \
 *     --wallets .deploy-recovery/sim-wallets-...json \
 *     --rpc "https://devnet.helius-rpc.com/?api-key=..." \
 *     --wallet ./deploy-keypair.json
 */
import {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  TransactionMessage,
  VersionedTransaction,
} from "@solana/web3.js";
import { AnchorProvider, BN, Wallet } from "@coral-xyz/anchor";
import { readFileSync } from "fs";
import { resolve } from "path";

import {
  PoolverClient,
  POOLVER_ALT_DEVNET,
  USDC_MINT_DEVNET_DEFAULT,
  advanceMonthIx,
  contributeIx,
  fetchPool,
  selectWinnerIx,
  slashUnpaidIx,
} from "../client/src";

interface Args {
  rpc: string;
  wallet: string;
  pool: string;
  wallets: string;
}

function parseArgs(): Args {
  const get = (k: string): string | undefined => {
    const i = process.argv.indexOf(`--${k}`);
    return i >= 0 ? process.argv[i + 1] : undefined;
  };
  const pool = get("pool");
  const wallets = get("wallets");
  if (!pool) throw new Error("missing --pool");
  if (!wallets) throw new Error("missing --wallets");
  return {
    rpc: get("rpc") ?? "https://api.devnet.solana.com",
    wallet: resolve(get("wallet") ?? "./deploy-keypair.json"),
    pool,
    wallets: resolve(wallets),
  };
}

function loadKeypair(path: string): Keypair {
  const json = JSON.parse(readFileSync(path, "utf8"));
  return Keypair.fromSecretKey(Uint8Array.from(json));
}

function loadWallets(path: string): Keypair[] {
  const data = JSON.parse(readFileSync(path, "utf8"));
  return data.wallets.map((w: { secret: number[] }) =>
    Keypair.fromSecretKey(Uint8Array.from(w.secret))
  );
}

function sleep(ms: number): Promise<void> {
  return new Promise((res) => setTimeout(res, ms));
}

function pickPayers(
  rng: () => number,
  good: number[],
  flaky: number[],
  bad: number[]
): number[] {
  const out: number[] = [];
  for (const i of good) out.push(i);
  for (const i of flaky) if (rng() < 0.6) out.push(i);
  for (const i of bad) if (rng() < 0.3) out.push(i);
  return out;
}

function mulberry32(seed: number) {
  let t = seed;
  return () => {
    t |= 0;
    t = (t + 0x6d2b79f5) | 0;
    let r = Math.imul(t ^ (t >>> 15), 1 | t);
    r = (r + Math.imul(r ^ (r >>> 7), 61 | r)) ^ r;
    return ((r ^ (r >>> 14)) >>> 0) / 4294967296;
  };
}

async function main() {
  const args = parseArgs();
  const conn = new Connection(args.rpc, "confirmed");
  const admin = loadKeypair(args.wallet);
  const adminProvider = new AnchorProvider(conn, new Wallet(admin), {
    commitment: "confirmed",
  });
  const adminClient = new PoolverClient(adminProvider);

  const wallets = loadWallets(args.wallets);
  const poolPk = new PublicKey(args.pool);

  console.log("admin: ", admin.publicKey.toBase58());
  console.log("pool:  ", poolPk.toBase58());
  console.log("wallets file:", args.wallets);

  const goodIndices = [0, 1, 2, 3, 4, 5];
  const flakyIndices = [6, 7, 8, 9];
  const badIndices = [10, 11];
  // Different RNG seed than simulate-pool.ts so the continuation isn't
  // a deterministic rerun of months we already saw.
  const rng = mulberry32(0xC07C0F);

  while (true) {
    const probe = await fetchPool(adminClient, poolPk);
    if (probe.isComplete) {
      console.log("\n✓ pool is_complete=true — done");
      break;
    }
    const m = probe.currentMonth;
    if (m < 1) {
      console.log("pool not started yet (current_month=0) — abort");
      break;
    }

    const payers = pickPayers(rng, goodIndices, flakyIndices, badIndices);
    const skippers = wallets
      .map((_, i) => i)
      .filter((i) => !payers.includes(i));

    console.log(`\n══ MONTH ${m} ══`);
    console.log(`  paying  (${payers.length})`);
    console.log(`  skipping(${skippers.length})`);

    for (const i of payers) {
      const kp = wallets[i];
      const userProvider = new AnchorProvider(conn, new Wallet(kp), {
        commitment: "confirmed",
      });
      const userClient = new PoolverClient(userProvider);
      try {
        const ix = await contributeIx(userClient, {
          pool: poolPk,
          tier: probe.tier,
          usdcMint: USDC_MINT_DEVNET_DEFAULT,
        });
        await userProvider.sendAndConfirm!(new Transaction().add(ix), [], {
          commitment: "confirmed",
        });
      } catch {
        // Already paid (e.g. month-1 join collateral) or already won —
        // ignore; we only care about coverage of THIS month's pot.
      }
    }

    // Wait until both month_end and reveal_window_ends_at have passed.
    {
      const p = await fetchPool(adminClient, poolPk);
      const monthEndSec =
        p.currentMonthStartedAt.toNumber() + p.monthDurationSeconds.toNumber();
      const revealEndSec = p.revealWindowEndsAt.toNumber();
      const targetSec = Math.max(monthEndSec, revealEndSec) + 5;
      const waitMs = Math.max(0, targetSec * 1000 - Date.now());
      console.log(`  ⏱  waiting ${(waitMs / 1000).toFixed(0)}s…`);
      await sleep(waitMs);
    }

    // Slash unpaid.
    if (skippers.length > 0) {
      console.log(`  ⚠ slashing ${skippers.length} unpaid…`);
      for (const i of skippers) {
        try {
          const ix = await slashUnpaidIx(adminClient, {
            pool: poolPk,
            delinquent: wallets[i].publicKey,
            tier: probe.tier,
          });
          await adminProvider.sendAndConfirm!(
            new Transaction().add(ix),
            [],
            { commitment: "confirmed" }
          );
        } catch {
          // Already slashed for this month or already paid — fine.
        }
      }
    }

    // Build candidate list (non-bidders, no-prior-win).
    const refresh = await fetchPool(adminClient, poolPk);
    const winnersSet = new Set<string>();
    const winnersArr = (
      refresh.raw as {
        winners: Array<{ winner: PublicKey; selectedAt: BN }>;
      }
    ).winners;
    for (const w of winnersArr) {
      if (w.selectedAt.gtn(0)) winnersSet.add(w.winner.toBase58());
    }
    const eligibleNonBidders: PublicKey[] = [];
    for (const kp of wallets) {
      if (winnersSet.has(kp.publicKey.toBase58())) continue;
      eligibleNonBidders.push(kp.publicKey);
    }

    try {
      const selectIx = await selectWinnerIx(adminClient, {
        pool: poolPk,
        tier: "vault",
        month: m,
        bidders: [],
        nonBidders: eligibleNonBidders,
      });
      const blockhash = (await conn.getLatestBlockhash()).blockhash;
      const alt = (await conn.getAddressLookupTable(POOLVER_ALT_DEVNET))
        .value;
      const message = new TransactionMessage({
        payerKey: admin.publicKey,
        recentBlockhash: blockhash,
        instructions: [selectIx],
      }).compileToV0Message(alt ? [alt] : []);
      const tx = new VersionedTransaction(message);
      tx.sign([admin]);
      const sig = await conn.sendRawTransaction(tx.serialize());
      await conn.confirmTransaction(sig, "confirmed");
      console.log(`  ▶ winner drawn (sig ${sig.slice(0, 12)}…)`);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      console.warn(`  select_winner failed: ${msg.slice(0, 200)}`);
    }

    try {
      const advIx = await advanceMonthIx(adminClient, { pool: poolPk });
      await adminProvider.sendAndConfirm!(new Transaction().add(advIx), [], {
        commitment: "confirmed",
      });
      console.log(`  ↯ advanced from month ${m}`);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      console.warn(`  advance_month failed: ${msg.slice(0, 200)}`);
    }
  }

  console.log("\n✓ continuation done");
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
