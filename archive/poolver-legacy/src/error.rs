use anchor_lang::prelude::*;

#[error_code]
pub enum ConsolError {
    // Group creation
    #[msg("Group size must be between 3 and 50 members")]
    InvalidGroupSize,
    #[msg("Monthly contribution is below the minimum")]
    ContributionTooLow,
    #[msg("Invalid collateral basis points")]
    InvalidCollateralBps,
    #[msg("Invalid insurance basis points")]
    InvalidInsuranceBps,
    #[msg("Mint must have 6 decimals (USDC standard)")]
    InvalidMintDecimals,
    #[msg("Description exceeds maximum length of 64 bytes")]
    DescriptionTooLong,

    // Group state
    #[msg("Group is not in the expected state for this operation")]
    InvalidGroupState,
    #[msg("Group is already full")]
    GroupFull,
    #[msg("Group formation period has expired")]
    FormationTimeout,

    // Membership
    #[msg("Already a member of this group")]
    AlreadyMember,
    #[msg("Not a member of this group")]
    NotMember,
    #[msg("Member has been defaulted")]
    MemberDefaulted,
    #[msg("Member has already received the pool")]
    AlreadyReceived,
    #[msg("Member has not received the pool yet")]
    NotYetReceived,

    // Payments
    #[msg("Payment window is not currently open")]
    PaymentWindowClosed,
    #[msg("Payment already made for this round")]
    AlreadyPaid,
    #[msg("Incorrect payment amount")]
    IncorrectAmount,
    #[msg("Insufficient collateral deposited")]
    InsufficientCollateral,

    // Round & selection
    #[msg("Round is not in the expected state")]
    InvalidRoundState,
    #[msg("No eligible members for selection")]
    NoEligibleMembers,
    #[msg("VRF result has not been received yet")]
    VrfNotResolved,
    #[msg("Member is not eligible for selection this round")]
    NotEligible,
    #[msg("Invalid Switchboard randomness account")]
    InvalidRandomnessAccount,

    // Defaults
    #[msg("Member has not exceeded the missed payment threshold")]
    NotInDefault,
    #[msg("Cannot withdraw after receiving the pool")]
    CannotWithdrawPostReceipt,
    #[msg("Grace period has not ended yet")]
    GracePeriodActive,

    // Math
    #[msg("Arithmetic overflow")]
    MathOverflow,
}
