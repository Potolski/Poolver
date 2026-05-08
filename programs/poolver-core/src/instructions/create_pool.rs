use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::constants::{
    BID_STAKE_VAULT_SEED, COLLATERAL_VAULT_SEED, CORE_INVOKER_SEED, DEFAULT_BID_WINDOW_SECS,
    DEFAULT_MONTH_DURATION_SECS, KYC_SEED, MAX_CONTRIBUTION,
    MIN_CONTRIBUTION, POOL_SEED, POOL_SIZE, POOL_USDC_VAULT_SEED, PROTOCOL_CONFIG_SEED,
    REPUTATION_SEED, TOTAL_MONTHS,
};
use crate::error::CoreError;
use crate::events::PoolCreated;
use crate::kyc::require_light_kyc;
use crate::state::{
    KycAttestation, MonthWinner, Pool, ProtocolConfig, Tier, UserReputation,
};

/// `create_pool` — spec §5.1. Validates contribution bounds, creator
/// KYC + reputation existence, then initializes the pool, the pool USDC
/// vault, the collateral vault, and CPIs into the yield adapter to mint
/// its per-pool state.
///
/// ## SPEC_QUESTION-36 — Tier-1 dispatch wired in step 13
///
/// Pre-step-13 this handler hard-rejected `Tier::DeFi` and pinned the
/// `yield_vault_program` to `poolver_yield_vault::ID`. Step 13 unlocks
/// Tier 1: the `address` constraint is dropped from the context, and
/// `cpi_adapter_initialize` validates the supplied program ID against
/// `pool.tier`. Tier 1 callers append the `adapter_ktoken_vault` to
/// `remaining_accounts` (see `adapter_cpi::adapter` doc table for the
/// canonical ordering).
///
/// SPEC_QUESTION-15: `Pool` is wrapped in `Box` so its ~1965 bytes
/// don't squash the BPF stack frame.
#[derive(Accounts)]
#[instruction(pool_id: u64, tier: Tier, contribution_amount: u64)]
pub struct CreatePool<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        seeds = [PROTOCOL_CONFIG_SEED],
        bump = protocol_config.bump,
        constraint = !protocol_config.paused @ CoreError::ProtocolPaused,
    )]
    pub protocol_config: Box<Account<'info, ProtocolConfig>>,

    /// Creator's KYC attestation. Must be Light or better; verification
    /// runs through the same helper used by every KYC-gated instruction
    /// (`crate::kyc::require_light_kyc`).
    #[account(
        seeds = [KYC_SEED, creator.key().as_ref()],
        bump = creator_kyc.bump,
    )]
    pub creator_kyc: Box<Account<'info, KycAttestation>>,

    /// Creator's reputation. Existence is required so `pools_completed`
    /// can be snapshotted; we do NOT mutate it here (snapshot is taken
    /// in `join_pool` for whichever user joins).
    #[account(
        seeds = [REPUTATION_SEED, creator.key().as_ref()],
        bump = creator_reputation.bump,
    )]
    pub creator_reputation: Box<Account<'info, UserReputation>>,

    /// The pool being created. Box'd to keep the stack frame lean.
    #[account(
        init,
        payer = creator,
        space = 8 + Pool::INIT_SPACE,
        seeds = [POOL_SEED, creator.key().as_ref(), &pool_id.to_le_bytes()],
        bump,
    )]
    pub pool: Box<Account<'info, Pool>>,

    pub usdc_mint: Box<Account<'info, Mint>>,

    /// PDA-owned USDC vault for this pool's contributions. Authority is
    /// the token account itself (its seeds sign for transfers in / out).
    #[account(
        init,
        payer = creator,
        seeds = [POOL_USDC_VAULT_SEED, pool.key().as_ref()],
        bump,
        token::mint = usdc_mint,
        token::authority = pool_usdc_vault,
    )]
    pub pool_usdc_vault: Box<Account<'info, TokenAccount>>,

    /// PDA-owned collateral vault. Same self-authority pattern.
    #[account(
        init,
        payer = creator,
        seeds = [COLLATERAL_VAULT_SEED, pool.key().as_ref()],
        bump,
        token::mint = usdc_mint,
        token::authority = collateral_vault,
    )]
    pub collateral_vault: Box<Account<'info, TokenAccount>>,

    /// PDA-owned vault that escrows the 1% anti-spam bid stakes
    /// (SPEC_QUESTION-3). Step 6 — `commit_bid` deposits, `reveal_bid`
    /// refunds, step 7's `select_winner` (or its no-reveal cleanup ix)
    /// sweeps any unrevealed stakes to the tier reserve. Self-authority
    /// pattern matches `pool_usdc_vault` and `collateral_vault`.
    #[account(
        init,
        payer = creator,
        seeds = [BID_STAKE_VAULT_SEED, pool.key().as_ref()],
        bump,
        token::mint = usdc_mint,
        token::authority = bid_stake_vault,
    )]
    pub bid_stake_vault: Box<Account<'info, TokenAccount>>,

    /// `core_invoker` PDA — used as signer for the CPI into yield-vault.
    /// CHECK: not deserialized; we just need the AccountInfo to feed
    /// into `invoke_signed`. Seeds are verified inside the adapter.
    #[account(
        seeds = [CORE_INVOKER_SEED],
        bump,
    )]
    pub core_invoker: UncheckedAccount<'info>,

    // ───── Yield-vault adapter accounts (CPI target) ───────────────────
    //
    // The adapter's `initialize_adapter` `init`s these PDAs itself with
    // `seeds = [VAULT_ADAPTER_SEED, pool.as_ref()]` and
    // `seeds = [VAULT_ADAPTER_USDC_SEED, pool.as_ref()]`. We pass them
    // as raw `UncheckedAccount`s; the seed validation runs inside the
    // adapter (the canonical contract). Re-validating here doubles the
    // stack pressure during `try_accounts` (SPEC_QUESTION-15).

    /// CHECK: validated by the chosen adapter's `initialize_adapter`.
    #[account(mut)]
    pub adapter_state: UncheckedAccount<'info>,

    /// CHECK: validated by the chosen adapter's `initialize_adapter`.
    #[account(mut)]
    pub adapter_usdc_vault: UncheckedAccount<'info>,

    /// CHECK: SPEC_QUESTION-36 — program ID is validated in the handler
    /// via `cpi_adapter_initialize` against `pool.tier`. The hardcoded
    /// `address = poolver_yield_vault::ID` constraint that pre-step-13
    /// builds carried was dropped so Tier 1 callers can pass
    /// `poolver_yield_defi::ID`.
    pub yield_adapter_program: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_create_pool<'info>(
    ctx: Context<'info, CreatePool<'info>>,
    pool_id: u64,
    tier: Tier,
    contribution_amount: u64,
    month_duration_seconds: Option<i64>,
) -> Result<()> {
    // ───── Pre-checks (spec §5.1) ──────────────────────────────────────
    require!(
        contribution_amount >= MIN_CONTRIBUTION
            && contribution_amount <= MAX_CONTRIBUTION,
        CoreError::InvalidContributionAmount
    );

    // SPEC_QUESTION-36: step 13 — Tier 1 unlocked. The handler dispatches
    // the `initialize_adapter` CPI by `tier` via `cpi_adapter_initialize`;
    // the helper validates the supplied `yield_adapter_program` against
    // the canonical adapter ID for the tier. Tier 1 callers must pass
    // `adapter_ktoken_vault` as `remaining_accounts[0]`.

    let now = Clock::get()?.unix_timestamp;
    require_light_kyc(&ctx.accounts.creator_kyc, &ctx.accounts.creator.key(), now)?;

    let month_duration = month_duration_seconds.unwrap_or(DEFAULT_MONTH_DURATION_SECS);
    require!(month_duration > 0, CoreError::InvalidAmount);

    // Scale the per-pool bid window to half the month duration, capped
    // at the production default (48h). Production-default 30-day months
    // get the full 48h; demo pools with 10-minute months get a 5-minute
    // bid window so the auction can actually complete inside the month.
    // Floor: 60 seconds (so even a 60s month leaves a 30s window — though
    // that would be an unusable demo cadence in practice).
    let scaled_bid_window = (month_duration / 2)
        .max(60)
        .min(DEFAULT_BID_WINDOW_SECS);

    // ───── Pool initial state ──────────────────────────────────────────
    {
        let pool = &mut ctx.accounts.pool;
        pool.pool_id = pool_id;
        pool.creator = ctx.accounts.creator.key();
        pool.tier = tier;
        pool.contribution_amount = contribution_amount;
        pool.participant_count = POOL_SIZE;
        pool.total_months = TOTAL_MONTHS;
        pool.current_month = 0;
        pool.start_timestamp = 0;
        pool.month_duration_seconds = month_duration;
        pool.bid_window_seconds = scaled_bid_window;
        pool.current_month_started_at = 0;
        pool.bid_window_ends_at = 0;
        pool.reveal_window_ends_at = 0;
        pool.total_contributed = 0;
        pool.total_distributed = 0;
        pool.total_collateral_locked = 0;
        pool.bid_credit_balance = 0;
        pool.is_complete = false;
        pool.vrf_in_flight = false;
        pool.vrf_account = Pubkey::default();
        pool.pool_usdc_vault = ctx.accounts.pool_usdc_vault.key();
        pool.collateral_vault = ctx.accounts.collateral_vault.key();
        pool.adapter_state = ctx.accounts.adapter_state.key();
        pool.bump = ctx.bumps.pool;
        pool.version = 1;
        pool.completed_at = 0;
        // Step 8 (Q-1): reset to 0; incremented in `contribute`, reset in
        // `advance_month` so the bid-credit pro-rata divisor reflects who
        // has not yet paid for the current month.
        pool.paid_count_for_current_month = 0;
        pool.participants = [None; 12];
        pool.winners = [MonthWinner::default(); 12];
        // Step 9: yield ledger starts at 0; `distribute_yield` increments.
        // For Tier 0 this remains 0 for the pool's lifetime (spec §5.3 —
        // Tier 0 generates no yield); Tier 1 (step 12) accrues here.
        pool.total_yield_distributed = 0;
    }

    let pool_key = ctx.accounts.pool.key();

    // ───── CPI → adapter::initialize_adapter (SPEC_QUESTION-36) ────────
    let core_invoker_bump = ctx.bumps.core_invoker;

    crate::adapter_cpi::adapter::cpi_adapter_initialize(
        tier,
        ctx.accounts.yield_adapter_program.to_account_info(),
        ctx.accounts.core_invoker.to_account_info(),
        ctx.accounts.creator.to_account_info(),
        ctx.accounts.adapter_state.to_account_info(),
        ctx.accounts.usdc_mint.to_account_info(),
        ctx.accounts.adapter_usdc_vault.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
        ctx.accounts.rent.to_account_info(),
        ctx.remaining_accounts,
        pool_key,
        core_invoker_bump,
    )?;

    emit!(PoolCreated {
        pool: pool_key,
        pool_id,
        creator: ctx.accounts.creator.key(),
        tier,
        contribution_amount,
        month_duration_seconds: month_duration,
        timestamp: now,
    });

    Ok(())
}
