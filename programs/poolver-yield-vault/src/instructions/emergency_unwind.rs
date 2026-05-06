use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::constants::{CORE_INVOKER_SEED, VAULT_ADAPTER_SEED, VAULT_ADAPTER_USDC_SEED};
use crate::error::YieldVaultError;
use crate::events::AdapterUnwound;
use crate::state::VaultAdapterState;
use crate::POOLVER_CORE_ID;

// Tier 0 has nothing to unwind from — it's just a token account — so
// `emergency_unwind` is functionally identical to draining the entire
// balance via `withdraw`. We keep the instruction so that core's
// circuit-breaker codepath can call the same name on either adapter
// (INV-20 / INV-21 / arch §13).
#[derive(Accounts)]
pub struct AdapterUnwind<'info> {
    #[account(
        seeds = [CORE_INVOKER_SEED],
        seeds::program = POOLVER_CORE_ID,
        bump,
    )]
    pub core_invoker: Signer<'info>,

    #[account(
        mut,
        seeds = [VAULT_ADAPTER_SEED, adapter_state.pool.as_ref()],
        bump = adapter_state.bump,
    )]
    pub adapter_state: Account<'info, VaultAdapterState>,

    #[account(
        mut,
        seeds = [VAULT_ADAPTER_USDC_SEED, adapter_state.pool.as_ref()],
        bump,
        constraint = adapter_usdc_vault.key() == adapter_state.usdc_vault
            @ YieldVaultError::Unauthorized,
    )]
    pub adapter_usdc_vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub destination_usdc: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_emergency_unwind(ctx: Context<AdapterUnwind>) -> Result<()> {
    let amount = ctx.accounts.adapter_usdc_vault.amount;

    if amount > 0 {
        let pool = ctx.accounts.adapter_state.pool;
        let usdc_vault_bump = ctx.bumps.adapter_usdc_vault;
        let signer_seeds: &[&[&[u8]]] = &[&[
            VAULT_ADAPTER_USDC_SEED,
            pool.as_ref(),
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
        token::transfer(cpi_ctx, amount)?;
    }

    // After unwind the ledger is necessarily 0 — vault is empty.
    let state = &mut ctx.accounts.adapter_state;
    state.total_deposited = 0;

    emit!(AdapterUnwound {
        pool: state.pool,
        amount_unwound: amount,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}
