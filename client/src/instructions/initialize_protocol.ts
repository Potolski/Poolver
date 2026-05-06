import { TransactionInstruction } from "@solana/web3.js";
import { PoolverClient } from "../poolver";
import { buildInitializeProtocolAccounts } from "./_accounts";

/**
 * Build the `initialize_protocol` instruction.
 *
 * Singleton — call once per cluster. Sets the admin (== `kyc_oracle` in V1)
 * and the `usdc_mint` that all subsequent pools inherit.
 */
export async function initializeProtocolIx(
  client: PoolverClient,
  args: { usdcMint: import("@solana/web3.js").PublicKey }
): Promise<TransactionInstruction> {
  const accounts = buildInitializeProtocolAccounts(
    client.authority,
    args.usdcMint
  );
  return client.core.methods
    .initializeProtocol()
    .accounts(accounts as any)
    .instruction();
}
