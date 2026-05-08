use anchor_lang::prelude::*;

use crate::state::{KycLevel, SelectionMethod, Tier};

// Spec §6: every state-changing instruction emits an indexer-rebuildable
// event. Step-4 only emits the events for the instructions in this step.
// Subsequent steps will append more.

#[event]
pub struct ProtocolInitialized {
    pub admin: Pubkey,
    pub kyc_oracle: Pubkey,
    pub usdc_mint: Pubkey,
    pub protocol_fee_vault: Pubkey,
    pub protocol_fee_bps: u16,
    pub vault_reserve_fee_bps: u16,
    pub defi_reserve_fee_bps: u16,
    pub timestamp: i64,
}

/// SPEC_QUESTION-26: emitted by `admin_close_protocol` when the admin tears
/// down the singleton `ProtocolConfig` + `protocol_fee_vault` ahead of a
/// re-`initialize_protocol` with a different USDC mint. Indexers should
/// treat this as "config rotation in progress"; a fresh `ProtocolInitialized`
/// will follow.
#[event]
pub struct ProtocolClosed {
    pub admin: Pubkey,
    pub protocol_config: Pubkey,
    pub protocol_fee_vault: Pubkey,
    pub timestamp: i64,
}

/// Admin fast-forwarded a phase via `admin_skip_phase`. Devnet only;
/// `phase` matches the `SkippedPhase` enum (0 = bid window, 1 = reveal
/// window, 2 = month duration).
#[event]
pub struct PhaseSkipped {
    pub pool: Pubkey,
    pub month: u8,
    pub phase: u8,
    pub timestamp: i64,
}

#[event]
pub struct KycAttestationIssued {
    pub user: Pubkey,
    pub level: KycLevel,
    pub issued_by: Pubkey,
    pub issued_at: i64,
    pub expires_at: i64,
    /// `true` if the attestation was issued via the V1 `mock_issue_kyc`
    /// path; `false` for production `issue_kyc_attestation` (not yet
    /// implemented). Indexers can colour rows by this flag.
    pub is_mock: bool,
}

#[event]
pub struct UserReputationInitialized {
    pub user: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct PoolCreated {
    pub pool: Pubkey,
    pub pool_id: u64,
    pub creator: Pubkey,
    pub tier: Tier,
    pub contribution_amount: u64,
    pub month_duration_seconds: i64,
    pub timestamp: i64,
}

#[event]
pub struct ParticipantJoined {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub slot_index: u8,
    pub gross_contribution: u64,
    pub protocol_fee: u64,
    pub reserve_fee: u64,
    pub net_to_pool: u64,
    pub completed_cycles_at_join: u8,
    pub timestamp: i64,
}

#[event]
pub struct PoolStarted {
    pub pool: Pubkey,
    pub start_timestamp: i64,
}

/// Spec §6 + §5.1 `contribute`. One emit per successful contribution.
/// Indexers can rebuild per-month payment status from `month` +
/// `paid_months_after`. `collateral_released` is non-zero only after the
/// participant has won and is paying down post-win schedule (spec §4).
#[event]
pub struct Contribution {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub month: u8,
    /// Gross USDC pulled from the user's source wallet (after applying
    /// any `bid_credit_balance` discount — Q-1).
    pub amount: u64,
    pub protocol_fee: u64,
    pub reserve_fee: u64,
    pub net_to_pool: u64,
    pub collateral_released: u64,
    pub paid_months_after: u16,
    pub timestamp: i64,
}

/// Spec §6 + §5.1 `advance_month`. Emitted on each successful month tick
/// (current_month → current_month + 1). Final tick (12 → 13) emits
/// `PoolCompleted` instead of this.
#[event]
pub struct MonthAdvanced {
    pub pool: Pubkey,
    pub new_month: u8,
    pub timestamp: i64,
}

/// Spec §6 + §5.1 `advance_month` final tick. Indexers can mark the pool
/// archived from this event alone.
#[event]
pub struct PoolCompleted {
    pub pool: Pubkey,
    pub total_contributed: u64,
    pub total_distributed: u64,
    pub completed_at: i64,
}

/// Spec §6 + §5.1 `commit_bid`. Emitted on each successful sealed-bid
/// commit. Indexers can rebuild the (pool, month, user) → commit_hash
/// map from these events alone. `stake_amount` is the 1% anti-spam
/// stake locked into the per-pool `bid_stake_vault` (Q-3).
#[event]
pub struct BidCommitted {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub month: u8,
    pub commit_hash: [u8; 32],
    pub stake_amount: u64,
    pub timestamp: i64,
}

/// Spec §6 + §5.1 `reveal_bid`. Emitted once the user opens their
/// commitment with the matching (bid_amount, nonce). At this point the
/// stake is refunded back to the user's USDC ATA in the same tx.
#[event]
pub struct BidRevealed {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub month: u8,
    pub bid_amount: u64,
    pub timestamp: i64,
}

/// Spec §6 + §5.1 `select_winner`. Emitted once per month when the
/// (sync, V1-mocked) winner-selection ix completes. `method` discriminates
/// the bid path (`SelectionMethod::Bid`, `winning_bid > 0`) from the
/// lottery path (`SelectionMethod::Lottery`, `winning_bid == 0`).
///
/// SPEC_QUESTION-21: in V1 the lottery branch uses a deterministic
/// pseudo-random seed (`sha256(pool || month || slot)`) instead of a
/// real Switchboard On-Demand VRF callback. The event shape is stable so
/// the indexer doesn't see a schema change when production swaps in real
/// VRF. See `select_winner.rs` for the integration point.
#[event]
pub struct WinnerSelected {
    pub pool: Pubkey,
    pub month: u8,
    pub winner: Pubkey,
    pub winning_bid: u64,
    pub gross_payout: u64,
    pub net_payout: u64,
    pub method: SelectionMethod,
    pub timestamp: i64,
}

/// Spec §6 — emitted whenever step 7 forfeits a committed-but-unrevealed
/// bid's stake to the tier reserve (Q-3). Per-bid; if multiple bids
/// expire the same month, multiple events are emitted in the same tx.
#[event]
pub struct BidStakeForfeited {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub month: u8,
    pub stake_amount: u64,
    pub timestamp: i64,
}

/// Spec §6 + §5.1 `claim_winning` (step 8). Emitted once per month when
/// the selected winner posts collateral and receives their net payout.
/// Indexers can rebuild the per-month claim status from this event alone.
/// `total_collateral_required` reflects the reputation multiplier
/// (Q-7 snapshot) and the bid premium (`winning_bid * 2`).
#[event]
pub struct WinningClaimed {
    pub pool: Pubkey,
    pub month: u8,
    pub winner: Pubkey,
    pub winning_bid: u64,
    pub net_payout: u64,
    pub total_collateral_required: u64,
    pub collateral_release_per_month: u64,
    pub timestamp: i64,
}

/// Spec §6 + §5.1 `claim_winning` bid distribution (step 8). One summary
/// event per claim per Q-17 (instead of per-recipient): indexers can
/// reconstruct the 75/20/5 split entirely from this single record.
/// `participant_share` is virtual — it lives in `pool.bid_credit_balance`
/// and discounts subsequent `contribute` calls; the on-chain tokens stay
/// in `pool_usdc_vault`.
#[event]
pub struct BidDistributed {
    pub pool: Pubkey,
    pub month: u8,
    pub total_bid: u64,
    pub participant_share: u64,
    pub reserve_share: u64,
    pub protocol_share: u64,
    pub bid_credit_balance_after: u64,
    pub timestamp: i64,
}

/// Spec §6 + §5.1 `distribute_yield` (step 9). Emitted at the START of
/// every `distribute_yield` call — once per harvest, regardless of whether
/// `yield_amount` is zero or positive. Tier 0 pools always emit
/// `yield_amount = 0` (spec §5.3); Tier 1 pools (step 12) emit the
/// realized USDC delta from the underlying DeFi adapter.
///
/// `tier` is encoded as `u8` (0 = Vault, 1 = DeFi) so indexers don't need
/// to maintain a parallel enum; matches the wire encoding in
/// `Tier::as_u8()`.
#[event]
pub struct YieldHarvested {
    pub pool: Pubkey,
    pub tier: u8,
    pub yield_amount: u64,
    pub timestamp: i64,
}

// ───── Step 10 — default cascade (spec §6 + §5.1) ──────────────────────

/// Emitted by `mark_late_payment` when a participant misses the strict
/// in-window contribution boundary and lands in the day-1..=5 grace
/// period. Single emit per (participant, month) — repeat marks revert.
/// `accrued_penalty` is the participant's *cumulative* penalty across
/// all months; new total after this mark.
#[event]
pub struct LatePayment {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub month: u8,
    pub penalty_added: u64,
    pub accrued_penalty: u64,
    pub timestamp: i64,
}

/// Emitted by `suspend_participant` once day 6 of the unpaid window
/// elapses. From this point onward `commit_bid` rejects the user;
/// `contribute` may still cure (Q-6), and `liquidate_default` runs at
/// day 30+.
#[event]
pub struct ParticipantSuspended {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub month: u8,
    pub timestamp: i64,
}

/// Emitted by `liquidate_default` for both Case A (post-win defaulter
/// with collateral to slash) and Case B (non-winner default — zero
/// token movement). `was_winner` lets indexers branch without rederiving
/// `participant.has_won`. INV-1 / arch §12 solvency proof: indexers can
/// verify the (collateral_drawn + reserve_drawn − total_owed) balance
/// from these fields alone.
#[event]
pub struct DefaultLiquidated {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub month: u8,
    pub was_winner: bool,
    pub total_owed: u64,
    pub liquidated_from_collateral: u64,
    pub drawn_from_reserve: u64,
    pub shortfall: u64,
    pub timestamp: i64,
}

/// Emitted only when `liquidate_default`'s reserve draw could not fully
/// cover the shortfall. Off-chain alerting hook (arch §5.4): the protocol
/// is technically still solvent because the missing amount is recorded
/// here, but the deficit must be made up by future reserve top-ups.
#[event]
pub struct LiquidationShortfall {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub month: u8,
    pub shortfall: u64,
    pub timestamp: i64,
}

/// Spec §6 + §5.1 `distribute_yield` (step 9). One summary event per
/// distribute_yield call per SPEC_QUESTION-17 (instead of per-participant):
/// the participant share lives in `pool.bid_credit_balance` and is
/// consumed via `contribute`'s pro-rata draw (Q-1) — same accounting
/// pattern as `BidDistributed`. Splits per spec §4: 70/20/10 (participants
/// / reserve / protocol). The participant share's tokens stay in
/// `pool_usdc_vault` and back the `bid_credit_balance` ledger; the reserve
/// and protocol shares are real on-chain transfers.
#[event]
pub struct YieldDistributed {
    pub pool: Pubkey,
    pub total_yield: u64,
    pub participant_share: u64,
    pub reserve_share: u64,
    pub protocol_share: u64,
    pub bid_credit_balance_after: u64,
    pub total_yield_distributed_after: u64,
    pub timestamp: i64,
}
