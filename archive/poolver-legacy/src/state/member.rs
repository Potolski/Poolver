use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Member {
    /// The group this membership belongs to
    pub group: Pubkey,
    /// The member's wallet
    pub wallet: Pubkey,
    /// Total collateral deposited (USDC)
    pub collateral_deposited: u64,
    /// Number of on-time payments made
    pub payments_made: u8,
    /// Number of missed payments (resets on catch-up, accumulates toward default)
    pub payments_missed: u8,
    /// Whether this member has received the pool distribution
    pub has_received: bool,
    /// The round in which this member received (u8::MAX = hasn't received; check has_received first)
    pub received_round: u8,
    /// Total amount paid across all rounds (for withdrawal refund calc)
    pub total_paid: u64,
    /// Last round this member paid for (used to prevent double-pay per round)
    pub last_paid_round: u8,
    /// Last round this member was marked as defaulting (prevents double-default per round)
    pub last_default_round: u8,
    /// Whether this member has claimed their insurance share
    pub insurance_claimed: bool,
    /// Member lifecycle status
    pub status: MemberStatus,
    /// Timestamp when the member joined
    pub joined_at: i64,
    /// Bump seed for this member PDA
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum MemberStatus {
    /// Active and in good standing
    Active,
    /// Missed MAX_MISSED_PAYMENTS, collateral seized
    Defaulted,
    /// Voluntarily left before receiving
    Withdrawn,
}
