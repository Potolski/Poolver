import { PublicKey, TransactionInstruction } from "@solana/web3.js";

import { PoolverClient } from "../poolver";
import { findProtocolConfig } from "../pdas";

/**
 * Admin-only fast-forward of a pool's current phase. Devnet/dev only —
 * detects whether we're in the bid window, reveal window, or post-reveal
 * waiting-for-month-end, and mutates the relevant timestamp so the next
 * regular instruction (commit / reveal / select_winner / advance_month)
 * passes its time check.
 *
 * Caller must equal `protocol_config.admin`.
 */
export async function adminSkipPhaseIx(
  client: PoolverClient,
  args: { pool: PublicKey }
): Promise<TransactionInstruction> {
  const [protocolConfig] = findProtocolConfig();
  return client.core.methods
    .adminSkipPhase()
    .accounts({
      admin: client.authority,
      protocolConfig,
      pool: args.pool,
    } as any)
    .instruction();
}
