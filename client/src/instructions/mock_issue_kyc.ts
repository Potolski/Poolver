import { PublicKey, TransactionInstruction } from "@solana/web3.js";
import { PoolverClient } from "../poolver";
import { KycLevelName, kycLevelToIdl } from "../constants";
import { buildMockIssueKycAccounts } from "./_accounts";

/**
 * Mint a KYC attestation in V1 — admin-signed.
 *
 * NOT AVAILABLE on mainnet builds (`--no-default-features`). The IDL
 * fetched from the cluster will be missing the `mock_issue_kyc` ix on
 * production deployments, and `program.methods.mockIssueKyc(...)` will
 * throw "Method not found" at call time. See `docs/mock-to-production.md`.
 */
export async function mockIssueKycIx(
  client: PoolverClient,
  args: { user: PublicKey; level: KycLevelName }
): Promise<TransactionInstruction> {
  const accounts = buildMockIssueKycAccounts(client.authority, args.user);
  return client.core.methods
    .mockIssueKyc(args.user, kycLevelToIdl(args.level))
    .accounts(accounts as any)
    .instruction();
}
