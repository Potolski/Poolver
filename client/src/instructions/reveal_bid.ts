import BN from "bn.js";
import { PublicKey, TransactionInstruction } from "@solana/web3.js";

import { PoolverClient } from "../poolver";
import { buildRevealBidAccounts } from "./_accounts";

export interface RevealBidArgs {
  pool: PublicKey;
  month: number;
  bidAmount: BN;
  nonce: Uint8Array; // exactly 16 bytes, matches the secret used at commit
  usdcMint: PublicKey;
}

/**
 * Reveal the previously-committed bid. INV-14 is enforced on-chain via
 * sha256 recomputation; if the SDK-computed commit_hash drifts from the
 * Rust formula, this call will fail with `BidRevealMismatch`.
 */
export async function revealBidIx(
  client: PoolverClient,
  args: RevealBidArgs
): Promise<TransactionInstruction> {
  if (args.nonce.length !== 16) {
    throw new Error(`nonce must be 16 bytes; got ${args.nonce.length}`);
  }
  const accounts = buildRevealBidAccounts(
    client.authority,
    args.pool,
    args.month,
    args.usdcMint
  );
  return client.core.methods
    .revealBid(args.bidAmount, Array.from(args.nonce))
    .accounts(accounts as any)
    .instruction();
}
