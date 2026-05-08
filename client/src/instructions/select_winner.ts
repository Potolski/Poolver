import { PublicKey, TransactionInstruction } from "@solana/web3.js";

import { PoolverClient } from "../poolver";
import { TierName } from "../constants";
import {
  buildSelectWinnerAccounts,
  buildSelectWinnerRemainingAccounts,
} from "./_accounts";

export interface SelectWinnerArgs {
  pool: PublicKey;
  /** Pool tier — needed to derive the matching reserve PDAs (forfeit
   *  destination for unrevealed bid stakes). */
  tier: TierName;
  /** Current month being resolved (1..=12). */
  month: number;
  /**
   * EVERY participant who called `commit_bid` for this month — both
   * those who revealed and those who didn't (unrevealed bidders get
   * their stake forfeit to the tier reserve here). SDK derives Bid +
   * Participant + KYC PDAs for each.
   */
  bidders: PublicKey[];
  /**
   * Non-bidder participants. Required for the lottery branch (no
   * revealed bids) so the handler has candidates to draw from. Pass
   * every active participant who didn't bid; eligibility (no prior
   * win, not defaulted, KYC ok) is checked on-chain.
   */
  nonBidders: PublicKey[];
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
  const accounts = buildSelectWinnerAccounts(client.authority, args.pool, args.tier);
  const remaining = buildSelectWinnerRemainingAccounts(
    args.pool,
    args.month,
    args.bidders,
    args.nonBidders
  );
  return client.core.methods
    .selectWinner()
    .accounts(accounts as any)
    .remainingAccounts(remaining)
    .instruction();
}
