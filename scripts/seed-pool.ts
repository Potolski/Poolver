#!/usr/bin/env npx tsx
/**
 * Seed a pool with N freshly-generated wallets — for testing + demos.
 *
 * Each wallet:
 *   1. is funded with SOL for tx fees (airdrop or admin transfer)
 *   2. gets a UserReputation PDA (initialize_user_reputation)
 *   3. gets a Full KYC attestation (mock_issue_kyc, admin signs)
 *   4. is faucet'd USDC (admin signs)
 *   5. joins the target pool (join_pool)
 *
 * Output: keypairs saved to `.deploy-recovery/seed-wallets-<pool>-<ts>.json`
 * (gitignored) so you can replay / inspect / send their txs later.
 *
 * Usage:
 *   NODE_PATH=$(pwd)/client/node_modules npx tsx scripts/seed-pool.ts \
 *     --pool EQnZSyGn76cnCamf4donH5a9cyxfWA3zhwphNcrUfrvv \
 *     --rpc "https://devnet.helius-rpc.com/?api-key=..." \
 *     [--count 10] \
 *     [--admin-keypair ./deploy-keypair.json] \
 *     [--usdc-amount 100000] \
 *     [--sol-amount 0.05]
 */
import {
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import {
  createAssociatedTokenAccountIdempotentInstruction,
  createMintToInstruction,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import { AnchorProvider, Wallet } from "@coral-xyz/anchor";
import { mkdirSync, readFileSync, writeFileSync } from "fs";
import { resolve } from "path";

import {
  PoolverClient,
  fetchPool,
  initializeUserReputationIx,
  mockIssueKycIx,
  joinPoolIx,
  USDC_MINT_DEVNET_DEFAULT,
} from "../client/src";

const USDC_DECIMALS = 6;
const MICRO_PER_USDC = 1_000_000n;

interface Args {
  pool: PublicKey;
  rpc: string;
  count: number;
  adminKeypair: string;
  usdcAmount: number;
  solAmount: number;
}

function parseArgs(argv: string[]): Args {
  const get = (k: string): string | undefined => {
    const i = argv.indexOf(`--${k}`);
    return i >= 0 ? argv[i + 1] : undefined;
  };
  const pool = get("pool");
  const rpc = get("rpc") ?? "https://api.devnet.solana.com";
  if (!pool) throw new Error("Missing --pool <address>");
  return {
    pool: new PublicKey(pool),
    rpc,
    count: parseInt(get("count") ?? "10", 10),
    adminKeypair: resolve(get("admin-keypair") ?? "./deploy-keypair.json"),
    usdcAmount: parseFloat(get("usdc-amount") ?? "100000"),
    solAmount: parseFloat(get("sol-amount") ?? "0.05"),
  };
}

function loadKeypair(path: string): Keypair {
  return Keypair.fromSecretKey(Uint8Array.from(JSON.parse(readFileSync(path, "utf8"))));
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

async function fundSol(
  conn: Connection,
  admin: Keypair,
  recipient: PublicKey,
  lamports: number
): Promise<string> {
  const tx = new Transaction().add(
    SystemProgram.transfer({
      fromPubkey: admin.publicKey,
      toPubkey: recipient,
      lamports,
    })
  );
  return sendAndConfirmTransaction(conn, tx, [admin], { commitment: "confirmed" });
}

async function faucetUsdc(
  conn: Connection,
  admin: Keypair,
  mint: PublicKey,
  recipient: PublicKey,
  humanAmount: number
): Promise<string> {
  const ata = getAssociatedTokenAddressSync(mint, recipient);
  const microAmount =
    BigInt(Math.floor(humanAmount)) * MICRO_PER_USDC;
  const tx = new Transaction()
    .add(
      createAssociatedTokenAccountIdempotentInstruction(
        admin.publicKey,
        ata,
        recipient,
        mint
      )
    )
    .add(createMintToInstruction(mint, ata, admin.publicKey, microAmount));
  return sendAndConfirmTransaction(conn, tx, [admin], { commitment: "confirmed" });
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const admin = loadKeypair(args.adminKeypair);
  const conn = new Connection(args.rpc, "confirmed");

  console.log(`[seed] pool=${args.pool.toBase58()} count=${args.count} rpc=${args.rpc.slice(0, 50)}…`);
  console.log(`[seed] admin=${admin.publicKey.toBase58()}`);

  const adminBalance = await conn.getBalance(admin.publicKey);
  const requiredLamports =
    args.count * (args.solAmount * LAMPORTS_PER_SOL + 0.005 * LAMPORTS_PER_SOL);
  if (adminBalance < requiredLamports) {
    console.warn(
      `[seed] WARN admin has ${adminBalance / LAMPORTS_PER_SOL} SOL; estimated need ${requiredLamports / LAMPORTS_PER_SOL} SOL`
    );
  }

  const pool = await fetchPool(
    new PoolverClient({
      connection: conn,
      wallet: keypairWallet(admin),
      cluster: "devnet",
    }),
    args.pool
  );
  if (!pool) throw new Error(`pool ${args.pool.toBase58()} not found on chain`);
  console.log(`[seed] pool tier=${pool.tier} contribution=${Number(pool.contributionAmount) / 1e6} USDC current_month=${pool.currentMonth} filled=${pool.participantCount}/${pool.maxParticipants}`);

  const remainingSlots = pool.maxParticipants - pool.participantCount;
  if (remainingSlots <= 0) {
    console.log(`[seed] pool already full; nothing to do.`);
    return;
  }
  const targetCount = Math.min(args.count, remainingSlots);
  if (targetCount < args.count) {
    console.log(`[seed] only ${remainingSlots} slot(s) free; will seed ${targetCount} of ${args.count}`);
  }

  const wallets: Keypair[] = Array.from({ length: targetCount }, () => Keypair.generate());

  // Persist before any tx so we can recover even if the run crashes
  mkdirSync(".deploy-recovery", { recursive: true });
  const outPath = resolve(
    `.deploy-recovery/seed-wallets-${args.pool.toBase58().slice(0, 8)}-${Date.now()}.json`
  );
  writeFileSync(
    outPath,
    JSON.stringify(
      {
        pool: args.pool.toBase58(),
        rpc: args.rpc,
        createdAt: new Date().toISOString(),
        wallets: wallets.map((kp) => ({
          publicKey: kp.publicKey.toBase58(),
          secretKey: Array.from(kp.secretKey),
        })),
      },
      null,
      2
    )
  );
  console.log(`[seed] saved ${targetCount} keypairs to ${outPath}`);

  const usdcMint = USDC_MINT_DEVNET_DEFAULT;

  for (let i = 0; i < wallets.length; i++) {
    const user = wallets[i]!;
    const tag = `[${i + 1}/${targetCount}] ${user.publicKey.toBase58().slice(0, 8)}…`;
    try {
      // 1. Fund SOL
      const solSig = await fundSol(
        conn,
        admin,
        user.publicKey,
        Math.floor(args.solAmount * LAMPORTS_PER_SOL)
      );
      console.log(`${tag} sol → ${solSig.slice(0, 12)}…`);

      // 2. Faucet USDC (must happen before join_pool — the join consumes it)
      const usdcSig = await faucetUsdc(
        conn,
        admin,
        usdcMint,
        user.publicKey,
        args.usdcAmount
      );
      console.log(`${tag} usdc(${args.usdcAmount}) → ${usdcSig.slice(0, 12)}…`);

      // 3. Issue Full KYC (admin signs the mock_issue_kyc ix)
      const adminClient = new PoolverClient({
        connection: conn,
        wallet: keypairWallet(admin),
        cluster: "devnet",
      });
      const kycIx = await mockIssueKycIx(adminClient, {
        user: user.publicKey,
        level: "full",
      });
      const kycTx = new Transaction().add(kycIx);
      try {
        const kycSig = await sendAndConfirmTransaction(conn, kycTx, [admin]);
        console.log(`${tag} kyc → ${kycSig.slice(0, 12)}…`);
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        if (msg.includes("already in use") || msg.includes("custom program error: 0x0")) {
          console.log(`${tag} kyc → already issued`);
        } else throw e;
      }

      // 4. Initialize UserReputation (user signs as their own wallet)
      const userClient = new PoolverClient({
        connection: conn,
        wallet: keypairWallet(user),
        cluster: "devnet",
      });
      const repIx = await initializeUserReputationIx(userClient);
      const repTx = new Transaction().add(repIx);
      try {
        const repSig = await sendAndConfirmTransaction(conn, repTx, [user]);
        console.log(`${tag} reputation → ${repSig.slice(0, 12)}…`);
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        if (msg.includes("already in use")) {
          console.log(`${tag} reputation → already exists`);
        } else throw e;
      }

      // 5. Join pool
      const joinIx = await joinPoolIx(userClient, {
        pool: args.pool,
        tier: pool.tier,
        usdcMint,
      });
      const joinTx = new Transaction().add(joinIx);
      const joinSig = await sendAndConfirmTransaction(conn, joinTx, [user]);
      console.log(`${tag} join → ${joinSig.slice(0, 12)}…`);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      console.error(`${tag} FAILED: ${msg.slice(0, 220)}`);
    }
  }

  // Re-fetch pool to confirm state
  const finalClient = new PoolverClient({
    connection: conn,
    wallet: keypairWallet(admin),
    cluster: "devnet",
  });
  const after = await fetchPool(finalClient, args.pool);
  if (after) {
    console.log(
      `[seed] DONE. pool now ${after.participantCount}/${after.maxParticipants} filled. month=${after.currentMonth}, isComplete=${after.isComplete}`
    );
  }
  console.log(`[seed] keypair file: ${outPath}`);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
