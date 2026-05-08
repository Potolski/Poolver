import {
  AddressLookupTableAccount,
  PublicKey,
  Transaction,
  TransactionInstruction,
  TransactionMessage,
  VersionedTransaction,
} from "@solana/web3.js";
import {
  createAssociatedTokenAccountIdempotentInstruction,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import type { PoolverClient } from "@poolver/client";

export interface EnsureAtaResult {
  ata: PublicKey;
  preIx: TransactionInstruction;
}

/**
 * Build an idempotent ATA creation instruction. The on-chain `create_*`
 * idempotent helper makes this safe to include even when the ATA already
 * exists.
 */
export function ensureAtaIx(
  payer: PublicKey,
  owner: PublicKey,
  mint: PublicKey
): EnsureAtaResult {
  const ata = getAssociatedTokenAddressSync(mint, owner, true);
  const preIx = createAssociatedTokenAccountIdempotentInstruction(
    payer,
    ata,
    owner,
    mint
  );
  return { ata, preIx };
}

/**
 * Send a sequence of instructions as a single transaction via the wallet
 * already wired into the PoolverClient provider. Returns the signature.
 */
export async function sendIxs(
  client: PoolverClient,
  ixs: TransactionInstruction[]
): Promise<string> {
  if (ixs.length === 0) throw new Error("sendIxs: empty ix list");
  const tx = new Transaction().add(...ixs);
  return client.provider.sendAndConfirm!(tx, [], { commitment: "confirmed" });
}

/**
 * Send a versioned (v0) transaction with one or more Address Lookup
 * Tables. Lets a single ix include up to ~30 unique pubkeys without
 * blowing the 1232-byte wire cap — needed for `select_winner` when the
 * lottery branch has many non-bidder candidates.
 *
 * The wallet adapter handles signing transparently for v0 txes too —
 * Phantom / Solflare / Backpack all support them as of 2024-Q3.
 */
export async function sendIxsV0(
  client: PoolverClient,
  ixs: TransactionInstruction[],
  altAddresses: PublicKey[]
): Promise<string> {
  if (ixs.length === 0) throw new Error("sendIxsV0: empty ix list");
  const provider = client.provider;
  const wallet = provider.wallet;
  if (!wallet?.publicKey) {
    throw new Error("sendIxsV0: wallet not connected");
  }

  const lookupTables: AddressLookupTableAccount[] = [];
  for (const altPk of altAddresses) {
    const fetched = await provider.connection.getAddressLookupTable(altPk);
    if (!fetched.value) {
      throw new Error(
        `sendIxsV0: address lookup table ${altPk.toBase58()} not found`
      );
    }
    lookupTables.push(fetched.value);
  }

  const blockhash = (await provider.connection.getLatestBlockhash()).blockhash;
  const message = new TransactionMessage({
    payerKey: wallet.publicKey,
    recentBlockhash: blockhash,
    instructions: ixs,
  }).compileToV0Message(lookupTables);

  const tx = new VersionedTransaction(message);
  const signed = await wallet.signTransaction!(tx);
  const sig = await provider.connection.sendRawTransaction(signed.serialize());
  await provider.connection.confirmTransaction(sig, "confirmed");
  return sig;
}
