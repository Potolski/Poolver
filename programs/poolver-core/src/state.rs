use anchor_lang::prelude::*;

use crate::constants::{POOL_SIZE, TOTAL_MONTHS};

// ─────────────────────────────────────────────────────────────────────────
// Enums
// ─────────────────────────────────────────────────────────────────────────

/// Pool tier discriminant. Mirrors `poolver_reserve::Tier`'s wire bytes:
/// `Vault = 0`, `DeFi = 1`. The two enums must stay aligned because the
/// reserve seed `[RESERVE_FUND_SEED, &[tier_byte]]` is derived from
/// whichever tier is on the `Pool`. Tests assert the alignment.
///
/// Borsh discriminant assignment is source-order; do NOT reorder these
/// variants — INV-4 (tier isolation) depends on byte stability.
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tier {
    Vault,
    DeFi,
}

impl Tier {
    #[inline]
    pub fn as_u8(self) -> u8 {
        match self {
            Tier::Vault => 0,
            Tier::DeFi => 1,
        }
    }

    #[inline]
    pub fn seed_bytes(self) -> [u8; 1] {
        [self.as_u8()]
    }

    /// Cross-program safety: convert to the reserve crate's `Tier` so we
    /// can pass it through CPI args without re-serializing manually. The
    /// two enums are wire-byte-identical; this is a literal mapping.
    #[inline]
    pub fn to_reserve_tier(self) -> poolver_reserve::state::Tier {
        match self {
            Tier::Vault => poolver_reserve::state::Tier::Vault,
            Tier::DeFi => poolver_reserve::state::Tier::DeFi,
        }
    }
}

/// KYC level. Wire bytes: `None = 0`, `Light = 1`, `Full = 2`. Matches
/// arch §3.6's u8 encoding. `None` is never written into a
/// `KycAttestation` account (the account simply doesn't exist), but the
/// variant is present so `UserReputation.kyc_status` can carry it.
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, PartialEq, Eq, Debug)]
pub enum KycLevel {
    None,
    Light,
    Full,
}

impl KycLevel {
    #[inline]
    pub fn as_u8(self) -> u8 {
        match self {
            KycLevel::None => 0,
            KycLevel::Light => 1,
            KycLevel::Full => 2,
        }
    }
}

/// Selection method for `MonthWinner`. Filled in by future
/// `select_winner` / `consume_vrf_winner` instructions. Step-4 only
/// constructs the default value (Lottery=0) for empty winner slots.
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, PartialEq, Eq, Debug)]
pub enum SelectionMethod {
    Lottery,
    Bid,
}

impl Default for SelectionMethod {
    fn default() -> Self {
        SelectionMethod::Lottery
    }
}

// ─────────────────────────────────────────────────────────────────────────
// MonthWinner
// ─────────────────────────────────────────────────────────────────────────

/// Per-month winner record. Layout fixed by arch §3.2 (99 bytes per
/// entry). Stored inside `Pool.winners` as `[MonthWinner; 12]` with the
/// `month == 0` sentinel meaning "slot not yet filled".
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonthWinner {
    pub month: u8,
    pub winner: Pubkey,
    pub winning_bid: u64,
    pub gross_payout: u64,
    pub net_payout: u64,
    pub selected_at: i64,
    pub selection_method: SelectionMethod,
    pub claimed: bool,
    /// Reserved padding for forward compat. Sized to keep arch §3.2's
    /// 99-byte total. SPEC_QUESTION-15 mitigation: kept smaller than
    /// arch §3.2's nominal 32-byte target so the surrounding `Pool`
    /// struct fits into Anchor's 4 KB-per-frame `try_accounts` budget.
    pub _reserved: [u8; 8],
}

impl Default for MonthWinner {
    fn default() -> Self {
        Self {
            month: 0,
            winner: Pubkey::default(),
            winning_bid: 0,
            gross_payout: 0,
            net_payout: 0,
            selected_at: 0,
            selection_method: SelectionMethod::Lottery,
            claimed: false,
            _reserved: [0u8; 8],
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────
// ProtocolConfig (singleton — arch §3.1)
// ─────────────────────────────────────────────────────────────────────────

/// Protocol-wide configuration. Singleton; PDA derived from
/// `[PROTOCOL_CONFIG_SEED]`. Total ≈ 171 bytes including discriminator.
#[account]
#[derive(InitSpace)]
pub struct ProtocolConfig {
    pub admin: Pubkey,
    /// Authority that may issue real KYC attestations. In V1 = `admin`
    /// (placeholder; SPEC_QUESTION-26). Production rotates to a
    /// dedicated oracle key (HSM-backed Idwall integration).
    pub kyc_oracle: Pubkey,
    /// USDC token account that receives protocol fees. Owned by the
    /// `protocol_fee_vault` PDA (seeds `[PROTOCOL_FEE_VAULT_SEED]`).
    pub protocol_fee_vault: Pubkey,
    /// Canonical USDC mint pinned at protocol init.
    pub usdc_mint: Pubkey,
    pub protocol_fee_bps: u16,
    pub vault_reserve_fee_bps: u16,
    pub defi_reserve_fee_bps: u16,
    pub paused: bool,
    pub bump: u8,
    pub version: u8,
    pub _reserved: [u8; 64],
}

// ─────────────────────────────────────────────────────────────────────────
// Pool (size-critical — arch §3.2)
// ─────────────────────────────────────────────────────────────────────────

/// One 12-participant, 12-month pool. ~1965 bytes including
/// discriminator (verify via `Pool::INIT_SPACE`). SPEC_QUESTION-15:
/// always wrap in `Box<Account<'info, Pool>>` in handlers to keep the
/// 4 KB BPF stack frame breathable.
///
/// SPEC_QUESTION-8: fixed-size arrays for `participants` / `winners`.
#[account]
#[derive(InitSpace)]
pub struct Pool {
    pub pool_id: u64,
    pub creator: Pubkey,
    pub tier: Tier,
    pub contribution_amount: u64,
    pub participant_count: u8,
    pub total_months: u8,
    pub current_month: u8,
    pub start_timestamp: i64,
    pub month_duration_seconds: i64,
    pub bid_window_seconds: i64,
    pub current_month_started_at: i64,
    pub bid_window_ends_at: i64,
    pub reveal_window_ends_at: i64,
    pub total_contributed: u64,
    pub total_distributed: u64,
    pub total_collateral_locked: u64,
    pub bid_credit_balance: u64,
    pub is_complete: bool,
    pub vrf_in_flight: bool,
    pub vrf_account: Pubkey,
    pub pool_usdc_vault: Pubkey,
    pub collateral_vault: Pubkey,
    pub adapter_state: Pubkey,
    pub bump: u8,
    pub version: u8,
    /// Set by `advance_month` when the pool transitions past month 12.
    /// 0 ⇒ still active. SPEC_QUESTION-15 reserved-shrink: the 8 bytes
    /// for this field came out of the nominal `_reserved` budget so
    /// total bytes / Anchor INIT_SPACE remain stable from step 4.
    pub completed_at: i64,
    /// Number of `Participant`s that have already paid for the
    /// `current_month`. Incremented on every successful `contribute`,
    /// reset to 0 on every `advance_month` tick. Used by step 8's
    /// bid-credit pro-rata formula (SPEC_QUESTION-1): each contributing
    /// participant draws `bid_credit_balance / (POOL_SIZE - paid_count)`
    /// from the credit ledger, so the pool depletes evenly as the month
    /// progresses. 1 byte carved out of `_reserved` (8 → 7) — INIT_SPACE
    /// stays stable.
    pub paid_count_for_current_month: u8,
    /// Filled left-to-right as users `join_pool`. `Some(user)` means the
    /// slot is taken; `None` means free. SPEC_QUESTION-8.
    pub participants: [Option<Pubkey>; 12],
    /// Winner per month (1-indexed). `month == 0` ⇒ unfilled.
    pub winners: [MonthWinner; 12],
    /// Cumulative yield harvested + distributed for this pool across all
    /// `distribute_yield` calls. Monotonic non-decreasing (INV "Yield
    /// monotonic"). For Tier 0 pools this stays at 0 forever (Tier 0
    /// generates no yield by definition — spec §5.3); Tier 1 pools
    /// accumulate realized yield here in step 12. Indexers can rebuild
    /// per-pool APY from this field + `created_at`. SPEC_QUESTION-31
    /// reserved-shrink: 8 bytes added; the previous `_reserved: [u8; 7]`
    /// is exhausted and dropped to `[u8; 0]`. Net Pool size delta: +1
    /// byte vs step 8.
    pub total_yield_distributed: u64,
    /// Reserved padding for forward compat. Trimmed from arch §3.2's
    /// nominal 128 bytes for SPEC_QUESTION-15 compatibility (BPF
    /// 4 KB stack budget). Step 5 carved 8 bytes out for `completed_at`;
    /// step 8 carved 1 byte out for `paid_count_for_current_month`;
    /// step 9 carved 8 bytes out for `total_yield_distributed`. The
    /// remaining slot (0 bytes) is intentional — keeps the field
    /// declared for future migrations without further INIT_SPACE growth.
    pub _reserved: [u8; 0],
}

impl Pool {
    /// Number of currently-occupied participant slots.
    pub fn participant_filled(&self) -> u8 {
        let mut count: u8 = 0;
        for slot in self.participants.iter() {
            if slot.is_some() {
                count += 1;
            }
        }
        count
    }

    /// Returns true if `user` already occupies a slot.
    pub fn has_participant(&self, user: &Pubkey) -> bool {
        for slot in self.participants.iter() {
            if let Some(p) = slot {
                if p == user {
                    return true;
                }
            }
        }
        false
    }

    /// Index of the next free participant slot, if any.
    pub fn next_free_slot(&self) -> Option<usize> {
        for (i, slot) in self.participants.iter().enumerate() {
            if slot.is_none() {
                return Some(i);
            }
        }
        None
    }

    pub const POOL_SIZE: u8 = POOL_SIZE;
    pub const TOTAL_MONTHS: u8 = TOTAL_MONTHS;

    /// `true` while `now` is inside `[current_month_started_at,
    /// current_month_started_at + month_duration_seconds)`. Used by
    /// `contribute` (strict in-window — grace period is added in step 10
    /// per `// SPEC_QUESTION-6:`).
    pub fn within_current_month_window(&self, now: i64) -> bool {
        let started = self.current_month_started_at;
        let end = started.saturating_add(self.month_duration_seconds);
        now >= started && now < end
    }

    /// `true` if `user` has been selected as the winner of any past or
    /// current month, regardless of whether they've claimed yet. Used to
    /// gate commit_bid and select_winner candidate filtering — a winner
    /// is excluded from future selections the moment select_winner runs,
    /// not after claim_winning. (`Participant.has_won` only flips at
    /// claim time; relying on it would let an unclaimed winner win
    /// again, which the user explicitly does not want.)
    pub fn has_won_any_month(&self, user: &Pubkey) -> bool {
        for slot in self.winners.iter() {
            if slot.month != 0 && slot.selected_at != 0 && &slot.winner == user {
                return true;
            }
        }
        false
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Participant (arch §3.3)
// ─────────────────────────────────────────────────────────────────────────

#[account]
#[derive(InitSpace)]
pub struct Participant {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub joined_at: i64,
    /// Bitmap, bit N = month N+1 paid. Bit 0 is set on `join_pool`
    /// because the join contribution covers month 1 (spec §5.1).
    pub paid_months: u16,
    pub has_won: bool,
    pub win_month: u8,
    pub bid_amount_when_won: u64,
    pub collateral_locked: u64,
    pub collateral_initial: u64,
    pub is_defaulted: bool,
    pub is_suspended: bool,
    pub defaulted_at: i64,
    /// 200 bps (2%) penalty accrued via `mark_late_payment` (spec §4 +
    /// SPEC_QUESTION-6). Cleared when the participant cures by calling
    /// `contribute` (penalty is added on top of contribution and routed
    /// to `pool.bid_credit_balance` per Q-6) OR rolled into the
    /// liquidation amount on `liquidate_default`. Renamed from step 4's
    /// `late_penalty_accrued` for spec-§3 alignment without churning
    /// INIT_SPACE.
    pub late_penalty_accrued: u64,
    /// Total liquidated USDC (collateral + reserve drawdown) for this
    /// participant, populated on `liquidate_default`. Repurposed from
    /// step 4's unused `pending_credit` field — same 8-byte slot, no
    /// INIT_SPACE delta. SPEC_QUESTION-31 size playbook.
    pub liquidation_amount: u64,
    /// Snapshot of `UserReputation.pools_completed` at join (Q-7).
    pub completed_cycles_at_join: u8,
    pub bump: u8,
    /// Per-on-time-payment collateral release amount, cached at win-time
    /// (step 8 — `claim_winning`). Spec §4 collateral release schedule:
    /// `collateral_initial / months_remaining_at_win`. Step 5 reads this
    /// inside `contribute`'s post-win release branch; the field is `0`
    /// until step 8 actually populates it. SPEC_QUESTION-15 reserved-
    /// shrink: 8 bytes carved out of `_reserved` to keep total stable.
    pub collateral_release_per_month: u64,
    /// Step 10 default cascade — set by `mark_late_payment` (day 1 of
    /// grace). 1 byte carved from `_reserved` (24 → 23). Spec §4 + §5.1.
    pub is_late: bool,
    /// Wall-clock when `mark_late_payment` flagged this participant. Used
    /// by `mark_late_payment` to prevent double-mark within the same
    /// month. 8 bytes carved from `_reserved` (23 → 15).
    pub late_marked_at: i64,
    /// Wall-clock when `suspend_participant` flagged this participant.
    /// 8 bytes carved from `_reserved` (15 → 7).
    pub suspended_at: i64,
    /// Bitmap, bit N = month N+1 was *slashed* (filled by
    /// `slash_unpaid`) rather than paid normally via `contribute`.
    /// `paid_months` and `slashed_months` may both have the same bit
    /// set on a given month — semantically the slot was "satisfied"
    /// either way, but the UI distinguishes them so a viewer can see
    /// who actually defaulted on each month vs paid on time.
    /// 2 bytes carved from `_reserved` (7 → 5).
    pub slashed_months: u16,
    /// Reserved padding for forward compat. Step 10 default-cascade
    /// fields (`is_late: 1, late_marked_at: 8, suspended_at: 8` = 17 B)
    /// were carved from the original 24 B; `liquidation_amount` was
    /// repurposed in-place from `pending_credit`. Net Participant size
    /// delta vs step 9: 0 bytes. SPEC_QUESTION-31 size playbook.
    pub _reserved: [u8; 5],
}

impl Participant {
    /// `true` iff bit `(month - 1)` of `paid_months` is set.
    /// `month` is 1-indexed (months 1..=12). Out-of-range months return
    /// `false` so the caller can short-circuit on impossible inputs.
    pub fn has_paid_month(&self, month: u8) -> bool {
        if month == 0 || month as u16 > 16 {
            return false;
        }
        (self.paid_months & (1u16 << (month - 1))) != 0
    }

    /// INV-3: bits flip 0 → 1 only. We OR (`|=`) — never AND-NOT — so the
    /// monotonic property is structurally enforced.
    pub fn mark_month_paid(&mut self, month: u8) {
        debug_assert!(month >= 1 && month <= 12);
        self.paid_months |= 1u16 << (month - 1);
    }

    /// Set bit `(month - 1)` of `slashed_months`. Called by
    /// `slash_unpaid` in addition to `mark_month_paid` — the slot is
    /// "satisfied" in `paid_months`, but `slashed_months` records that
    /// the satisfaction came from a collateral slash rather than an
    /// on-time contribution.
    pub fn mark_month_slashed(&mut self, month: u8) {
        debug_assert!(month >= 1 && month <= 12);
        self.slashed_months |= 1u16 << (month - 1);
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Bid (arch §3.4) — sealed-bid commit-reveal record
// ─────────────────────────────────────────────────────────────────────────

/// One `Bid` PDA per (pool, month, user). PDA seeds:
/// `[BID_SEED, pool.key().as_ref(), &month.to_le_bytes(), user.key().as_ref()]`.
///
/// The `init` constraint on this PDA structurally enforces INV-16 (one
/// bid per user per month): a second `commit_bid` for the same triple
/// fails with `AccountAlreadyInitialized` before any handler logic
/// runs, so we don't carry an explicit "already committed" boolean.
///
/// `commit_hash` follows spec §3 / INV-14:
/// `sha256(bid_amount.to_le_bytes() (8) || nonce ([u8;16]) || user_pubkey (32))`.
/// The 56-byte input is fixed-length so reveal can deterministically
/// reconstruct it without a length prefix.
///
/// Layout matches arch §3.4 with one addition: `stake_refunded` is
/// carried locally (instead of `_reserved` padding) so step 7's
/// `select_winner` and the future no-reveal cleanup ix can both use it
/// as the idempotency flag for the stake side-effect.
#[account]
#[derive(InitSpace)]
pub struct Bid {
    pub pool: Pubkey,           // 32
    pub user: Pubkey,           // 32
    pub month: u8,              // 1
    pub commit_hash: [u8; 32],  // 32
    pub committed_at: i64,      // 8
    /// 1% of `pool.contribution_amount` at commit time (Q-3). Refunded
    /// to user on successful reveal, forfeit to tier reserve on
    /// no-reveal (forfeit path is step 7's concern).
    pub stake_amount: u64,      // 8
    pub revealed: bool,         // 1
    pub revealed_amount: u64,   // 8 — 0 until reveal
    pub revealed_at: i64,       // 8 — 0 until reveal
    /// Set in step 7's `select_winner`. False at commit / reveal time.
    pub is_winner: bool,        // 1
    /// True after the stake has been refunded (reveal happy path) OR
    /// forfeited to reserve (step 7 cleanup). Either side-effect is
    /// idempotent thanks to this flag.
    pub stake_refunded: bool,   // 1
    pub bump: u8,               // 1
    pub _reserved: [u8; 16],    // padding to keep arch §3.4's 156-byte slot
}

// ─────────────────────────────────────────────────────────────────────────
// UserReputation (global per user — arch §3.6)
// ─────────────────────────────────────────────────────────────────────────

#[account]
#[derive(InitSpace)]
pub struct UserReputation {
    pub user: Pubkey,
    pub pools_joined: u32,
    pub pools_completed: u32,
    pub pools_defaulted: u32,
    pub total_contributed_lifetime: u64,
    pub total_received_lifetime: u64,
    /// 0 = None, 1 = Light, 2 = Full. Mirrors `KycLevel::as_u8()`.
    pub kyc_status: u8,
    pub kyc_attestation: Pubkey,
    pub last_kyc_at: i64,
    pub bump: u8,
    /// Number of (pool, month) pairs where this user was slashed for
    /// missing the contribution. Soft signal — bumps the user into the
    /// "yellow" tier without flipping them to "red" (which is reserved
    /// for full defaults). Carved out of the original `_reserved: [u8; 32]`
    /// budget; existing on-chain accounts read 0 here, which matches the
    /// "never been slashed" case.
    pub months_missed_lifetime: u32,
    pub _reserved: [u8; 28],
}

// ─────────────────────────────────────────────────────────────────────────
// KycAttestation (arch §3.7)
// ─────────────────────────────────────────────────────────────────────────

#[account]
#[derive(InitSpace)]
pub struct KycAttestation {
    pub user: Pubkey,
    /// 1 = Light, 2 = Full. Never 0; the account simply does not exist
    /// when the user has no attestation.
    pub level: u8,
    pub issued_by: Pubkey,
    pub issued_at: i64,
    pub expires_at: i64,
    /// CPF hash (Brazilian tax ID). Zeroed in V1 mock; real KYC oracle
    /// will populate. // MOCK_KYC: zero placeholder.
    pub cpf_hash: [u8; 32],
    /// Sanctions screen result. Always `true` in V1 mock; real KYC
    /// oracle will populate. // MOCK_KYC: always true placeholder.
    pub sanctions_clean: bool,
    pub bump: u8,
    pub _reserved: [u8; 32],
}
