use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::constants::{
    CORE_INVOKER_SEED, DEFI_ADAPTER_KTOKEN_SEED, DEFI_ADAPTER_SEED, DEFI_ADAPTER_USDC_SEED,
};
use crate::error::YieldDefiError;
use crate::events::AdapterWithdrew;
use crate::state::DefiAdapterState;
use crate::POOLVER_CORE_ID;

// Mirror of arch §13.2's `AdapterWithdraw`. Tier 1 appends Kamino-side
// accounts AFTER `token_program` — here, the kToken vault. Same
// leading-prefix shape as Tier 0 (INV-21).
#[derive(Accounts)]
pub struct AdapterWithdraw<'info> {
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
        mut,
        seeds = [DEFI_ADAPTER_USDC_SEED, adapter_state.pool.as_ref()],
        bump,
        constraint = adapter_usdc_vault.key() == adapter_state.usdc_vault
            @ YieldDefiError::Unauthorized,
    )]
    pub adapter_usdc_vault: Account<'info, TokenAccount>,

    /// Where to send the withdrawn USDC. Core's `claim_winning` /
    /// `liquidate_default` flows pick this; same trade as Tier 0.
    #[account(mut)]
    pub destination_usdc: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,

    #[account(
        mut,
        seeds = [DEFI_ADAPTER_KTOKEN_SEED, adapter_state.pool.as_ref()],
        bump,
        constraint = adapter_ktoken_vault.key() == adapter_state.ktoken_vault
            @ YieldDefiError::Unauthorized,
    )]
    pub adapter_ktoken_vault: Account<'info, TokenAccount>,
}

pub fn handle_withdraw(ctx: Context<AdapterWithdraw>, amount: u64) -> Result<()> {
    require!(amount > 0, YieldDefiError::InvalidAmount);
    require!(
        !ctx.accounts.adapter_state.tripped,
        YieldDefiError::CircuitBreakerTripped
    );

    let pool_key = ctx.accounts.adapter_state.pool;

    // Snapshot ledgers + token balances; we'll cross-check both. The
    // ledger fields can drift from the vault balances under unwind /
    // injected-yield scenarios, so for solvency the truth is the
    // token-account balance (spec §9.1). For UX we honor the ledger's
    // `liquid_reserved` cap when partitioning.
    let liquid_balance = ctx.accounts.adapter_usdc_vault.amount;
    let ktoken_balance = ctx.accounts.adapter_ktoken_vault.amount;

    // Drain liquid first.
    let from_liquid = amount.min(liquid_balance);
    let from_kamino = amount
        .checked_sub(from_liquid)
        .ok_or(YieldDefiError::MathOverflow)?;

    // Solvency: the kToken side must cover the rest. SPEC_QUESTION-19:
    // in production the redeem CPI returns the realized USDC; here we
    // just bounds-check the mock kToken vault's USDC balance.
    require!(
        from_kamino <= ktoken_balance,
        YieldDefiError::InsufficientLiquidity
    );

    // Drain liquid → destination.
    if from_liquid > 0 {
        let usdc_vault_bump = ctx.bumps.adapter_usdc_vault;
        let signer_seeds: &[&[&[u8]]] = &[&[
            DEFI_ADAPTER_USDC_SEED,
            pool_key.as_ref(),
            &[usdc_vault_bump],
        ]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            Transfer {
                from: ctx.accounts.adapter_usdc_vault.to_account_info(),
                to: ctx.accounts.destination_usdc.to_account_info(),
                authority: ctx.accounts.adapter_usdc_vault.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(cpi_ctx, from_liquid)?;
    }

    // Drain kToken → destination. SPEC_QUESTION-19: real Kamino redeem
    // CPI replaces this transfer; the mock simulates it 1:1 by moving
    // USDC out of the kToken-vault PDA.
    if from_kamino > 0 {
        let ktoken_vault_bump = ctx.bumps.adapter_ktoken_vault;
        let signer_seeds: &[&[&[u8]]] = &[&[
            DEFI_ADAPTER_KTOKEN_SEED,
            pool_key.as_ref(),
            &[ktoken_vault_bump],
        ]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            Transfer {
                from: ctx.accounts.adapter_ktoken_vault.to_account_info(),
                to: ctx.accounts.destination_usdc.to_account_info(),
                authority: ctx.accounts.adapter_ktoken_vault.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(cpi_ctx, from_kamino)?;
    }

    // Bookkeeping. Saturating-sub on the ledgers — they're a softer
    // truth than the token balances and may drift after harvest /
    // injected yield (which deliberately doesn't bump the ledger).
    let state = &mut ctx.accounts.adapter_state;
    state.liquid_reserved = state.liquid_reserved.saturating_sub(from_liquid);
    state.total_deployed_to_kamino = state
        .total_deployed_to_kamino
        .saturating_sub(from_kamino);
    state.total_deposited = state.total_deposited.saturating_sub(amount);

    emit!(AdapterWithdrew {
        pool: pool_key,
        amount,
        from_liquid,
        from_kamino,
        total_deposited: state.total_deposited,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}
