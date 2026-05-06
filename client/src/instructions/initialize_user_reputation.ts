import { TransactionInstruction } from "@solana/web3.js";
import { PoolverClient } from "../poolver";
import { buildInitializeUserReputationAccounts } from "./_accounts";

/**
 * Build the per-user `initialize_user_reputation` instruction.
 *
 * Idempotent-by-rejection: re-issuing for the same user fails with
 * `account already in use`. This add-on instruction was introduced
 * because Anchor's `init_if_needed` is banned by spec §9.10 (cf.
 * SPEC_QUESTION-12). Call once before the user's first `join_pool`.
 */
export async function initializeUserReputationIx(
  client: PoolverClient
): Promise<TransactionInstruction> {
  const accounts = buildInitializeUserReputationAccounts(client.authority);
  return client.core.methods
    .initializeUserReputation()
    .accounts(accounts as any)
    .instruction();
}
