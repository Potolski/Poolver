import { PublicKey, TransactionInstruction } from "@solana/web3.js";

import { PoolverClient } from "../poolver";
import {
  buildSelectWinnerAccounts,
  buildSelectWinnerRemainingAccounts,
} from "./_accounts";

export interface SelectWinnerArgs {
  pool: PublicKey;
  /** Current month being resolved (1..=12). */
  month: number;
  /**
   * The pubkeys of every participant who successfully `revealBid`'d for
   * this month. The SDK derives the `bid` PDA for each. If empty the
   * on-chain handler falls back to the (mock) VRF entropy path.
   */
  revealedBidders: PublicKey[];
}

/**
 * Build `select_winner`. Per arch §8, the `revealed Bid` set may be up to
 * 12 PDAs; for >8 callers should use an Address Lookup Table. This SDK
 * does NOT auto-construct the ALT — that's the keeper bot's job.
 */
export async function selectWinnerIx(
  client: PoolverClient,
  args: SelectWinnerArgs
): Promise<TransactionInstruction> {
  const accounts = buildSelectWinnerAccounts(client.authority, args.pool);
  const remaining = buildSelectWinnerRemainingAccounts(
    args.pool,
    args.month,
    args.revealedBidders
  );
  return client.core.methods
    .selectWinner()
    .accounts(accounts as any)
    .remainingAccounts(remaining)
    .instruction();
}
