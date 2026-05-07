import {
  PublicKey,
  Transaction,
  TransactionInstruction,
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
