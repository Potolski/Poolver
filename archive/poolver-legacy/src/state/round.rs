use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Round {
    /// The group this round belongs to
    pub group: Pubkey,
    /// Round number (0-indexed)
    pub round_number: u8,
    /// Total USDC collected this round
    pub total_collected: u64,
    /// Number of members who have paid this round
    pub payments_received: u8,
    /// Winner selected by VRF lottery
    pub lottery_winner: Pubkey,
    /// Whether a winner has been selected
    pub winner_selected: bool,
    /// Whether the winner has claimed the distribution
    pub distribution_claimed: bool,
    /// Amount distributed to the winner
    pub distribution_amount: u64,
    /// VRF result used for selection (first 32 bytes of proof)
    pub vrf_result: [u8; 32],
    /// Round lifecycle status
    pub status: RoundStatus,
    /// Timestamp when this round started (collection window opened)
    pub started_at: i64,
    /// Slot when randomness was committed (for verification at reveal)
    pub commit_slot: u64,
    /// The randomness account used for this round's VRF
    pub randomness_account: Pubkey,
    /// Bump seed for this round PDA
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum RoundStatus {
    /// Payment window is open
    Collecting,
    /// Collection closed, awaiting VRF / selection
    Selecting,
    /// Winner selected, awaiting claim
    Distributing,
    /// Round fully resolved
    Completed,
}
