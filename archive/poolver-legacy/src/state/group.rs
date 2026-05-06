use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct ConsorcioGroup {
    /// Creator of the group
    pub creator: Pubkey,
    /// USDC mint address for this group
    pub mint: Pubkey,
    /// Monthly contribution per member (USDC, 6 decimals)
    pub monthly_contribution: u64,
    /// Maximum number of members (= number of rounds)
    pub total_members: u8,
    /// Current number of joined members
    pub current_members: u8,
    /// Current round (0-indexed, incremented after each distribution)
    pub current_round: u8,
    /// Group lifecycle status
    pub status: GroupStatus,
    /// Required collateral in basis points of total obligation
    pub collateral_bps: u16,
    /// Percentage of each payment allocated to insurance pool (bps)
    pub insurance_bps: u16,
    /// Protocol fee in basis points taken from each distribution
    pub protocol_fee_bps: u16,
    /// Timestamp when the group was created
    pub created_at: i64,
    /// Timestamp when formation phase expires (auto-refund after this)
    pub formation_deadline: i64,
    /// Timestamp when the current round's payment window opened
    pub round_started_at: i64,
    /// Number of members who have received the pool so far
    pub members_received: u8,
    /// Number of active (non-defaulted, non-withdrawn) members
    pub active_members: u8,
    /// Bump seed for the group PDA
    pub bump: u8,
    /// Bump seed for the vault PDA
    pub vault_bump: u8,
    /// Bump seed for the insurance vault PDA
    pub insurance_bump: u8,
    /// Bump seed for the protocol treasury PDA
    pub treasury_bump: u8,
    /// Short description of the group's purpose
    #[max_len(64)]
    pub description: String,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum GroupStatus {
    /// Accepting new members
    Forming,
    /// All slots filled, rounds in progress
    Active,
    /// All rounds completed, collateral returned
    Completed,
    /// Cancelled (formation timeout or dissolution)
    Cancelled,
}
