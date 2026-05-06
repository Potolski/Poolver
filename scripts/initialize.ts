#!/usr/bin/env npx tsx
/**
 * Poolver V1 one-time post-deploy initialization.
 *
 * Idempotent: if any account already exists, skip with a log line.
 *
 *   1. poolver_core.initialize_protocol(usdc_mint, admin)
 *   2. poolver_reserve.initialize_reserve(Tier::Vault)
 *   3. poolver_reserve.initialize_reserve(Tier::DeFi)
 *
 * Usage:
 *
 *   npx tsx scripts/initialize.ts \
 *     --cluster devnet \
 *     --usdc-mint Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr \
 *     --wallet ./deploy-keypair.json
 */
import {
  Connection,
  Keypair,
  PublicKey,
  sendAndConfirmTransaction,
  Transaction,
} from "@solana/web3.js";
import { Wallet } from "@coral-xyz/anchor";
import { readFileSync } from "fs";
import { resolve } from "path";

import {
  PoolverClient,
  initializeProtocolIx,
  buildInitializeReserveAccounts,
  findProtocolConfig,
  findReserveFund,
  USDC_MINT_DEVNET_DEFAULT,
} from "../client/src";

interface InitArgs {
  cluster: "devnet" | "mainnet-beta" | "localnet";
  usdcMint: PublicKey;
  walletPath: string;
}

function parseArgs(): InitArgs {
  const argv = process.argv.slice(2);
  const get = (k: string): string | undefined => {
    const i = argv.indexOf(k);
    return i >= 0 ? argv[i + 1] : undefined;
  };
  const cluster = (get("--cluster") ?? "devnet") as InitArgs["cluster"];
  const usdcRaw = get("--usdc-mint");
  return {
    cluster,
    usdcMint: usdcRaw ? new PublicKey(usdcRaw) : USDC_MINT_DEVNET_DEFAULT,
    walletPath: get("--wallet") ?? resolve(__dirname, "../deploy-keypair.json"),
  };
}

function clusterUrl(c: InitArgs["cluster"]): string {
  return c === "mainnet-beta"
    ? "https://api.mainnet-beta.solana.com"
    : c === "devnet"
    ? "https://api.devnet.solana.com"
    : "http://127.0.0.1:8899";
}

function loadKeypair(path: string): Keypair {
  const bytes = JSON.parse(readFileSync(path, "utf-8"));
  return Keypair.fromSecretKey(Uint8Array.from(bytes));
}

async function main(): Promise<void> {
  const args = parseArgs();
  const connection = new Connection(clusterUrl(args.cluster), "confirmed");
  const signer = loadKeypair(args.walletPath);
  const wallet = new Wallet(signer);
  const client = new PoolverClient({ connection, wallet });

  console.log(
    `[initialize] cluster=${args.cluster} admin=${signer.publicKey.toBase58()} usdc=${args.usdcMint.toBase58()}`
  );

  // ─── Step 1: protocol_config ───
  const [protocolConfig] = findProtocolConfig();
  if (await accountExists(connection, protocolConfig)) {
    console.log(`[skip] ProtocolConfig already exists at ${protocolConfig.toBase58()}`);
  } else {
    const ix = await initializeProtocolIx(client, { usdcMint: args.usdcMint });
    const sig = await sendAndConfirmTransaction(
      connection,
      new Transaction().add(ix),
      [signer]
    );
    console.log(`[ok] initialize_protocol → ${sig}`);
  }

  // ─── Step 2/3: per-tier reserves ───
  for (const tier of ["vault", "defi"] as const) {
    const [reserveFund] = findReserveFund(tier);
    if (await accountExists(connection, reserveFund)) {
      console.log(`[skip] ReserveFund(${tier}) already exists at ${reserveFund.toBase58()}`);
      continue;
    }
    const accounts = buildInitializeReserveAccounts(
      signer.publicKey,
      tier,
      args.usdcMint
    );
    // The Tier enum is passed as { vault: {} } | { defi: {} }
    const tierIdl = tier === "vault" ? { vault: {} } : { defi: {} };
    const ix = await client.reserve.methods
      .initializeReserve(tierIdl)
      .accounts(accounts)
      .instruction();
    const sig = await sendAndConfirmTransaction(
      connection,
      new Transaction().add(ix),
      [signer]
    );
    console.log(`[ok] initialize_reserve(${tier}) → ${sig}`);
  }

  console.log("\n[initialize] DONE.");
}

async function accountExists(
  conn: Connection,
  pk: PublicKey
): Promise<boolean> {
  const info = await conn.getAccountInfo(pk);
  return info !== null;
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
