#!/usr/bin/env npx tsx
/**
 * Mock USDC faucet — mints test USDC to a recipient wallet.
 *
 * Usage:
 *
 *   npx tsx scripts/faucet.ts \
 *     --recipient <wallet-pubkey> \
 *     --amount 1000 \
 *     --rpc https://devnet.helius-rpc.com/?api-key=... \
 *     [--mint B6dnuZtKH7FsSK6tySfWkk6ReW2LdKpmnfGAoMKsv8w8] \
 *     [--admin-keypair ./deploy-keypair.json]
 *
 * The amount is in human USDC (e.g. 1000 = 1000 USDC = 1_000_000_000
 * micro-USDC). Decimals are USDC standard (6).
 *
 * Creates the recipient's ATA if it doesn't exist, then mints.
 *
 * Production note: replace this with a server-side endpoint that
 * rate-limits per IP and per recipient pubkey.
 */
import {
  Connection,
  Keypair,
  PublicKey,
  sendAndConfirmTransaction,
  Transaction,
} from "@solana/web3.js";
import {
  createAssociatedTokenAccountIdempotentInstruction,
  createMintToInstruction,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import { readFileSync } from "fs";
import { resolve } from "path";

const DEFAULT_MINT = "B6dnuZtKH7FsSK6tySfWkk6ReW2LdKpmnfGAoMKsv8w8";
const USDC_DECIMALS = 6;

interface Args {
  recipient: PublicKey;
  amount: bigint;
  rpc: string;
  mint: PublicKey;
  adminKeypair: string;
}

function parseArgs(argv: string[]): Args {
  const get = (k: string): string | undefined => {
    const i = argv.indexOf(`--${k}`);
    return i >= 0 ? argv[i + 1] : undefined;
  };
  const recipient = get("recipient");
  const amount = get("amount");
  const rpc = get("rpc") ?? "https://api.devnet.solana.com";
  const mint = get("mint") ?? DEFAULT_MINT;
  const adminKeypair = get("admin-keypair") ?? "./deploy-keypair.json";
  if (!recipient) throw new Error("Missing --recipient <wallet-pubkey>");
  if (!amount) throw new Error("Missing --amount <human-usdc>");
  return {
    recipient: new PublicKey(recipient),
    amount: BigInt(Math.floor(parseFloat(amount) * 10 ** USDC_DECIMALS)),
    rpc,
    mint: new PublicKey(mint),
    adminKeypair: resolve(adminKeypair),
  };
}

function loadKeypair(path: string): Keypair {
  const raw = JSON.parse(readFileSync(path, "utf8"));
  return Keypair.fromSecretKey(Uint8Array.from(raw));
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const admin = loadKeypair(args.adminKeypair);
  const conn = new Connection(args.rpc, "confirmed");

  const recipientAta = getAssociatedTokenAddressSync(args.mint, args.recipient);

  console.log(
    `[faucet] mint=${args.mint.toBase58()} recipient=${args.recipient.toBase58()} ata=${recipientAta.toBase58()} amount=${args.amount} (${Number(args.amount) / 10 ** USDC_DECIMALS} USDC)`
  );

  const tx = new Transaction()
    .add(
      createAssociatedTokenAccountIdempotentInstruction(
        admin.publicKey,
        recipientAta,
        args.recipient,
        args.mint
      )
    )
    .add(
      createMintToInstruction(
        args.mint,
        recipientAta,
        admin.publicKey,
        args.amount
      )
    );

  const sig = await sendAndConfirmTransaction(conn, tx, [admin]);
  console.log(`[ok] minted → ${sig}`);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
