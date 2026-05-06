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
}

/**
 * Build `claim_winning`. Caller is the winner; they must claim within
 * the 24h window (spec §5.1). Posts collateral and pulls
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
    .claimWinning()
    .accounts(accounts as any)
    .remainingAccounts(tail)
    .instruction();
}
