use anchor_lang::prelude::*;

/// Core program errors. Names align with spec §7's catalogue. Only the
/// subset required by step-4 instructions (`initialize_protocol`,
/// `mock_issue_kyc`, `initialize_user_reputation`, `create_pool`,
/// `join_pool`) is enumerated here; subsequent steps will append.
#[error_code]
pub enum CoreError {
    // ───── Auth / config ────────────────────────────────────────────────
    #[msg("Caller is not authorized for this instruction")]
    Unauthorized,
    #[msg("Protocol is paused")]
    ProtocolPaused,

    // ───── Math / amounts ──────────────────────────────────────────────
    #[msg("Arithmetic overflow")]
    MathOverflow,
    #[msg("Amount must be non-zero")]
    InvalidAmount,
    #[msg("Contribution amount outside the [100, 10_000] USDC range")]
    InvalidContributionAmount,

    // ───── Pool lifecycle ───────────────────────────────────────────────
    #[msg("Pool has 12 participants and cannot accept more")]
    PoolFull,
    #[msg("Pool has already started; new joins are no longer accepted")]
    PoolAlreadyStarted,
    #[msg("Pool is complete; no further mutations allowed")]
    PoolComplete,
    #[msg("This tier is not yet supported in V1; only Vault (Tier 0) is enabled")]
    TierNotYetSupported,

    // ───── Participant ──────────────────────────────────────────────────
    #[msg("User is already a participant in this pool")]
    AlreadyParticipant,

    // ───── KYC ──────────────────────────────────────────────────────────
    #[msg("User has no KYC attestation; Light KYC required to join a pool")]
    KycMissing,
    #[msg("KYC attestation has expired")]
    KycExpired,
    #[msg("KYC attestation level is below the required threshold")]
    KycInsufficientLevel,
    #[msg("KYC attestation flags a sanctions hit; user is blocked")]
    KycSanctionsHit,

    // ───── Reputation ───────────────────────────────────────────────────
    #[msg("UserReputation account is missing; call initialize_user_reputation first")]
    UserReputationMissing,

    // ───── Contribution / month-flow (step 5) ───────────────────────────
    #[msg("Pool has not started; current_month must be in 1..=12")]
    PoolNotStarted,
    #[msg("Caller is not a participant of this pool")]
    NotAParticipant,
    #[msg("Participant has already contributed for the current month")]
    ContributionAlreadyMade,
    #[msg("Outside the current-month contribution window (grace period not yet implemented)")]
    OutsideMonthWindow,
    #[msg("Participant is defaulted; contributions blocked")]
    Defaulted,
    #[msg("Participant is suspended; contributions blocked")]
    Suspended,
    #[msg("Current month duration has not elapsed; advance_month rejected")]
    MonthDurationNotElapsed,

    // ───── Bid commit-reveal (step 6) ───────────────────────────────────
    #[msg("Bid window is closed; commits not accepted (and reveal expired)")]
    BidWindowClosed,
    #[msg("Bid (commit) window is still open; reveal not yet allowed")]
    BidWindowOpen,
    #[msg("Bid amount exceeds the per-month bid cap (20% of monthly pot)")]
    BidExceedsCap,
    #[msg("Reveal hash does not match the stored commit_hash")]
    BidRevealMismatch,
    #[msg("Bid has already been revealed; second reveal rejected")]
    AlreadyRevealed,
    #[msg("Caller has already won a previous month and cannot bid again")]
    AlreadyWon,

    // ───── Winner selection (step 7) ────────────────────────────────────
    #[msg("Winner has already been selected for the current month")]
    WinnerAlreadySelected,
    #[msg("No eligible participants for the lottery (all have won, defaulted, or lack Full KYC)")]
    NoEligibleParticipants,
    #[msg("`select_winner` `remaining_accounts` is malformed: expected (bid|participant) chunks")]
    SelectWinnerAccountsMalformed,
    #[msg("Cannot advance to the next month before drawing the current month's winner")]
    WinnerNotSelected,

    // ───── Claim winning (step 8) ───────────────────────────────────────
    #[msg("Caller is not the selected winner for the current month")]
    NotWinner,
    #[msg("Winner has already claimed for this month")]
    AlreadyClaimed,
    #[msg("Winner does not have enough USDC to post the required collateral")]
    CollateralInsufficient,

    // ───── Default cascade (step 10) ────────────────────────────────────
    #[msg("Grace period has not elapsed yet (mark_late) or suspension threshold not reached")]
    GracePeriodNotElapsed,
    #[msg("Grace period has elapsed; mark_late no longer accepted — call suspend_participant")]
    GracePeriodElapsed,
    #[msg("30-day default threshold has not been reached; liquidation rejected")]
    DefaultThresholdNotReached,
    #[msg("Participant has already been liquidated; double-liquidate rejected")]
    AlreadyLiquidated,
    #[msg("Participant has already been marked late this month")]
    AlreadyMarkedLate,
    #[msg("Participant must be suspended before liquidation (defense-in-depth)")]
    NotSuspended,
    #[msg("Participant is not late (already paid this month or no overdue contribution)")]
    NotLate,
    #[msg("Reputation gate: user has prior defaults; new pool joins blocked (Q-11)")]
    ReputationDefaulted,
}
