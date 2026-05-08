#!/usr/bin/env npx tsx
/**
 * One-time bootstrap of the Address Lookup Table that select_winner uses
 * to fit a 12-non-bidder lottery draw under the 1232-byte tx size cap.
 *
 * Holds the 8 protocol-static addresses:
 *   - protocolConfig (PDA)
 *   - coreInvoker   (PDA)
 *   - reserveFund   ×2 (Vault + DeFi)
 *   - reserveVault  ×2 (Vault + DeFi)
 *   - reserveProgram (program id)
 *   - tokenProgram   (program id)
 *
 * `bid_stake_vault` is per-pool so it stays out of the ALT.
 *
 * Idempotent: re-running with the same wallet on the same slot will print
 * the existing ALT address if you've already extended it. To create a
 * brand-new ALT, delete the cached address from constants and run again.
 *
 * Usage:
 *   npx tsx scripts/setup-alt.ts \
 *     --rpc "https://devnet.helius-rpc.com/?api-key=..." \
 *     --wallet ./deploy-keypair.json
 */
import {
  AddressLookupTableProgram,
  Connection,
  Keypair,
  PublicKey,
  sendAndConfirmTransaction,
  TransactionMessage,
  VersionedTransaction,
} from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { readFileSync } from "fs";
import { resolve } from "path";

import {
  POOLVER_RESERVE_PROGRAM_ID,
  findCoreInvoker,
  findProtocolConfig,
  findReserveFund,
  findReserveVault,
} from "../client/src";

interface Args {
  rpc: string;
  walletPath: string;
}

function parseArgs(): Args {
  const argv = process.argv.slice(2);
  const get = (k: string): string | undefined => {
    const i = argv.indexOf(k);
    return i >= 0 ? argv[i + 1] : undefined;
  };
  const rpc = get("--rpc") ?? "https://api.devnet.solana.com";
  const walletPath = resolve(get("--wallet") ?? "./deploy-keypair.json");
  return { rpc, walletPath };
}

function loadKeypair(path: string): Keypair {
  const json = JSON.parse(readFileSync(path, "utf8"));
  return Keypair.fromSecretKey(Uint8Array.from(json));
}

async function main() {
  const { rpc, walletPath } = parseArgs();
  const connection = new Connection(rpc, "confirmed");
  const payer = loadKeypair(walletPath);

  const [protocolConfig] = findProtocolConfig();
  const [coreInvoker] = findCoreInvoker();
  const [reserveFundVault] = findReserveFund("vault");
  const [reserveFundDefi] = findReserveFund("defi");
  const [reserveVaultVault] = findReserveVault("vault");
  const [reserveVaultDefi] = findReserveVault("defi");

  const entries: PublicKey[] = [
    protocolConfig,
    coreInvoker,
    reserveFundVault,
    reserveFundDefi,
    reserveVaultVault,
    reserveVaultDefi,
    POOLVER_RESERVE_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
  ];

  console.log("Using payer:", payer.publicKey.toBase58());
  console.log("RPC:        ", rpc);
  console.log("Entries (8):");
  for (const e of entries) console.log("  -", e.toBase58());

  const slot = await connection.getSlot("finalized");
  const [createIx, lookupTableAddress] =
    AddressLookupTableProgram.createLookupTable({
      authority: payer.publicKey,
      payer: payer.publicKey,
      recentSlot: slot,
    });

  const extendIx = AddressLookupTableProgram.extendLookupTable({
    payer: payer.publicKey,
    authority: payer.publicKey,
    lookupTable: lookupTableAddress,
    addresses: entries,
  });

  const blockhash = (await connection.getLatestBlockhash()).blockhash;
  const message = new TransactionMessage({
    payerKey: payer.publicKey,
    recentBlockhash: blockhash,
    instructions: [createIx, extendIx],
  }).compileToV0Message();

  const tx = new VersionedTransaction(message);
  tx.sign([payer]);

  const sig = await connection.sendRawTransaction(tx.serialize());
  await connection.confirmTransaction(sig, "confirmed");

  console.log("\n✓ Created + extended ALT");
  console.log("  address:", lookupTableAddress.toBase58());
  console.log("  sig:    ", sig);
  console.log(
    "\nAdd this to client/src/constants.ts as POOLVER_ALT_DEVNET."
  );
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
