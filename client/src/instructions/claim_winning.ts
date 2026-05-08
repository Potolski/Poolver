import { PublicKey, TransactionInstruction } from "@solana/web3.js";

import { PoolverClient } from "../poolver";
import { TierName } from "../constants";
import {
  adapterTailRemaining,
  buildClaimWinningAccounts,
} from "./_accounts";

export interface ClaimWinningArgs {
  pool: PublicKey;
  tier: TierName;
  usdcMint: PublicKey;
  /** The month the caller is claiming for. Must be in [1, pool.currentMonth].
   *  Lets winners claim retroactively after the month has advanced —
   *  the protocol no longer requires claim within the same month. */
  claimMonth: number;
}

/**
 * Build `claim_winning`. Caller is the winner; passes the month they're
 * claiming for so retroactive claims work after `advance_month` has run.
 * Posts collateral and pulls
 * `net_payout = winning_bid - protocol_fee_5% - reserve_fee_20%`.
 */
export async function claimWinningIx(
  client: PoolverClient,
  args: ClaimWinningArgs
): Promise<TransactionInstruction> {
  const accounts = buildClaimWinningAccounts(
    client.authority,
    args.pool,
    args.tier,
    args.usdcMint
  );
  const tail = adapterTailRemaining(args.tier, args.pool);
  return client.core.methods
    .claimWinning(args.claimMonth)
    .accounts(accounts as any)
    .remainingAccounts(tail)
    .instruction();
}
