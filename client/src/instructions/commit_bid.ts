import BN from "bn.js";
import { PublicKey, TransactionInstruction } from "@solana/web3.js";

import { PoolverClient } from "../poolver";
import { buildCommitBidAccounts } from "./_accounts";
import { buildBidCommitHash, randomBidNonce } from "../utils/bid_hash";

export interface CommitBidArgs {
  pool: PublicKey;
  /** Pool month (1..=12) the bid is for. */
  month: number;
  usdcMint: PublicKey;
  /**
   * Either provide a precomputed `commitHash`, or pass `bidAmount` + (optional)
   * `nonce` to have the SDK compute it. Either way the caller MUST persist
   * `nonce` + `bidAmount` until reveal.
   */
  commitHash?: Uint8Array;
  bidAmount?: BN;
  nonce?: Uint8Array;
}

export interface CommitBidPlan {
  ix: TransactionInstruction;
  nonce: Uint8Array | null;
  bidAmount: BN | null;
  commitHash: Uint8Array;
}

/**
 * Plan a `commit_bid` call. Returns the instruction and the secret
 * material the caller must save off-chain to reveal later.
 */
export async function commitBidIx(
  client: PoolverClient,
  args: CommitBidArgs
): Promise<CommitBidPlan> {
  let commitHash: Uint8Array;
  let nonce: Uint8Array | null = null;
  let bidAmount: BN | null = null;

  if (args.commitHash) {
    commitHash = args.commitHash;
  } else {
    if (!args.bidAmount) {
      throw new Error("commitBidIx: either commitHash or bidAmount required");
    }
    nonce = args.nonce ?? randomBidNonce();
    bidAmount = args.bidAmount;
    commitHash = buildBidCommitHash(bidAmount, nonce, client.authority);
  }
  if (commitHash.length !== 32) {
    throw new Error("commit hash must be 32 bytes");
  }

  const accounts = buildCommitBidAccounts(
    client.authority,
    args.pool,
    args.month,
    args.usdcMint
  );

  const ix = await client.core.methods
    .commitBid(Array.from(commitHash))
    .accounts(accounts as any)
    .instruction();

  return { ix, nonce, bidAmount, commitHash };
}
