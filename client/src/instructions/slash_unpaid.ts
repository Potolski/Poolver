import { PublicKey, TransactionInstruction, AccountMeta } from "@solana/web3.js";

import { PoolverClient } from "../poolver";
import { TierName } from "../constants";
import { adapterTailRemaining, buildSlashUnpaidAccounts } from "./_accounts";

/**
 * Slash a participant's collateral for missing the current month's
 * contribution. Permissionless; callable as soon as the month duration
 * has elapsed (no grace period in V1). Forwards the slashed amount into
 * the yield adapter so the monthly pot stays whole.
 *
 * Tier-aware: for Tier 1 (DeFi) callers, the SDK appends the
 * `adapter_ktoken_vault` to `remaining_accounts` per arch §13.
 */
export async function slashUnpaidIx(
  client: PoolverClient,
  args: { pool: PublicKey; delinquent: PublicKey; tier: TierName }
): Promise<TransactionInstruction> {
  const accounts = buildSlashUnpaidAccounts(
    client.authority,
    args.pool,
    args.delinquent,
    args.tier
  );
  const remaining: AccountMeta[] = adapterTailRemaining(args.tier, args.pool);
  return client.core.methods
    .slashUnpaid()
    .accounts(accounts as any)
    .remainingAccounts(remaining)
    .instruction();
}
