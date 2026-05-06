import BN from "bn.js";
import { PublicKey, TransactionInstruction } from "@solana/web3.js";

import { PoolverClient } from "../poolver";
import { TierName, tierToIdl } from "../constants";
import {
  adapterTailRemaining,
  buildCreatePoolAccounts,
} from "./_accounts";

export interface CreatePoolArgs {
  poolId: BN;
  tier: TierName;
  contributionAmount: BN; // microUSDC
  /**
   * Optional override; defaults to 30 days. Must match the bound
   * documented in spec §3 (24h..=90 days). The on-chain handler
   * validates.
   */
  monthDurationSeconds?: BN | null;
  usdcMint: PublicKey;
}

/**
 * Build the `create_pool` instruction.
 *
 * Returns the instruction AND the deterministic `pool` PDA so callers can
 * track it without re-deriving. Tier 1 callers automatically get the
 * `[adapter_ktoken_vault]` remaining-account appended (arch §13).
 */
export async function createPoolIx(
  client: PoolverClient,
  args: CreatePoolArgs
): Promise<{ ix: TransactionInstruction; pool: PublicKey }> {
  const { accounts, pool } = buildCreatePoolAccounts(
    client.authority,
    args.poolId,
    args.tier,
    args.usdcMint
  );

  const tail = adapterTailRemaining(args.tier, pool);

  const ix = await client.core.methods
    .createPool(
      args.poolId,
      tierToIdl(args.tier),
      args.contributionAmount,
      args.monthDurationSeconds ?? null
    )
    .accounts(accounts as any)
    .remainingAccounts(tail)
    .instruction();

  return { ix, pool };
}
