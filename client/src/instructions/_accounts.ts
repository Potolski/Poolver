/**
 * Account-context builders.
 *
 * Each function returns the `accounts` map that `program.methods.<verb>(...)`
 * expects. The naming intentionally mirrors the Rust `#[derive(Accounts)]`
 * struct field names so a future engineer can grep the SDK against the
 * program with confidence.
 *
 * For tier-aware instructions (`createPool`, `joinPool`, `contribute`,
 * `claimWinning`, `distributeYield`), this module also produces the
 * `remainingAccounts` array per arch §13 / SPEC_QUESTION-36:
 *
 *   - Tier 0 (Vault): empty
 *   - Tier 1 (DeFi):  [adapter_ktoken_vault]   (single extra account)
 */
import { PublicKey, AccountMeta } from "@solana/web3.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import BN from "bn.js";

import {
  POOLVER_RESERVE_PROGRAM_ID,
  POOLVER_YIELD_DEFI_PROGRAM_ID,
  POOLVER_YIELD_VAULT_PROGRAM_ID,
  TierName,
} from "../constants";
import {
  adapterProgramId,
  findAdapterState,
  findAdapterUsdc,
  findBid,
  findBidStakeVault,
  findCollateralVault,
  findCoreInvoker,
  findDefiAdapterKtoken,
  findKycAttestation,
  findParticipant,
  findPool,
  findPoolUsdcVault,
  findProtocolConfig,
  findProtocolFeeVault,
  findReserveFund,
  findReserveVault,
  findUserReputation,
} from "../pdas";

const SYSTEM_PROGRAM_ID = new PublicKey("11111111111111111111111111111111");
const RENT_SYSVAR_ID = new PublicKey("SysvarRent111111111111111111111111111111111");

/** Adapter "tail" `remaining_accounts` per arch §13 (Tier 1 only). */
export function adapterTailRemaining(
  tier: TierName,
  pool: PublicKey
): AccountMeta[] {
  if (tier === "vault") return [];
  // Tier 1 (DeFi): the kToken-side vault (mock holds the "deployed" 75%).
  const [ktoken] = findDefiAdapterKtoken(pool);
  return [{ pubkey: ktoken, isSigner: false, isWritable: true }];
}

// ─────────────────────────── Singletons ───────────────────────────────

export interface InitializeProtocolAccounts {
  admin: PublicKey;
  protocolConfig: PublicKey;
  protocolFeeVault: PublicKey;
  usdcMint: PublicKey;
  systemProgram: PublicKey;
  tokenProgram: PublicKey;
  rent: PublicKey;
}

export function buildInitializeProtocolAccounts(
  admin: PublicKey,
  usdcMint: PublicKey
): InitializeProtocolAccounts {
  const [protocolConfig] = findProtocolConfig();
  const [protocolFeeVault] = findProtocolFeeVault();
  return {
    admin,
    protocolConfig,
    protocolFeeVault,
    usdcMint,
    systemProgram: SYSTEM_PROGRAM_ID,
    tokenProgram: TOKEN_PROGRAM_ID,
    rent: RENT_SYSVAR_ID,
  };
}

// ─────────────────────────── KYC + Reputation ─────────────────────────

export interface MockIssueKycAccounts {
  admin: PublicKey;
  protocolConfig: PublicKey;
  /** The recipient's wallet pubkey. Rust handler types this as an
   *  UncheckedAccount (it doesn't need to exist on chain yet, so
   *  Anchor can't auto-resolve it). */
  userPubkey: PublicKey;
  /** PDA-derived KycAttestation. IDL field name is `attestation`. */
  attestation: PublicKey;
  systemProgram: PublicKey;
}

/** `user` is the recipient pubkey; `admin` is the kyc_oracle (== admin in V1). */
export function buildMockIssueKycAccounts(
  admin: PublicKey,
  user: PublicKey
): MockIssueKycAccounts {
  const [protocolConfig] = findProtocolConfig();
  const [attestation] = findKycAttestation(user);
  return {
    admin,
    protocolConfig,
    userPubkey: user,
    attestation,
    systemProgram: SYSTEM_PROGRAM_ID,
  };
}

export interface InitializeUserReputationAccounts {
  user: PublicKey;
  reputation: PublicKey;
  systemProgram: PublicKey;
}

export function buildInitializeUserReputationAccounts(
  user: PublicKey
): InitializeUserReputationAccounts {
  const [reputation] = findUserReputation(user);
  return { user, reputation, systemProgram: SYSTEM_PROGRAM_ID };
}

// ─────────────────────────── Pool lifecycle ───────────────────────────

export interface CreatePoolAccounts {
  creator: PublicKey;
  protocolConfig: PublicKey;
  creatorKyc: PublicKey;
  creatorReputation: PublicKey;
  pool: PublicKey;
  poolUsdcVault: PublicKey;
  collateralVault: PublicKey;
  bidStakeVault: PublicKey;
  usdcMint: PublicKey;
  coreInvoker: PublicKey;
  adapterState: PublicKey;
  adapterUsdcVault: PublicKey;
  yieldAdapterProgram: PublicKey;
  systemProgram: PublicKey;
  tokenProgram: PublicKey;
  rent: PublicKey;
}

/**
 * `create_pool` accounts. Tier dispatch determines `adapterState`,
 * `adapterUsdcVault`, and `yieldAdapterProgram`. Tier 1 callers MUST
 * append `remainingAccounts = adapterTailRemaining("defi", pool)`.
 */
export function buildCreatePoolAccounts(
  creator: PublicKey,
  poolId: BN,
  tier: TierName,
  usdcMint: PublicKey
): { accounts: CreatePoolAccounts; pool: PublicKey } {
  const [protocolConfig] = findProtocolConfig();
  const [creatorKyc] = findKycAttestation(creator);
  const [creatorReputation] = findUserReputation(creator);
  const [pool] = findPool(creator, poolId);
  const [poolUsdcVault] = findPoolUsdcVault(pool);
  const [collateralVault] = findCollateralVault(pool);
  const [bidStakeVault] = findBidStakeVault(pool);
  const [coreInvoker] = findCoreInvoker();
  const [adapterState] = findAdapterState(tier, pool);
  const [adapterUsdcVault] = findAdapterUsdc(tier, pool);

  return {
    accounts: {
      creator,
      protocolConfig,
      creatorKyc,
      creatorReputation,
      pool,
      poolUsdcVault,
      collateralVault,
      bidStakeVault,
      usdcMint,
      coreInvoker,
      adapterState,
      adapterUsdcVault,
      yieldAdapterProgram: adapterProgramId(tier),
      systemProgram: SYSTEM_PROGRAM_ID,
      tokenProgram: TOKEN_PROGRAM_ID,
      rent: RENT_SYSVAR_ID,
    },
    pool,
  };
}

export interface JoinPoolAccounts {
  user: PublicKey;
  protocolConfig: PublicKey;
  userKyc: PublicKey;
  userReputation: PublicKey;
  pool: PublicKey;
  participant: PublicKey;
  userUsdc: PublicKey;
  poolUsdcVault: PublicKey;
  protocolFeeVault: PublicKey;
  coreInvoker: PublicKey;
  reserveFund: PublicKey;
  reserveUsdcVault: PublicKey;
  reserveProgram: PublicKey;
  adapterState: PublicKey;
  adapterUsdcVault: PublicKey;
  yieldAdapterProgram: PublicKey;
  systemProgram: PublicKey;
  tokenProgram: PublicKey;
  rent: PublicKey;
}

export function buildJoinPoolAccounts(
  user: PublicKey,
  pool: PublicKey,
  tier: TierName,
  usdcMint: PublicKey
): JoinPoolAccounts {
  const [protocolConfig] = findProtocolConfig();
  const [userKyc] = findKycAttestation(user);
  const [userReputation] = findUserReputation(user);
  const [participant] = findParticipant(pool, user);
  const [poolUsdcVault] = findPoolUsdcVault(pool);
  const [protocolFeeVault] = findProtocolFeeVault();
  const [coreInvoker] = findCoreInvoker();
  const [reserveFund] = findReserveFund(tier);
  const [reserveUsdcVault] = findReserveVault(tier);
  const [adapterState] = findAdapterState(tier, pool);
  const [adapterUsdcVault] = findAdapterUsdc(tier, pool);

  return {
    user,
    protocolConfig,
    userKyc,
    userReputation,
    pool,
    participant,
    userUsdc: getAssociatedTokenAddressSync(usdcMint, user),
    poolUsdcVault,
    protocolFeeVault,
    coreInvoker,
    reserveFund,
    reserveUsdcVault,
    reserveProgram: POOLVER_RESERVE_PROGRAM_ID,
    adapterState,
    adapterUsdcVault,
    yieldAdapterProgram: adapterProgramId(tier),
    systemProgram: SYSTEM_PROGRAM_ID,
    tokenProgram: TOKEN_PROGRAM_ID,
    rent: RENT_SYSVAR_ID,
  };
}

export interface ContributeAccounts extends JoinPoolAccounts {
  // Same shape as JoinPool — the on-chain layout reuses the same set
  // of accounts. (Field-by-field aliasing kept for IDE auto-complete.)
}

export function buildContributeAccounts(
  user: PublicKey,
  pool: PublicKey,
  tier: TierName,
  usdcMint: PublicKey
): ContributeAccounts {
  return buildJoinPoolAccounts(user, pool, tier, usdcMint);
}

export interface AdvanceMonthAccounts {
  caller: PublicKey;
  protocolConfig: PublicKey;
  pool: PublicKey;
}

export function buildAdvanceMonthAccounts(
  caller: PublicKey,
  pool: PublicKey
): AdvanceMonthAccounts {
  const [protocolConfig] = findProtocolConfig();
  return { caller, protocolConfig, pool };
}

// ─────────────────────────── Bidding ──────────────────────────────────

export interface CommitBidAccounts {
  user: PublicKey;
  pool: PublicKey;
  participant: PublicKey;
  bid: PublicKey;
  userUsdc: PublicKey;
  bidStakeVault: PublicKey;
  tokenProgram: PublicKey;
  systemProgram: PublicKey;
  rent: PublicKey;
}

export function buildCommitBidAccounts(
  user: PublicKey,
  pool: PublicKey,
  month: number,
  usdcMint: PublicKey
): CommitBidAccounts {
  const [participant] = findParticipant(pool, user);
  const [bid] = findBid(pool, month, user);
  const [bidStakeVault] = findBidStakeVault(pool);
  return {
    user,
    pool,
    participant,
    bid,
    userUsdc: getAssociatedTokenAddressSync(usdcMint, user),
    bidStakeVault,
    tokenProgram: TOKEN_PROGRAM_ID,
    systemProgram: SYSTEM_PROGRAM_ID,
    rent: RENT_SYSVAR_ID,
  };
}

export interface RevealBidAccounts {
  user: PublicKey;
  pool: PublicKey;
  participant: PublicKey;
  bid: PublicKey;
  userUsdc: PublicKey;
  bidStakeVault: PublicKey;
  tokenProgram: PublicKey;
}

export function buildRevealBidAccounts(
  user: PublicKey,
  pool: PublicKey,
  month: number,
  usdcMint: PublicKey
): RevealBidAccounts {
  const [participant] = findParticipant(pool, user);
  const [bid] = findBid(pool, month, user);
  const [bidStakeVault] = findBidStakeVault(pool);
  return {
    user,
    pool,
    participant,
    bid,
    userUsdc: getAssociatedTokenAddressSync(usdcMint, user),
    bidStakeVault,
    tokenProgram: TOKEN_PROGRAM_ID,
  };
}

// ─────────────────────────── Winner selection / claim ────────────────

export interface SelectWinnerAccounts {
  caller: PublicKey;
  protocolConfig: PublicKey;
  pool: PublicKey;
  bidStakeVault: PublicKey;
  coreInvoker: PublicKey;
  reserveFund: PublicKey;
  reserveUsdcVault: PublicKey;
  reserveProgram: PublicKey;
  tokenProgram: PublicKey;
}

export function buildSelectWinnerAccounts(
  caller: PublicKey,
  pool: PublicKey,
  tier: TierName
): SelectWinnerAccounts {
  const [protocolConfig] = findProtocolConfig();
  const [bidStakeVault] = findBidStakeVault(pool);
  const [coreInvoker] = findCoreInvoker();
  const [reserveFund] = findReserveFund(tier);
  const [reserveUsdcVault] = findReserveVault(tier);
  return {
    caller,
    protocolConfig,
    pool,
    bidStakeVault,
    coreInvoker,
    reserveFund,
    reserveUsdcVault,
    reserveProgram: POOLVER_RESERVE_PROGRAM_ID,
    tokenProgram: TOKEN_PROGRAM_ID,
  };
}

/**
 * `select_winner`'s remaining-accounts is self-describing per the on-chain
 * convention (see select_winner.rs module comment):
 *
 *   remaining = (
 *     [bid, participant, kyc] × N_committed_bids,
 *     [participant, kyc]      × M_non_bidder_candidates,
 *   )
 *
 * Pass EVERY `Bid` PDA for the current month (revealed or unrevealed —
 * unrevealed ones get their stake forfeit to the tier reserve in this
 * same ix). For the lottery path, also pass non-bidder participants so
 * the handler can find a candidate.
 *
 * `bidders` and `nonBidders` may overlap conceptually but should be
 * disjoint sets — bidders are wallets that called commit_bid for the
 * current month; non-bidders are everyone else still active.
 */
export function buildSelectWinnerRemainingAccounts(
  pool: PublicKey,
  month: number,
  bidders: PublicKey[],
  nonBidders: PublicKey[] = []
): AccountMeta[] {
  const out: AccountMeta[] = [];
  for (const bidder of bidders) {
    const [bid] = findBid(pool, month, bidder);
    const [participant] = findParticipant(pool, bidder);
    const [kyc] = findKycAttestation(bidder);
    out.push({ pubkey: bid, isSigner: false, isWritable: true });
    out.push({ pubkey: participant, isSigner: false, isWritable: false });
    out.push({ pubkey: kyc, isSigner: false, isWritable: false });
  }
  for (const user of nonBidders) {
    const [participant] = findParticipant(pool, user);
    const [kyc] = findKycAttestation(user);
    out.push({ pubkey: participant, isSigner: false, isWritable: false });
    out.push({ pubkey: kyc, isSigner: false, isWritable: false });
  }
  return out;
}

export interface ClaimWinningAccounts {
  winner: PublicKey;
  pool: PublicKey;
  participant: PublicKey;
  protocolConfig: PublicKey;
  winnerUsdc: PublicKey;
  poolUsdcVault: PublicKey;
  collateralVault: PublicKey;
  protocolFeeVault: PublicKey;
  coreInvoker: PublicKey;
  reserveFund: PublicKey;
  reserveUsdcVault: PublicKey;
  reserveProgram: PublicKey;
  adapterState: PublicKey;
  adapterUsdcVault: PublicKey;
  yieldAdapterProgram: PublicKey;
  tokenProgram: PublicKey;
}

export function buildClaimWinningAccounts(
  winner: PublicKey,
  pool: PublicKey,
  tier: TierName,
  usdcMint: PublicKey
): ClaimWinningAccounts {
  const [participant] = findParticipant(pool, winner);
  const [protocolConfig] = findProtocolConfig();
  const [poolUsdcVault] = findPoolUsdcVault(pool);
  const [collateralVault] = findCollateralVault(pool);
  const [protocolFeeVault] = findProtocolFeeVault();
  const [coreInvoker] = findCoreInvoker();
  const [reserveFund] = findReserveFund(tier);
  const [reserveUsdcVault] = findReserveVault(tier);
  const [adapterState] = findAdapterState(tier, pool);
  const [adapterUsdcVault] = findAdapterUsdc(tier, pool);
  return {
    winner,
    pool,
    participant,
    protocolConfig,
    winnerUsdc: getAssociatedTokenAddressSync(usdcMint, winner),
    poolUsdcVault,
    collateralVault,
    protocolFeeVault,
    coreInvoker,
    reserveFund,
    reserveUsdcVault,
    reserveProgram: POOLVER_RESERVE_PROGRAM_ID,
    adapterState,
    adapterUsdcVault,
    yieldAdapterProgram: adapterProgramId(tier),
    tokenProgram: TOKEN_PROGRAM_ID,
  };
}

// ─────────────────────────── Yield ────────────────────────────────────

export interface DistributeYieldAccounts {
  caller: PublicKey;
  pool: PublicKey;
  protocolConfig: PublicKey;
  poolUsdcVault: PublicKey;
  protocolFeeVault: PublicKey;
  coreInvoker: PublicKey;
  reserveFund: PublicKey;
  reserveUsdcVault: PublicKey;
  reserveProgram: PublicKey;
  adapterState: PublicKey;
  adapterUsdcVault: PublicKey;
  yieldAdapterProgram: PublicKey;
  tokenProgram: PublicKey;
}

export function buildDistributeYieldAccounts(
  caller: PublicKey,
  pool: PublicKey,
  tier: TierName
): DistributeYieldAccounts {
  const [protocolConfig] = findProtocolConfig();
  const [poolUsdcVault] = findPoolUsdcVault(pool);
  const [protocolFeeVault] = findProtocolFeeVault();
  const [coreInvoker] = findCoreInvoker();
  const [reserveFund] = findReserveFund(tier);
  const [reserveUsdcVault] = findReserveVault(tier);
  const [adapterState] = findAdapterState(tier, pool);
  const [adapterUsdcVault] = findAdapterUsdc(tier, pool);
  return {
    caller,
    pool,
    protocolConfig,
    poolUsdcVault,
    protocolFeeVault,
    coreInvoker,
    reserveFund,
    reserveUsdcVault,
    reserveProgram: POOLVER_RESERVE_PROGRAM_ID,
    adapterState,
    adapterUsdcVault,
    yieldAdapterProgram: adapterProgramId(tier),
    tokenProgram: TOKEN_PROGRAM_ID,
  };
}

// ─────────────────────────── Default cascade ─────────────────────────

export interface MarkLatePaymentAccounts {
  caller: PublicKey;
  pool: PublicKey;
  participant: PublicKey;
  protocolConfig: PublicKey;
}

export function buildMarkLatePaymentAccounts(
  caller: PublicKey,
  pool: PublicKey,
  delinquent: PublicKey
): MarkLatePaymentAccounts {
  const [participant] = findParticipant(pool, delinquent);
  const [protocolConfig] = findProtocolConfig();
  return { caller, pool, participant, protocolConfig };
}

export interface SuspendParticipantAccounts {
  caller: PublicKey;
  pool: PublicKey;
  participant: PublicKey;
  reputation: PublicKey;
  protocolConfig: PublicKey;
}

export function buildSuspendParticipantAccounts(
  caller: PublicKey,
  pool: PublicKey,
  delinquent: PublicKey
): SuspendParticipantAccounts {
  const [participant] = findParticipant(pool, delinquent);
  const [reputation] = findUserReputation(delinquent);
  const [protocolConfig] = findProtocolConfig();
  return { caller, pool, participant, reputation, protocolConfig };
}

export interface LiquidateDefaultAccounts {
  caller: PublicKey;
  pool: PublicKey;
  participant: PublicKey;
  reputation: PublicKey;
  protocolConfig: PublicKey;
  poolUsdcVault: PublicKey;
  collateralVault: PublicKey;
  coreInvoker: PublicKey;
  reserveFund: PublicKey;
  reserveUsdcVault: PublicKey;
  reserveProgram: PublicKey;
  tokenProgram: PublicKey;
}

export function buildLiquidateDefaultAccounts(
  caller: PublicKey,
  pool: PublicKey,
  defaulter: PublicKey,
  tier: TierName
): LiquidateDefaultAccounts {
  const [participant] = findParticipant(pool, defaulter);
  const [reputation] = findUserReputation(defaulter);
  const [protocolConfig] = findProtocolConfig();
  const [poolUsdcVault] = findPoolUsdcVault(pool);
  const [collateralVault] = findCollateralVault(pool);
  const [coreInvoker] = findCoreInvoker();
  const [reserveFund] = findReserveFund(tier);
  const [reserveUsdcVault] = findReserveVault(tier);
  return {
    caller,
    pool,
    participant,
    reputation,
    protocolConfig,
    poolUsdcVault,
    collateralVault,
    coreInvoker,
    reserveFund,
    reserveUsdcVault,
    reserveProgram: POOLVER_RESERVE_PROGRAM_ID,
    tokenProgram: TOKEN_PROGRAM_ID,
  };
}

// ─────────────────────────── Reserve admin ───────────────────────────

export interface InitializeReserveAccounts {
  admin: PublicKey;
  reserveFund: PublicKey;
  reserveUsdcVault: PublicKey;
  usdcMint: PublicKey;
  systemProgram: PublicKey;
  tokenProgram: PublicKey;
  rent: PublicKey;
}

export function buildInitializeReserveAccounts(
  admin: PublicKey,
  tier: TierName,
  usdcMint: PublicKey
): InitializeReserveAccounts {
  const [reserveFund] = findReserveFund(tier);
  const [reserveUsdcVault] = findReserveVault(tier);
  return {
    admin,
    reserveFund,
    reserveUsdcVault,
    usdcMint,
    systemProgram: SYSTEM_PROGRAM_ID,
    tokenProgram: TOKEN_PROGRAM_ID,
    rent: RENT_SYSVAR_ID,
  };
}

export interface SeedReserveAccounts {
  admin: PublicKey;
  reserveFund: PublicKey;
  reserveUsdcVault: PublicKey;
  adminUsdc: PublicKey;
  tokenProgram: PublicKey;
}

export function buildSeedReserveAccounts(
  admin: PublicKey,
  tier: TierName,
  usdcMint: PublicKey
): SeedReserveAccounts {
  const [reserveFund] = findReserveFund(tier);
  const [reserveUsdcVault] = findReserveVault(tier);
  return {
    admin,
    reserveFund,
    reserveUsdcVault,
    adminUsdc: getAssociatedTokenAddressSync(usdcMint, admin),
    tokenProgram: TOKEN_PROGRAM_ID,
  };
}

/** Stable export of common system / SPL programs for callers. */
export const SDK_PROGRAMS = {
  systemProgram: SYSTEM_PROGRAM_ID,
  tokenProgram: TOKEN_PROGRAM_ID,
  associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  rent: RENT_SYSVAR_ID,
  reserveProgram: POOLVER_RESERVE_PROGRAM_ID,
  yieldVaultProgram: POOLVER_YIELD_VAULT_PROGRAM_ID,
  yieldDefiProgram: POOLVER_YIELD_DEFI_PROGRAM_ID,
};

// ─────────────────────────── Slash unpaid (V1) ────────────────────────

export interface SlashUnpaidAccounts {
  caller: PublicKey;
  protocolConfig: PublicKey;
  pool: PublicKey;
  participant: PublicKey;
  userReputation: PublicKey;
  collateralVault: PublicKey;
  poolUsdcVault: PublicKey;
  coreInvoker: PublicKey;
  adapterState: PublicKey;
  adapterUsdcVault: PublicKey;
  yieldAdapterProgram: PublicKey;
  tokenProgram: PublicKey;
}

export function buildSlashUnpaidAccounts(
  caller: PublicKey,
  pool: PublicKey,
  delinquent: PublicKey,
  tier: TierName
): SlashUnpaidAccounts {
  const [protocolConfig] = findProtocolConfig();
  const [participant] = findParticipant(pool, delinquent);
  const [userReputation] = findUserReputation(delinquent);
  const [collateralVault] = findCollateralVault(pool);
  const [poolUsdcVault] = findPoolUsdcVault(pool);
  const [coreInvoker] = findCoreInvoker();
  const [adapterState] = findAdapterState(tier, pool);
  const [adapterUsdcVault] = findAdapterUsdc(tier, pool);
  return {
    caller,
    protocolConfig,
    pool,
    participant,
    userReputation,
    collateralVault,
    poolUsdcVault,
    coreInvoker,
    adapterState,
    adapterUsdcVault,
    yieldAdapterProgram: adapterProgramId(tier),
    tokenProgram: TOKEN_PROGRAM_ID,
  };
}
