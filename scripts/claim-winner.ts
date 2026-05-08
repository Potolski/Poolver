#!/usr/bin/env npx tsx
/**
 * Find the current month's winner in the pool, and if that winner is one
 * of the seed wallets, sign + send `claim_winning` from that wallet.
 *
 * Usage:
 *   NODE_PATH=$(pwd)/client/node_modules npx tsx scripts/claim-winner.ts \
 *     --keypair-file .deploy-recovery/seed-wallets-EQnZSyGn-... \
 *     --rpc "https://devnet.helius-rpc.com/?api-key=..."
 *
 * No-ops gracefully if:
 *   - no winner is selected for the current month
 *   - the winner is already claimed
 *   - the winner is NOT in the seed file (e.g. it was the demo runner)
 */
import {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import { Wallet } from "@coral-xyz/anchor";
import { readFileSync } from "fs";
import { resolve } from "path";

import {
  PoolverClient,
  claimWinningIx,
  fetchPool,
  USDC_MINT_DEVNET_DEFAULT,
} from "../client/src";
import BN from "bn.js";

interface SeedFile {
  pool: string;
  rpc: string;
  wallets: Array<{ publicKey: string; secretKey: number[] }>;
}

interface MonthWinnerRaw {
  month: number;
  winner: PublicKey;
  selectedAt: BN;
  claimed: boolean;
}

interface Args {
  keypairFile: string;
  rpc: string;
}

function parseArgs(argv: string[]): Args {
  const get = (k: string): string | undefined => {
    const i = argv.indexOf(`--${k}`);
    return i >= 0 ? argv[i + 1] : undefined;
  };
  const keypairFile = get("keypair-file");
  if (!keypairFile) throw new Error("Missing --keypair-file <path>");
  return {
    keypairFile: resolve(keypairFile),
    rpc: get("rpc") ?? "https://api.devnet.solana.com",
  };
}

function keypairWallet(kp: Keypair): Wallet {
  return {
    publicKey: kp.publicKey,
    payer: kp,
    signTransaction: async <T extends Transaction>(tx: T) => {
      tx.partialSign(kp);
      return tx;
    },
    signAllTransactions: async <T extends Transaction>(txs: T[]) => {
      for (const tx of txs) tx.partialSign(kp);
      return txs;
    },
  } as Wallet;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const seed = JSON.parse(readFileSync(args.keypairFile, "utf8")) as SeedFile;
  const conn = new Connection(args.rpc, "confirmed");
  const poolPk = new PublicKey(seed.pool);

  const pool = await fetchPool(
    new PoolverClient({
      connection: conn,
      wallet: keypairWallet(Keypair.generate()),
      cluster: "devnet",
    }),
    poolPk
  );
  if (!pool) throw new Error(`pool ${poolPk.toBase58()} not found`);
  if (pool.currentMonth < 1) {
    console.log(`[claim] pool not started (month=${pool.currentMonth})`);
    return;
  }

  const winnersArr = (pool.raw as { winners?: MonthWinnerRaw[] }).winners ?? [];
  const monthWinner = winnersArr[pool.currentMonth - 1];
  if (!monthWinner || !monthWinner.selectedAt?.gtn?.(0)) {
    console.log(
      `[claim] no winner selected yet for month ${pool.currentMonth}. Run select_winner first.`
    );
    return;
  }
  if (monthWinner.claimed) {
    console.log(
      `[claim] winner already claimed for month ${pool.currentMonth} (winner=${monthWinner.winner.toBase58().slice(0, 8)}…). Nothing to do.`
    );
    return;
  }

  const winnerPk = monthWinner.winner;
  const seedEntry = seed.wallets.find((w) => w.publicKey === winnerPk.toBase58());
  if (!seedEntry) {
    console.log(
      `[claim] winner ${winnerPk.toBase58()} is NOT a seed wallet — likely the demo runner. They need to click "Claim winnings" in the UI.`
    );
    return;
  }

  const winnerKp = Keypair.fromSecretKey(Uint8Array.from(seedEntry.secretKey));
  const winnerClient = new PoolverClient({
    connection: conn,
    wallet: keypairWallet(winnerKp),
    cluster: "devnet",
  });

  console.log(
    `[claim] winner=${winnerPk.toBase58().slice(0, 8)}… is a seed wallet — sending claim_winning…`
  );
  const ix = await claimWinningIx(winnerClient, {
    pool: poolPk,
    tier: pool.tier,
    usdcMint: USDC_MINT_DEVNET_DEFAULT,
  });
  const tx = new Transaction().add(ix);
  const sig = await sendAndConfirmTransaction(conn, tx, [winnerKp], {
    commitment: "confirmed",
  });
  console.log(`[claim] DONE — sig=${sig}`);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
