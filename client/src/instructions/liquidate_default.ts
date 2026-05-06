import { PublicKey, TransactionInstruction } from "@solana/web3.js";

import { PoolverClient } from "../poolver";
import { TierName } from "../constants";
import { buildLiquidateDefaultAccounts } from "./_accounts";

/**
 * Liquidate a participant who has been unpaid for >30 days. Drains their
 * collateral first, then taps the tier reserve for any shortfall (INV-4
 * tier isolation enforced via PDA seeds).
 */
export async function liquidateDefaultIx(
  client: PoolverClient,
  args: { pool: PublicKey; defaulter: PublicKey; tier: TierName }
): Promise<TransactionInstruction> {
  const accounts = buildLiquidateDefaultAccounts(
    client.authority,
    args.pool,
    args.defaulter,
    args.tier
  );
  return client.core.methods
    .liquidateDefault()
    .accounts(accounts as any)
    .instruction();
}
