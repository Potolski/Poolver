use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Reputation {
    /// The wallet this reputation belongs to
    pub wallet: Pubkey,
    /// Number of consórcios completed without default
    pub completed: u16,
    /// Number of consórcios defaulted on
    pub defaulted: u16,
    /// Total number of on-time payments across all groups
    pub total_payments: u32,
    /// Timestamp of last completed consórcio
    pub last_completed_at: i64,
    /// Bump seed for this reputation PDA
    pub bump: u8,
}
