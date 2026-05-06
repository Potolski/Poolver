import { PublicKey, TransactionInstruction } from "@solana/web3.js";

import { PoolverClient } from "../poolver";
import { TierName } from "../constants";
import {
  adapterTailRemaining,
  buildDistributeYieldAccounts,
} from "./_accounts";

export interface DistributeYieldArgs {
  pool: PublicKey;
  tier: TierName;
}

/**
 * Build `distribute_yield`. Permissionless once the harvest cadence
 * elapses (spec §5.3). Splits net yield across pool / reserve / protocol
 * per tier-dependent shares (arch §11).
 */
export async function distributeYieldIx(
  client: PoolverClient,
  args: DistributeYieldArgs
): Promise<TransactionInstruction> {
  const accounts = buildDistributeYieldAccounts(
    client.authority,
    args.pool,
    args.tier
  );
  const tail = adapterTailRemaining(args.tier, args.pool);
  return client.core.methods
    .distributeYield()
    .accounts(accounts as any)
    .remainingAccounts(tail)
    .instruction();
}
