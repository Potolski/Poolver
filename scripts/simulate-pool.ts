#!/usr/bin/env npx tsx
/**
 * End-to-end pool simulation that walks 6 months of partial
 * contributions and leaves the pool in a "month-7 ended with N unpaid"
 * state so the UI can demo the slash flow + the spread of reputation
 * tiers (green / yellow / red).
 *
 * Wallet personas (12 total, indices 0..11):
 *   0..5  → "good payers" — pay every month
 *   6..9  → "flaky payers" — 60% chance per month
 *   10,11 → "bad payers"   — 30% chance per month
 *
 * After 6 fully-resolved months (slash + draw + advance), month 7 is
 * left mid-flight: a random subset has paid, the rest haven't, and
 * the month duration has elapsed so the auto-slash flow on the pool
 * page kicks in immediately when an admin opens it.
 *
 * Pool config:
 *   contribution_amount    = 4,167 USDC / month / user
 *   monthly_pot (gross)    = 12 × net_contribution ≈ 50K USDC
 *   month_duration_seconds = 120 (2 minutes — fast demo)
 *   tier                   = vault
 *
 * Usage:
 *   npx tsx scripts/simulate-pool.ts \
 *     --rpc "https://devnet.helius-rpc.com/?api-key=..." \
 *     --wallet ./deploy-keypair.json
 */
import {
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionMessage,
  VersionedTransaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import {
  createAssociatedTokenAccountIdempotentInstruction,
  createMintToInstruction,
  getAssociatedTokenAddressSync,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { AnchorProvider, BN, Wallet } from "@coral-xyz/anchor";
import { mkdirSync, readFileSync, writeFileSync } from "fs";
import { resolve } from "path";

import {
  PoolverClient,
  POOLVER_ALT_DEVNET,
  USDC_MINT_DEVNET_DEFAULT,
  advanceMonthIx,
  contributeIx,
  createPoolIx,
  fetchPool,
  initializeUserReputationIx,
  joinPoolIx,
  mockIssueKycIx,
  selectWinnerIx,
  slashUnpaidIx,
  type TierName,
} from "../client/src";

interface Args {
  rpc: string;
  wallet: string;
  monthDurationSecs: number;
  tier: TierName;
}

function parseArgs(): Args {
  const get = (k: string): string | undefined => {
    const i = process.argv.indexOf(`--${k}`);
    return i >= 0 ? process.argv[i + 1] : undefined;
  };
  const tierArg = (get("tier") ?? "vault").toLowerCase();
  if (tierArg !== "vault" && tierArg !== "defi") {
    throw new Error(`--tier must be "vault" or "defi", got ${tierArg}`);
  }
  return {
    rpc: get("rpc") ?? "https://api.devnet.solana.com",
    wallet: resolve(get("wallet") ?? "./deploy-keypair.json"),
    monthDurationSecs: parseInt(get("month-duration") ?? "90", 10),
    tier: tierArg as TierName,
  };
}

function loadKeypair(path: string): Keypair {
  const json = JSON.parse(readFileSync(path, "utf8"));
  return Keypair.fromSecretKey(Uint8Array.from(json));
}

function sleep(ms: number): Promise<void> {
  return new Promise((res) => setTimeout(res, ms));
}

function pickPayers(
  rng: () => number,
  goodIndices: number[],
  flakyIndices: number[],
  badIndices: number[]
): number[] {
  const pay: number[] = [];
  for (const i of goodIndices) pay.push(i);
  for (const i of flakyIndices) if (rng() < 0.6) pay.push(i);
  for (const i of badIndices) if (rng() < 0.3) pay.push(i);
  return pay;
}

// Deterministic-ish PRNG so the same run is reproducible if needed.
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

  console.log("admin:", admin.publicKey.toBase58());
  console.log("rpc:  ", args.rpc);
  console.log("month duration:", args.monthDurationSecs, "s");
  console.log("tier:          ", args.tier);

  // ───── 1. Generate 12 wallets ────────────────────────────────────
  const wallets: Keypair[] = Array.from({ length: 12 }, () =>
    Keypair.generate()
  );
  const ts = new Date().toISOString().replace(/[:.]/g, "-");
  mkdirSync(".deploy-recovery", { recursive: true });
  const outFile = `.deploy-recovery/sim-wallets-${ts}.json`;
  writeFileSync(
    outFile,
    JSON.stringify(
      {
        wallets: wallets.map((kp) => ({
          publicKey: kp.publicKey.toBase58(),
          secret: Array.from(kp.secretKey),
        })),
      },
      null,
      2
    )
  );
  console.log("\n12 wallets generated →", outFile);

  // ───── 2. Fund each with SOL + USDC, init KYC + reputation ───────
  const SOL_PER_WALLET = 0.05 * LAMPORTS_PER_SOL;
  const USDC_PER_WALLET = 150_000n * 1_000_000n; // 150K USDC
  const adminUsdc = getAssociatedTokenAddressSync(
    USDC_MINT_DEVNET_DEFAULT,
    admin.publicKey
  );

  console.log("\nfunding 12 wallets…");
  for (let i = 0; i < wallets.length; i++) {
    const kp = wallets[i];
    const userUsdc = getAssociatedTokenAddressSync(
      USDC_MINT_DEVNET_DEFAULT,
      kp.publicKey
    );

    // (a) SOL transfer + USDC mint in one tx (admin signs).
    const fundTx = new Transaction();
    fundTx.add(
      SystemProgram.transfer({
        fromPubkey: admin.publicKey,
        toPubkey: kp.publicKey,
        lamports: SOL_PER_WALLET,
      })
    );
    fundTx.add(
      createAssociatedTokenAccountIdempotentInstruction(
        admin.publicKey,
        userUsdc,
        kp.publicKey,
        USDC_MINT_DEVNET_DEFAULT
      )
    );
    fundTx.add(
      createMintToInstruction(
        USDC_MINT_DEVNET_DEFAULT,
        userUsdc,
        admin.publicKey,
        USDC_PER_WALLET,
        [],
        TOKEN_PROGRAM_ID
      )
    );
    await sendAndConfirmTransaction(conn, fundTx, [admin], {
      commitment: "confirmed",
    });

    // (b) KYC + reputation init (user signs reputation).
    const userProvider = new AnchorProvider(conn, new Wallet(kp), {
      commitment: "confirmed",
    });
    const userClient = new PoolverClient(userProvider);

    const kycIx = await mockIssueKycIx(adminClient, {
      user: kp.publicKey,
      level: "full",
      validForSecs: 365 * 24 * 60 * 60,
    });
    const repIx = await initializeUserReputationIx(userClient, {
      user: kp.publicKey,
    });
    const setupTx = new Transaction().add(kycIx, repIx);
    setupTx.feePayer = admin.publicKey;
    setupTx.recentBlockhash = (await conn.getLatestBlockhash()).blockhash;
    setupTx.partialSign(admin);
    setupTx.partialSign(kp);
    await conn.sendRawTransaction(setupTx.serialize(), {
      skipPreflight: false,
    });
    // Wait for kyc account to be visible before the next round.
    await sleep(500);

    process.stdout.write(`  [${i + 1}/12] ${kp.publicKey.toBase58().slice(0, 8)}… funded\n`);
  }

  // ───── 3. Admin creates the pool ─────────────────────────────────
  // 4,167 USDC × 12 users × 12 months ≈ 600K USDC contributed lifetime,
  // monthly_pot ≈ 12 × net(4167) ≈ 50K → "50K total" per cycle.
  const contributionMicro = new BN("4167000000"); // 4,167 USDC in microUSDC
  const poolId = new BN(Math.floor(Date.now() / 1000));
  const { ix: createIx, pool: poolPk } = await createPoolIx(adminClient, {
    poolId,
    tier: args.tier,
    contributionAmount: contributionMicro,
    monthDurationSeconds: new BN(args.monthDurationSecs),
    usdcMint: USDC_MINT_DEVNET_DEFAULT,
  });
  await adminProvider.sendAndConfirm!(new Transaction().add(createIx), [], {
    commitment: "confirmed",
  });
  console.log("\npool created:", poolPk.toBase58());

  // ───── 4. All 12 join ────────────────────────────────────────────
  console.log("\njoining all 12 wallets to the pool…");
  for (let i = 0; i < wallets.length; i++) {
    const kp = wallets[i];
    const userProvider = new AnchorProvider(conn, new Wallet(kp), {
      commitment: "confirmed",
    });
    const userClient = new PoolverClient(userProvider);
    const ix = await joinPoolIx(userClient, {
      pool: poolPk,
      tier: args.tier,
      usdcMint: USDC_MINT_DEVNET_DEFAULT,
    });
    await userProvider.sendAndConfirm!(new Transaction().add(ix), [], {
      commitment: "confirmed",
    });
    process.stdout.write(`  [${i + 1}/12] joined\n`);
  }

  const goodIndices = [0, 1, 2, 3, 4, 5];
  const flakyIndices = [6, 7, 8, 9];
  const badIndices = [10, 11];
  const rng = mulberry32(0xC07150);

  // ───── 5. Walk 6 months: contribute → slash → draw → advance ─────
  const FULLY_RESOLVED_MONTHS = 6;
  for (let m = 1; m <= FULLY_RESOLVED_MONTHS; m++) {
    const payers = pickPayers(rng, goodIndices, flakyIndices, badIndices);
    const skippers = wallets
      .map((_, i) => i)
      .filter((i) => !payers.includes(i));

    console.log(`\n══ MONTH ${m} ══`);
    console.log(
      `  paying  (${payers.length}):   ${payers.map((i) => wallets[i].publicKey.toBase58().slice(0, 6)).join(", ")}`
    );
    console.log(
      `  skipping(${skippers.length}): ${skippers.map((i) => wallets[i].publicKey.toBase58().slice(0, 6)).join(", ") || "—"}`
    );

    for (const i of payers) {
      const kp = wallets[i];
      const userProvider = new AnchorProvider(conn, new Wallet(kp), {
        commitment: "confirmed",
      });
      const userClient = new PoolverClient(userProvider);
      try {
        const ix = await contributeIx(userClient, {
          pool: poolPk,
          tier: args.tier,
          usdcMint: USDC_MINT_DEVNET_DEFAULT,
        });
        await userProvider.sendAndConfirm!(new Transaction().add(ix), [], {
          commitment: "confirmed",
        });
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        console.warn(`  contribute failed for ${kp.publicKey.toBase58().slice(0, 8)}…: ${msg.slice(0, 120)}`);
      }
    }

    // Wait until both gates are satisfied:
    //   - `now >= month_end`        → slash_unpaid + advance_month accept
    //   - `now >= reveal_window_ends_at` → select_winner accepts
    // The reveal window has a 60s minimum floor on-chain
    // (`(bid_window/2).max(60)`), so for short month durations the
    // reveal_window_ends_at can land AFTER month_end. Read both from
    // chain and sleep until the later one (+5s buffer).
    {
      const probe = await fetchPool(adminClient, poolPk);
      const monthEndSec =
        probe.currentMonthStartedAt.toNumber() +
        probe.monthDurationSeconds.toNumber();
      const revealEndSec = probe.revealWindowEndsAt.toNumber();
      const targetSec = Math.max(monthEndSec, revealEndSec) + 5;
      const waitMs = Math.max(0, targetSec * 1000 - Date.now());
      console.log(
        `  ⏱  waiting ${(waitMs / 1000).toFixed(0)}s for month_end + reveal_close…`
      );
      await sleep(waitMs);
    }

    // Slash unpaid.
    if (skippers.length > 0) {
      console.log(`  ⚠ slashing ${skippers.length} unpaid…`);
      for (const i of skippers) {
        const delinquent = wallets[i].publicKey;
        try {
          const ix = await slashUnpaidIx(adminClient, {
            pool: poolPk,
            delinquent,
            tier: args.tier,
          });
          await adminProvider.sendAndConfirm!(
            new Transaction().add(ix),
            [],
            { commitment: "confirmed" }
          );
        } catch (e) {
          const msg = e instanceof Error ? e.message : String(e);
          console.warn(
            `    slash ${delinquent.toBase58().slice(0, 8)}…: ${msg.slice(0, 120)}`
          );
        }
      }
    }

    // Build candidates for select_winner. No one bid this month, so we
    // hit the lottery branch with non-bidders only. Filter out:
    //   - participants already won a prior month (eligibility)
    //   - participants now defaulted from being slashed dry
    //   - participants whose collateral got fully drained (is_defaulted
    //     was set inside slash_unpaid)
    const poolView = await fetchPool(adminClient, poolPk);
    const winnersSet = new Set<string>();
    const winnersArr = (
      poolView.raw as {
        winners: Array<{
          winner: PublicKey;
          selectedAt: BN;
        }>;
      }
    ).winners;
    for (const w of winnersArr) {
      if (w.selectedAt.gtn(0)) winnersSet.add(w.winner.toBase58());
    }

    const eligibleNonBidders: PublicKey[] = [];
    for (const kp of wallets) {
      const k = kp.publicKey.toBase58();
      if (winnersSet.has(k)) continue;
      eligibleNonBidders.push(kp.publicKey);
    }

    // Only first 11 candidates fit in the legacy tx; with our ALT we
    // can squeeze 12. Attempt the call.
    try {
      const selectIx = await selectWinnerIx(adminClient, {
        pool: poolPk,
        tier: args.tier,
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

    // Advance.
    try {
      const advIx = await advanceMonthIx(adminClient, { pool: poolPk });
      await adminProvider.sendAndConfirm!(new Transaction().add(advIx), [], {
        commitment: "confirmed",
      });
      console.log(`  ↯ advanced to month ${m + 1}`);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      console.warn(`  advance_month failed: ${msg.slice(0, 200)}`);
    }
  }

  // ───── 6. Month 7 — contribute partially, leave unpaid + ended ───
  const m = FULLY_RESOLVED_MONTHS + 1;
  const payers = pickPayers(rng, goodIndices, flakyIndices, badIndices);
  const skippers = wallets
    .map((_, i) => i)
    .filter((i) => !payers.includes(i));
  console.log(`\n══ MONTH ${m} (partial — left for UI demo) ══`);
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
        tier: args.tier,
        usdcMint: USDC_MINT_DEVNET_DEFAULT,
      });
      await userProvider.sendAndConfirm!(new Transaction().add(ix), [], {
        commitment: "confirmed",
      });
    } catch (e) {
      // Some "good payers" may already have won and have no further
      // contribution obligation — ignore those failures.
    }
  }

  // Same wait pattern as the loop — sleep until both month_end and
  // reveal_window_ends_at have passed, so the UI's auto-slash effect
  // fires immediately on page load.
  {
    const probe = await fetchPool(adminClient, poolPk);
    const monthEndSec =
      probe.raw && (probe.raw as any).currentMonthStartedAt
        ? (probe.raw as any).currentMonthStartedAt.toNumber() +
          (probe.raw as any).monthDurationSeconds.toNumber()
        : 0;
    const revealEndSec = probe.revealWindowEndsAt.toNumber();
    const targetSec = Math.max(monthEndSec, revealEndSec) + 5;
    const waitMs = Math.max(0, targetSec * 1000 - Date.now());
    console.log(`  ⏱  waiting ${(waitMs / 1000).toFixed(0)}s for month_end…`);
    await sleep(waitMs);
  }

  console.log("\n✓ simulation done");
  console.log("  pool:", poolPk.toBase58());
  console.log(
    "  url:  https://poolver.com/pool/" + poolPk.toBase58()
  );
  console.log(
    "  Open in the UI: month",
    m,
    "has ended with",
    skippers.length,
    "unpaid wallets — the auto-slash effect will fire on page load."
  );
  console.log("  wallets file:", outFile);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
