use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::set_return_data;
use anchor_spl::token::{Token, TokenAccount};

use crate::constants::{
    CORE_INVOKER_SEED, DEFI_ADAPTER_KTOKEN_SEED, DEFI_ADAPTER_SEED, DEFI_ADAPTER_USDC_SEED,
};
use crate::error::YieldDefiError;
use crate::events::AdapterHarvested;
use crate::state::DefiAdapterState;
use crate::POOLVER_CORE_ID;

// Same instruction-discriminator parity story as Tier 0: the byte
// shape `harvest()` MUST match `poolver-yield-vault::harvest()` so
// core's CPI dispatch is uniform (arch §13.1, INV-21). Tier 1's twist
// is that this one actually returns a meaningful number — the realized
// yield since the previous harvest.
#[derive(Accounts)]
pub struct AdapterHarvest<'info> {
    #[account(
        seeds = [CORE_INVOKER_SEED],
        seeds::program = POOLVER_CORE_ID,
        bump,
    )]
    pub core_invoker: Signer<'info>,

    #[account(
        mut,
        seeds = [DEFI_ADAPTER_SEED, adapter_state.pool.as_ref()],
        bump = adapter_state.bump,
    )]
    pub adapter_state: Account<'info, DefiAdapterState>,

    #[account(
        seeds = [DEFI_ADAPTER_USDC_SEED, adapter_state.pool.as_ref()],
        bump,
        constraint = adapter_usdc_vault.key() == adapter_state.usdc_vault
            @ YieldDefiError::Unauthorized,
    )]
    pub adapter_usdc_vault: Account<'info, TokenAccount>,

    #[account(
        seeds = [DEFI_ADAPTER_KTOKEN_SEED, adapter_state.pool.as_ref()],
        bump,
        constraint = adapter_ktoken_vault.key() == adapter_state.ktoken_vault
            @ YieldDefiError::Unauthorized,
    )]
    pub adapter_ktoken_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_harvest(ctx: Context<AdapterHarvest>) -> Result<u64> {
    require!(
        !ctx.accounts.adapter_state.tripped,
        YieldDefiError::CircuitBreakerTripped
    );

    // Realized yield = sum of both vault balances minus the snapshot
    // taken at the last harvest. SPEC_QUESTION-19: in production
    // `ktoken_vault.amount` is replaced by `kTokens × exchange_rate`
    // queried from Kamino; here we treat the kToken vault as already
    // denominated in USDC (the mock injects directly).
    let current_balance = ctx
        .accounts
        .adapter_usdc_vault
        .amount
        .checked_add(ctx.accounts.adapter_ktoken_vault.amount)
        .ok_or(YieldDefiError::MathOverflow)?;

    let last = ctx.accounts.adapter_state.last_recorded_balance;
    let yield_amount = current_balance.saturating_sub(last);

    // Update the snapshot. We update unconditionally (even when delta
    // is 0) so a future deposit/withdraw doesn't make the next
    // `harvest` see a "phantom" yield.
    let pool_key = ctx.accounts.adapter_state.pool;
    {
        let state = &mut ctx.accounts.adapter_state;
        state.last_recorded_balance = current_balance;
    }

    // Important: harvest does NOT move tokens. The yield is already
    // sitting in the kToken vault (as mock-injected USDC); core's
    // `distribute_yield` flow then calls `withdraw(yield_amount)` to
    // pull it out. Arch §5 + §13: harvest returns the amount via
    // Anchor's `set_return_data`/`get_return_data` plumbing.
    set_return_data(&yield_amount.to_le_bytes());

    emit!(AdapterHarvested {
        pool: pool_key,
        yield_amount,
        last_recorded_balance: current_balance,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(yield_amount)
}
