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

export function getAdminKeypair(): Keypair {
  if (cachedKeypair) return cachedKeypair;
  const raw = process.env.ADMIN_KEYPAIR_BASE58;
  if (!raw) {
    throw new Error(
      "ADMIN_KEYPAIR_BASE58 is not set. Populate it in .env.local with the base58 encoding of the deploy keypair's secret key."
    );
  }
  let secretBytes: Uint8Array;
  try {
    secretBytes = bs58.decode(raw.trim());
  } catch (e) {
    throw new Error(
      `ADMIN_KEYPAIR_BASE58 could not be base58-decoded: ${
        e instanceof Error ? e.message : String(e)
      }`
    );
  }
  if (secretBytes.length !== 64) {
    throw new Error(
      `ADMIN_KEYPAIR_BASE58 must decode to 64 bytes; got ${secretBytes.length}`
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
