#!/usr/bin/env npx tsx
/**
 * Admin emergency top-up of a pool's yield-adapter USDC vault.
 *
 * Mock-USDC only. The admin deploy-keypair holds mint authority on
 * `B6dnuZtKH7FsSK6tySfWkk6ReW2LdKpmnfGAoMKsv8w8`, so we can mint USDC
 * directly into the adapter's token account. The adapter's `withdraw`
 * handler verifies live SPL balance (not the `total_deposited` ledger),
 * so this unblocks `claim_winning` without any on-chain change.
 *
 * Use this to recover from a demo state where months were advanced
 * without all 12 wallets contributing.
 *
 * Usage:
 *   npx tsx scripts/topup-adapter.ts \
 *     --pool <pool_pubkey> \
 *     --amount <human_usdc>           # e.g. 35000 (no decimals)
 *     --rpc "https://devnet.helius-rpc.com/?api-key=..." \
 *     --wallet ./deploy-keypair.json
 */
import {
  Connection,
  Keypair,
  PublicKey,
  sendAndConfirmTransaction,
  Transaction,
} from "@solana/web3.js";
import {
  createMintToInstruction,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { AnchorProvider, BN, Wallet } from "@coral-xyz/anchor";
import { readFileSync } from "fs";
import { resolve } from "path";

import {
  PoolverClient,
  POOLVER_YIELD_VAULT_PROGRAM_ID,
  POOLVER_YIELD_DEFI_PROGRAM_ID,
  USDC_MINT_DEVNET_DEFAULT,
  fetchPool,
  microUsdcToHuman,
  humanUsdcToMicro,
} from "../client/src";

function get(k: string): string | undefined {
  const i = process.argv.indexOf(k);
  return i >= 0 ? process.argv[i + 1] : undefined;
}

function loadKeypair(path: string): Keypair {
  const json = JSON.parse(readFileSync(resolve(path), "utf8"));
  return Keypair.fromSecretKey(Uint8Array.from(json));
}

async function main() {
  const poolArg = get("--pool");
  const amountArg = get("--amount");
  if (!poolArg || !amountArg) {
    console.error("usage: --pool <pubkey> --amount <human-usdc>");
    process.exit(1);
  }

  const rpc = get("--rpc") ?? "https://api.devnet.solana.com";
  const walletPath = get("--wallet") ?? "./deploy-keypair.json";
  const conn = new Connection(rpc, "confirmed");
  const admin = loadKeypair(walletPath);

  const provider = new AnchorProvider(conn, new Wallet(admin), {
    commitment: "confirmed",
  });
  const client = new PoolverClient(provider);

  const poolPk = new PublicKey(poolArg);
  const pool = await fetchPool(client, poolPk);

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

  const amountMicro = humanUsdcToMicro(Number(amountArg));

  console.log("Admin wallet:        ", admin.publicKey.toBase58());
  console.log("Pool:                ", poolPk.toBase58());
  console.log("Tier:                ", pool.tier);
  console.log("Adapter USDC vault:  ", adapterUsdcVault.toBase58());
  console.log("Mint:                ", USDC_MINT_DEVNET_DEFAULT.toBase58());
  console.log("Top-up amount:       ", `${amountArg} USDC`);

  const ix = createMintToInstruction(
    USDC_MINT_DEVNET_DEFAULT,
    adapterUsdcVault,
    admin.publicKey,
    BigInt(amountMicro.toString()),
    [],
    TOKEN_PROGRAM_ID
  );
  const tx = new Transaction().add(ix);
  const sig = await sendAndConfirmTransaction(conn, tx, [admin], {
    commitment: "confirmed",
  });
  console.log("\n✓ minted →", sig);
  console.log(
    "  https://explorer.solana.com/tx/" + sig + "?cluster=devnet"
  );
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
