#!/usr/bin/env npx tsx
/**
 * Have all wallets in a seed-pool keypair file pay the current month's
 * contribution.
 *
 * Usage:
 *   NODE_PATH=$(pwd)/client/node_modules npx tsx scripts/contribute-all.ts \
 *     --keypair-file .deploy-recovery/seed-wallets-EQnZSyGn-1778254772397.json \
 *     --rpc "https://devnet.helius-rpc.com/?api-key=..."
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
  contributeIx,
  fetchPool,
  fetchParticipant,
  USDC_MINT_DEVNET_DEFAULT,
  hasPaidMonth,
} from "../client/src";

interface SeedFile {
  pool: string;
  rpc: string;
  wallets: Array<{ publicKey: string; secretKey: number[] }>;
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
  console.log(`[contrib-all] pool=${poolPk.toBase58()} wallets=${seed.wallets.length}`);

  // Probe the pool once for tier + current_month
  const probe = await fetchPool(
    new PoolverClient({
      connection: conn,
      wallet: keypairWallet(Keypair.generate()), // throwaway for read-only
      cluster: "devnet",
    }),
    poolPk
  );
  if (!probe) throw new Error(`pool not found on chain`);
  if (probe.currentMonth < 1 || probe.currentMonth > probe.totalMonths) {
    throw new Error(
      `pool current_month=${probe.currentMonth} (not in [1..${probe.totalMonths}]) — nothing to contribute for`
    );
  }
  console.log(
    `[contrib-all] tier=${probe.tier} current_month=${probe.currentMonth} paid_so_far=${probe.paidCountForCurrentMonth}/12`
  );

  let success = 0;
  let alreadyPaid = 0;
  let failed = 0;

  for (let i = 0; i < seed.wallets.length; i++) {
    const w = seed.wallets[i]!;
    const kp = Keypair.fromSecretKey(Uint8Array.from(w.secretKey));
    const tag = `[${i + 1}/${seed.wallets.length}] ${kp.publicKey.toBase58().slice(0, 8)}…`;
    try {
      const userClient = new PoolverClient({
        connection: conn,
        wallet: keypairWallet(kp),
        cluster: "devnet",
      });

      // Skip if already paid this month
      const part = await fetchParticipant(userClient, poolPk, kp.publicKey);
      if (!part) {
        console.log(`${tag} not a participant (skipping)`);
        continue;
      }
      if (hasPaidMonth(part, probe.currentMonth)) {
        console.log(`${tag} already paid month ${probe.currentMonth} — skipping`);
        alreadyPaid++;
        continue;
      }

      const ix = await contributeIx(userClient, {
        pool: poolPk,
        tier: probe.tier,
        usdcMint: USDC_MINT_DEVNET_DEFAULT,
      });
      const tx = new Transaction().add(ix);
      const sig = await sendAndConfirmTransaction(conn, tx, [kp], {
        commitment: "confirmed",
      });
      console.log(`${tag} contribute → ${sig.slice(0, 12)}…`);
      success++;
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      console.error(`${tag} FAILED: ${msg.slice(0, 220)}`);
      failed++;
    }
  }

  // Final state
  const after = await fetchPool(
    new PoolverClient({
      connection: conn,
      wallet: keypairWallet(Keypair.generate()),
      cluster: "devnet",
    }),
    poolPk
  );
  console.log(
    `[contrib-all] DONE. success=${success} alreadyPaid=${alreadyPaid} failed=${failed}. pool now ${after?.paidCountForCurrentMonth ?? "?"}/12 paid for month ${after?.currentMonth ?? "?"}.`
  );
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
