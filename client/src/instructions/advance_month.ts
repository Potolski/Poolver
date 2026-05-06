import { PublicKey, TransactionInstruction } from "@solana/web3.js";

import { PoolverClient } from "../poolver";
import { buildAdvanceMonthAccounts } from "./_accounts";

/**
 * Permissionless bump of `pool.current_month`. Anyone may call once the
 * current-month duration has elapsed. The keeper bot typically owns this.
 */
export async function advanceMonthIx(
  client: PoolverClient,
  args: { pool: PublicKey }
): Promise<TransactionInstruction> {
  const accounts = buildAdvanceMonthAccounts(client.authority, args.pool);
  return client.core.methods
    .advanceMonth()
    .accounts(accounts as any)
    .instruction();
}
