// ───── Top-level public surface of @poolver/client ─────────────────────

export { PoolverClient } from "./poolver";
export type { PoolverClientOpts } from "./poolver";

// Constants & types
export * from "./constants";
export * from "./types";
export * from "./pdas";

// Utilities
export {
  buildBidCommitHash,
  randomBidNonce,
  BID_NONCE_LEN,
} from "./utils/bid_hash";
export { humanUsdcToMicro, microUsdcToHuman } from "./utils/constants";

// Instruction builders (high-level — return TransactionInstruction)
export { initializeProtocolIx } from "./instructions/initialize_protocol";
export { mockIssueKycIx } from "./instructions/mock_issue_kyc";
export { initializeUserReputationIx } from "./instructions/initialize_user_reputation";
export { createPoolIx } from "./instructions/create_pool";
export type { CreatePoolArgs } from "./instructions/create_pool";
export { joinPoolIx } from "./instructions/join_pool";
export type { JoinPoolArgs } from "./instructions/join_pool";
export { contributeIx } from "./instructions/contribute";
export type { ContributeArgs } from "./instructions/contribute";
export { advanceMonthIx } from "./instructions/advance_month";
export { adminSkipPhaseIx } from "./instructions/admin_skip_phase";
export { refundCollateralIx } from "./instructions/refund_collateral";
export { commitBidIx } from "./instructions/commit_bid";
export type { CommitBidArgs, CommitBidPlan } from "./instructions/commit_bid";
export { revealBidIx } from "./instructions/reveal_bid";
export type { RevealBidArgs } from "./instructions/reveal_bid";
export { selectWinnerIx } from "./instructions/select_winner";
export type { SelectWinnerArgs } from "./instructions/select_winner";
export { claimWinningIx } from "./instructions/claim_winning";
export type { ClaimWinningArgs } from "./instructions/claim_winning";
export { distributeYieldIx } from "./instructions/distribute_yield";
export type { DistributeYieldArgs } from "./instructions/distribute_yield";
export { markLatePaymentIx } from "./instructions/mark_late_payment";
export { suspendParticipantIx } from "./instructions/suspend_participant";
export { liquidateDefaultIx } from "./instructions/liquidate_default";

// Account-context builders (low-level — for callers composing custom txs)
export * from "./instructions/_accounts";

// Queries
export {
  fetchPool,
  fetchPoolByCreatorAndId,
  computeMonthState,
  countFilledParticipants,
} from "./queries/pool";
export type { PoolView, PoolMonthState } from "./queries/pool";
export { fetchParticipant, hasPaidMonth } from "./queries/participant";
export type { ParticipantView } from "./queries/participant";
export { fetchReserveFund } from "./queries/reserve";
export type { ReserveFundView } from "./queries/reserve";
export { fetchUserReputation } from "./queries/reputation";
export type { UserReputationView } from "./queries/reputation";
export { fetchKycAttestation, isKycValid } from "./queries/kyc";
export type { KycAttestationView } from "./queries/kyc";
