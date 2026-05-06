use anchor_lang::prelude::*;

#[event]
pub struct GroupCreated {
    pub group: Pubkey,
    pub creator: Pubkey,
    pub monthly_contribution: u64,
    pub total_members: u8,
    pub collateral_bps: u16,
    pub timestamp: i64,
}

#[event]
pub struct MemberJoined {
    pub group: Pubkey,
    pub member: Pubkey,
    pub collateral_deposited: u64,
    pub current_members: u8,
    pub timestamp: i64,
}

#[event]
pub struct MemberLeft {
    pub group: Pubkey,
    pub member: Pubkey,
    pub refund_amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct GroupActivated {
    pub group: Pubkey,
    pub total_members: u8,
    pub timestamp: i64,
}

#[event]
pub struct PaymentMade {
    pub group: Pubkey,
    pub member: Pubkey,
    pub round: u8,
    pub amount: u64,
    pub is_late: bool,
    pub timestamp: i64,
}

#[event]
pub struct RoundStarted {
    pub group: Pubkey,
    pub round: u8,
    pub timestamp: i64,
}

#[event]
pub struct WinnerSelected {
    pub group: Pubkey,
    pub round: u8,
    pub winner: Pubkey,
    pub amount: u64,
    pub vrf_proof: [u8; 32],
    pub timestamp: i64,
}

#[event]
pub struct DistributionClaimed {
    pub group: Pubkey,
    pub round: u8,
    pub member: Pubkey,
    pub amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct DefaultMarked {
    pub group: Pubkey,
    pub member: Pubkey,
    pub round: u8,
    pub collateral_slashed: u64,
    pub total_missed: u8,
    pub timestamp: i64,
}

#[event]
pub struct InsuranceDistributed {
    pub group: Pubkey,
    pub member: Pubkey,
    pub amount: u64,
    pub remaining_members: u8,
    pub timestamp: i64,
}

#[event]
pub struct GroupCompleted {
    pub group: Pubkey,
    pub total_rounds: u8,
    pub insurance_surplus: u64,
    pub timestamp: i64,
}
