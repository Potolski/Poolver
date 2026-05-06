use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::constants::{
    CORE_INVOKER_SEED, DEFI_ADAPTER_KTOKEN_SEED, DEFI_ADAPTER_SEED, DEFI_ADAPTER_USDC_SEED,
    TRIP_REASON_NONE,
};
use crate::events::AdapterInitialized;
use crate::state::DefiAdapterState;
use crate::POOLVER_CORE_ID;

/// `initialize_adapter` — same shape as `poolver-yield-vault`'s entry
/// point so `poolver-core::create_pool` can dispatch on `pool.tier`
/// and reuse the same context wiring (arch §13 common interface).
///
/// V1 mock: this also `init`s a second token account (`ktoken_vault`)
/// that simulates Kamino's kToken position. SPEC_QUESTION-19: in
/// production this is replaced by a Kamino-supply CPI that mints
/// kTokens into a kToken-mint-typed account; the seed table here
/// already reserves the PDA for it.
#[derive(Accounts)]
#[instruction(pool: Pubkey)]
pub struct InitializeAdapter<'info> {
    /// PDA-as-signer proving the call comes from `poolver-core` (arch
    /// §5.2). The `seeds::program` clause anchors the derivation to
    /// core's program ID; no other caller can mint a matching signature.
    #[account(
        seeds = [CORE_INVOKER_SEED],
        seeds::program = POOLVER_CORE_ID,
        bump,
    )]
    pub core_invoker: Signer<'info>,

    /// Pays for both the state account and the two token-account rents.
    /// Core proxies this from the pool creator.
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + DefiAdapterState::INIT_SPACE,
        seeds = [DEFI_ADAPTER_SEED, pool.as_ref()],
        bump,
    )]
    pub adapter_state: Account<'info, DefiAdapterState>,

    /// USDC mint. Anchor's runtime mint check is sufficient — we don't
    /// pin the mint pubkey into the program (same trade as
    /// `poolver-yield-vault`).
    pub usdc_mint: Account<'info, Mint>,

    /// PDA-owned LIQUID USDC vault (the 25% buffer). Authority = the
    /// token-account itself; its own seeds sign for transfers.
    #[account(
        init,
        payer = payer,
        seeds = [DEFI_ADAPTER_USDC_SEED, pool.as_ref()],
        bump,
        token::mint = usdc_mint,
        token::authority = adapter_usdc_vault,
    )]
    pub adapter_usdc_vault: Account<'info, TokenAccount>,

    /// PDA-owned ktoken vault (mocked as a USDC token account in V1 —
    /// SPEC_QUESTION-19). Authority = the token-account itself.
    /// In production this account's `mint` would be the Kamino kToken
    /// mint; the seed binding stays.
    #[account(
        init,
        payer = payer,
        seeds = [DEFI_ADAPTER_KTOKEN_SEED, pool.as_ref()],
        bump,
        token::mint = usdc_mint,
        token::authority = adapter_ktoken_vault,
    )]
    pub adapter_ktoken_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_initialize_adapter(
    ctx: Context<InitializeAdapter>,
    pool: Pubkey,
) -> Result<()> {
    let state = &mut ctx.accounts.adapter_state;
    state.pool = pool;
    state.usdc_vault = ctx.accounts.adapter_usdc_vault.key();
    state.ktoken_vault = ctx.accounts.adapter_ktoken_vault.key();
    // SPEC_QUESTION-19: real Kamino reserve account ref slots in here.
    // V1 mock leaves it as the default sentinel.
    state.kamino_reserve = Pubkey::default();
    state.total_deposited = 0;
    state.total_deployed_to_kamino = 0;
    state.liquid_reserved = 0;
    state.last_recorded_balance = 0;
    state.tripped = false;
    state.tripped_at = 0;
    state.tripped_reason = TRIP_REASON_NONE;
    // Default-safe mock readings: 50% utilization, 0 deviation, not
    // paused. With the live thresholds (>9500 bps, >200 bps), none of
    // these trip. Tests + the demo bump them with the `mock_set_*`
    // helpers when they want to exercise breaker paths.
    state.mock_utilization_bps = 5_000;
    state.mock_oracle_deviation_bps = 0;
    state.mock_kamino_paused = false;
    state.bump = ctx.bumps.adapter_state;
    state._reserved = [0u8; 56];

    emit!(AdapterInitialized {
        pool,
        adapter_state: state.key(),
        usdc_vault: state.usdc_vault,
        ktoken_vault: state.ktoken_vault,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}
