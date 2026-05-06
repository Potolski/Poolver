import { PublicKey, TransactionInstruction } from "@solana/web3.js";

import { PoolverClient } from "../poolver";
import { buildSuspendParticipantAccounts } from "./_accounts";

/**
 * Suspend a participant who is past day-6 unpaid status. Suspended
 * participants are gated out of bidding/claiming until they cure or
 * are liquidated.
 */
export async function suspendParticipantIx(
  client: PoolverClient,
  args: { pool: PublicKey; delinquent: PublicKey }
): Promise<TransactionInstruction> {
  const accounts = buildSuspendParticipantAccounts(
    client.authority,
    args.pool,
    args.delinquent
  );
  return client.core.methods
    .suspendParticipant()
    .accounts(accounts as any)
    .instruction();
}
