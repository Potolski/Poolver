// Server-only — DO NOT import from a Client Component or the admin
// keypair will leak into the browser bundle.
import "server-only";

import {
  Connection,
  Keypair,
  Transaction,
  VersionedTransaction,
} from "@solana/web3.js";
import bs58 from "bs58";
import { PoolverClient, type PoolverClientOpts } from "@poolver/client";

// Duck-typed Anchor Wallet for the admin keypair. Avoids importing the
// Node-only `Wallet` class from `@coral-xyz/anchor`, which Turbopack
// resolves to a bundle that doesn't export it.
function keypairWallet(keypair: Keypair) {
  return {
    publicKey: keypair.publicKey,
    payer: keypair,
    async signTransaction<T extends Transaction | VersionedTransaction>(
      tx: T
    ): Promise<T> {
      if (tx instanceof Transaction) {
        tx.partialSign(keypair);
      } else {
        (tx as VersionedTransaction).sign([keypair]);
      }
      return tx;
    },
    async signAllTransactions<T extends Transaction | VersionedTransaction>(
      txs: T[]
    ): Promise<T[]> {
      for (const tx of txs) {
        if (tx instanceof Transaction) {
          tx.partialSign(keypair);
        } else {
          (tx as VersionedTransaction).sign([keypair]);
        }
      }
      return txs;
    },
  };
}

let cachedKeypair: Keypair | null = null;
let cachedClient: { connection: Connection; client: PoolverClient } | null = null;

/**
 * Accepts the admin keypair in three forms (any whitespace trimmed):
 *
 *   1. Base58 of the 64-byte secret (88 chars; what `bs58.encode` produces)
 *   2. JSON array of 64 numbers (e.g. `[136,186,93,...]` — the format
 *      Solana CLI keypair files use)
 *   3. Comma-separated 64 numbers without brackets (`136,186,93,...`) —
 *      sometimes Vercel's env-var UI strips brackets
 *
 * The flexibility avoids the silent-truncation traps when copy-pasting
 * into Vercel: a base58 string with a missing leading char decodes to
 * fewer bytes, and the resulting error doesn't tell you what got
 * dropped. The JSON / CSV form makes the byte count obvious.
 */
export function getAdminKeypair(): Keypair {
  if (cachedKeypair) return cachedKeypair;
  const raw = process.env.ADMIN_KEYPAIR_BASE58;
  if (!raw) {
    throw new Error(
      "ADMIN_KEYPAIR_BASE58 is not set. Populate it in .env.local (or your Vercel env) with the base58 encoding of the deploy keypair's secret key, or the JSON array form."
    );
  }
  const trimmed = raw.trim();
  let secretBytes: Uint8Array | null = null;
  let lastError = "";

  // Form 2 / 3 — JSON array or comma-separated numbers
  if (/^[\[\d]/.test(trimmed)) {
    try {
      const normalized = trimmed.startsWith("[") ? trimmed : `[${trimmed}]`;
      const arr = JSON.parse(normalized);
      if (Array.isArray(arr) && arr.every((n) => typeof n === "number")) {
        secretBytes = Uint8Array.from(arr);
      }
    } catch (e) {
      lastError = e instanceof Error ? e.message : String(e);
    }
  }

  // Form 1 — base58 (default and most common)
  if (!secretBytes) {
    try {
      secretBytes = bs58.decode(trimmed);
    } catch (e) {
      throw new Error(
        `ADMIN_KEYPAIR_BASE58 could not be parsed. Tried JSON array (${
          lastError || "n/a"
        }) and base58 (${e instanceof Error ? e.message : String(e)}). ` +
          `Got ${trimmed.length} chars: starts with "${trimmed.slice(
            0,
            8
          )}…" ends with "…${trimmed.slice(-8)}".`
      );
    }
  }

  if (secretBytes.length !== 64) {
    throw new Error(
      `ADMIN_KEYPAIR_BASE58 must decode to 64 bytes; got ${secretBytes.length}. ` +
        `Source had ${trimmed.length} chars. ` +
        `Tip: from a Solana CLI keypair JSON file, you can paste the array verbatim — e.g. \`[136,186,93,...]\` — and skip base58 entirely.`
    );
  }
  cachedKeypair = Keypair.fromSecretKey(secretBytes);
  return cachedKeypair;
}

export function getRpcUrl(): string {
  return (
    process.env.SOLANA_RPC ??
    process.env.NEXT_PUBLIC_SOLANA_RPC ??
    "https://api.devnet.solana.com"
  );
}

export function getAdminClient(): {
  client: PoolverClient;
  connection: Connection;
  keypair: Keypair;
} {
  if (cachedClient) {
    return {
      client: cachedClient.client,
      connection: cachedClient.connection,
      keypair: getAdminKeypair(),
    };
  }
  const keypair = getAdminKeypair();
  const connection = new Connection(getRpcUrl(), "confirmed");
  const wallet = keypairWallet(keypair);
  const client = new PoolverClient({
    connection,
    wallet: wallet as unknown as PoolverClientOpts["wallet"],
  });
  cachedClient = { connection, client };
  return { client, connection, keypair };
}
