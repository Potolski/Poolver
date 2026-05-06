import { PublicKey, TransactionInstruction } from "@solana/web3.js";

import { PoolverClient } from "../poolver";
import { TierName } from "../constants";
import { adapterTailRemaining, buildContributeAccounts } from "./_accounts";

export interface ContributeArgs {
  pool: PublicKey;
  tier: TierName;
  usdcMint: PublicKey;
}

/** Pay the monthly contribution. Must be called inside the active month window. */
export async function contributeIx(
  client: PoolverClient,
  args: ContributeArgs
): Promise<TransactionInstruction> {
  const accounts = buildContributeAccounts(
    client.authority,
    args.pool,
    args.tier,
    args.usdcMint
  );
  const tail = adapterTailRemaining(args.tier, args.pool);
  return client.core.methods
    .contribute()
    .accounts(accounts as any)
    .remainingAccounts(tail)
    .instruction();
}
