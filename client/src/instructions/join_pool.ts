import { PublicKey, TransactionInstruction } from "@solana/web3.js";

import { PoolverClient } from "../poolver";
import { TierName } from "../constants";
import { adapterTailRemaining, buildJoinPoolAccounts } from "./_accounts";

export interface JoinPoolArgs {
  pool: PublicKey;
  tier: TierName;
  usdcMint: PublicKey;
}

/**
 * Build `join_pool`. The user must already have:
 *   - a Light KYC attestation (`mockIssueKyc` in V1)
 *   - a `UserReputation` account (`initializeUserReputation`)
 *   - a USDC ATA funded with at least `contribution + protocol_fee +
 *     reserve_fee`.
 */
export async function joinPoolIx(
  client: PoolverClient,
  args: JoinPoolArgs
): Promise<TransactionInstruction> {
  const accounts = buildJoinPoolAccounts(
    client.authority,
    args.pool,
    args.tier,
    args.usdcMint
  );
  const tail = adapterTailRemaining(args.tier, args.pool);
  return client.core.methods
    .joinPool()
    .accounts(accounts as any)
    .remainingAccounts(tail)
    .instruction();
}
