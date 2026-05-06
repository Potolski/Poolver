import { PublicKey, TransactionInstruction } from "@solana/web3.js";

import { PoolverClient } from "../poolver";
import { buildMarkLatePaymentAccounts } from "./_accounts";

/**
 * Mark a participant as late after `month_end + 1d`. Charges the 200 bps
 * (2%) penalty and reroutes it to `pool.bid_credit_balance`. Permissionless;
 * keeper-driven.
 */
export async function markLatePaymentIx(
  client: PoolverClient,
  args: { pool: PublicKey; delinquent: PublicKey }
): Promise<TransactionInstruction> {
  const accounts = buildMarkLatePaymentAccounts(
    client.authority,
    args.pool,
    args.delinquent
  );
  return client.core.methods
    .markLatePayment()
    .accounts(accounts as any)
    .instruction();
}
