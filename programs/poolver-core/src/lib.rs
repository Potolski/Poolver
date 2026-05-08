pub mod adapter_cpi;
pub mod constants;
pub mod error;
pub mod events;
pub mod instructions;
pub mod kyc;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use error::*;
pub use events::*;
pub use instructions::*;
pub use state::*;

declare_id!("2SsxJqMCYKCYesfzfXASgAPPz153j8tYMXpMKKmt2QXk");

/// Poolver core program. Step-5 surface:
///   - `initialize_protocol`
///   - `mock_issue_kyc`            (gated under `mock-kyc` feature; spec §9.11)
///   - `initialize_user_reputation`
///   - `create_pool`
///   - `join_pool`
///   - `contribute`                (step 5)
///   - `advance_month`             (step 5)
///
/// Subsequent steps will append `commit_bid`, `reveal_bid` (step 6),
/// `select_winner` (step 7), `claim_winning` (step 8),
/// `liquidate_default` (step 10), `distribute_yield` (step 9),
/// `emergency_pause` / `emergency_unpause`, and `seed_reserve` (proxy).
#[program]
pub mod poolver_core {
    use super::*;

    pub fn initialize_protocol(ctx: Context<InitializeProtocol>) -> Result<()> {
        handle_initialize_protocol(ctx)
    }

    // SPEC_QUESTION-26: admin tear-down of the singleton config so a fresh
    // `initialize_protocol` can rebind the USDC mint. V2 multi-sig will
    // gate this; V1 enforces single-admin via `has_one = admin`.
    pub fn admin_close_protocol(ctx: Context<AdminCloseProtocol>) -> Result<()> {
        handle_admin_close_protocol(ctx)
    }

    // Admin-only fast-forward of the pool's current phase. Devnet/dev
    // convenience — bypasses the on-chain time check by mutating the
    // window endpoints. Does NOT bypass any other invariant.
    pub fn admin_skip_phase(ctx: Context<AdminSkipPhase>) -> Result<()> {
        handle_admin_skip_phase(ctx)
    }

    /// Refund a non-defaulting participant's locked collateral after
    /// the pool has completed. Permissionless — anyone may call.
    pub fn refund_collateral(ctx: Context<RefundCollateral>) -> Result<()> {
        handle_refund_collateral(ctx)
    }

    // MOCK_KYC: V1 only — replaced in production by `issue_kyc_attestation`
    // signed by `protocol_config.kyc_oracle`. Building with
    // `--no-default-features` drops this entry from the program dispatch
    // table, the IDL, and the .so binary entirely (INV-26, arch §10).
    #[cfg(feature = "mock-kyc")]
    pub fn mock_issue_kyc(
        ctx: Context<MockIssueKyc>,
        user: Pubkey,
        level: KycLevel,
    ) -> Result<()> {
        handle_mock_issue_kyc(ctx, user, level)
    }

    pub fn initialize_user_reputation(
        ctx: Context<InitializeUserReputation>,
    ) -> Result<()> {
        handle_initialize_user_reputation(ctx)
    }

    pub fn create_pool<'info>(
        ctx: Context<'info, CreatePool<'info>>,
        pool_id: u64,
        tier: Tier,
        contribution_amount: u64,
        month_duration_seconds: Option<i64>,
    ) -> Result<()> {
        handle_create_pool(ctx, pool_id, tier, contribution_amount, month_duration_seconds)
    }

    pub fn join_pool<'info>(
        ctx: Context<'info, JoinPool<'info>>,
    ) -> Result<()> {
        handle_join_pool(ctx)
    }

    pub fn contribute<'info>(
        ctx: Context<'info, Contribute<'info>>,
    ) -> Result<()> {
        handle_contribute(ctx)
    }

    pub fn advance_month(ctx: Context<AdvanceMonth>) -> Result<()> {
        handle_advance_month(ctx)
    }

    pub fn commit_bid(
        ctx: Context<CommitBid>,
        commit_hash: [u8; 32],
    ) -> Result<()> {
        handle_commit_bid(ctx, commit_hash)
    }

    pub fn reveal_bid(
        ctx: Context<RevealBid>,
        bid_amount: u64,
        nonce: [u8; 16],
    ) -> Result<()> {
        handle_reveal_bid(ctx, bid_amount, nonce)
    }

    pub fn select_winner<'info>(
        ctx: Context<'info, SelectWinner<'info>>,
    ) -> Result<()> {
        handle_select_winner(ctx)
    }

    pub fn claim_winning<'info>(
        ctx: Context<'info, ClaimWinning<'info>>,
        claim_month: u8,
    ) -> Result<()> {
        handle_claim_winning(ctx, claim_month)
    }

    pub fn distribute_yield<'info>(
        ctx: Context<'info, DistributeYield<'info>>,
    ) -> Result<()> {
        handle_distribute_yield(ctx)
    }

    pub fn mark_late_payment(ctx: Context<MarkLatePayment>) -> Result<()> {
        handle_mark_late_payment(ctx)
    }

    pub fn suspend_participant(ctx: Context<SuspendParticipant>) -> Result<()> {
        handle_suspend_participant(ctx)
    }

    pub fn liquidate_default(ctx: Context<LiquidateDefault>) -> Result<()> {
        handle_liquidate_default(ctx)
    }
}
