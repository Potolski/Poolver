#!/usr/bin/env npx tsx
/**
 * Top up a tier reserve from the admin wallet.
 *
 *   npx tsx scripts/seed-reserve.ts \
 *     --tier vault \
 *     --amount 10000 \
 *     --cluster devnet \
 *     --usdc-mint Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr
 *
 * `--amount` is in human USDC; the script multiplies by 1e6 to microUSDC.
 *
 * Calls `poolver_reserve::seed(amount)`. The admin must hold the funds
 * in their associated USDC account.
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
  buildSeedReserveAccounts,
  TierName,
  USDC_MINT_DEVNET_DEFAULT,
  humanUsdcToMicro,
} from "../client/src";

interface SeedArgs {
  cluster: "devnet" | "mainnet-beta" | "localnet";
  tier: TierName;
  humanAmount: string;
  usdcMint: PublicKey;
  walletPath: string;
}

function parseArgs(): SeedArgs {
  const argv = process.argv.slice(2);
  const get = (k: string): string | undefined => {
    const i = argv.indexOf(k);
    return i >= 0 ? argv[i + 1] : undefined;
  };
  const tier = (get("--tier") ?? "") as TierName;
  if (tier !== "vault" && tier !== "defi") {
    console.error('--tier must be one of: "vault", "defi"');
    process.exit(1);
  }
  const amount = get("--amount");
  if (!amount) {
    console.error("--amount is required (human USDC)");
    process.exit(1);
  }
  const cluster = (get("--cluster") ?? "devnet") as SeedArgs["cluster"];
  const usdcRaw = get("--usdc-mint");
  return {
    cluster,
    tier,
    humanAmount: amount,
    usdcMint: usdcRaw ? new PublicKey(usdcRaw) : USDC_MINT_DEVNET_DEFAULT,
    walletPath: get("--wallet") ?? resolve(__dirname, "../deploy-keypair.json"),
  };
}

function clusterUrl(c: SeedArgs["cluster"]): string {
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

  const microAmount = humanUsdcToMicro(args.humanAmount);
  console.log(
    `[seed-reserve] tier=${args.tier} amount=${args.humanAmount} USDC (${microAmount.toString()} microUSDC) admin=${signer.publicKey.toBase58()}`
  );

  const accounts = buildSeedReserveAccounts(
    signer.publicKey,
    args.tier,
    args.usdcMint
  );
  const ix = await client.reserve.methods
    .seed(microAmount)
    .accounts(accounts)
    .instruction();

  const sig = await sendAndConfirmTransaction(
    connection,
    new Transaction().add(ix),
    [signer]
  );
  console.log(`[ok] seed → ${sig}`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
