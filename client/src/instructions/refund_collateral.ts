import { PublicKey, TransactionInstruction } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync } from "@solana/spl-token";

import { PoolverClient } from "../poolver";
import {
  findCollateralVault,
  findParticipant,
} from "../pdas";

export interface RefundCollateralArgs {
  pool: PublicKey;
  participant: PublicKey;
  usdcMint: PublicKey;
}

/**
 * Refund a non-defaulting participant's locked collateral after the
 * pool has completed. Permissionless — caller can be anyone; the
 * refund still goes to `participant`'s USDC ATA.
 */
export async function refundCollateralIx(
  client: PoolverClient,
  args: RefundCollateralArgs
): Promise<TransactionInstruction> {
  const [participantPda] = findParticipant(args.pool, args.participant);
  const [collateralVault] = findCollateralVault(args.pool);
  const participantUsdc = getAssociatedTokenAddressSync(
    args.usdcMint,
    args.participant
  );
  return client.core.methods
    .refundCollateral()
    .accounts({
      caller: client.authority,
      pool: args.pool,
      participant: participantPda,
      participantUsdc,
      collateralVault,
      tokenProgram: TOKEN_PROGRAM_ID,
    } as any)
    .instruction();
}
