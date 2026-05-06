use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::constants::{CORE_INVOKER_SEED, VAULT_ADAPTER_SEED, VAULT_ADAPTER_USDC_SEED};
use crate::error::YieldVaultError;
use crate::events::AdapterWithdrew;
use crate::state::VaultAdapterState;
use crate::POOLVER_CORE_ID;

// Mirror of arch §13.2's `AdapterWithdraw`. Tier 1 will append Kamino-side
// accounts AFTER `token_program`; the leading prefix must match this exact
// shape (INV-21).
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

    /// Where to send the withdrawn USDC. Core's `claim_winning` /
    /// `liquidate_default` flows pick this; we don't constrain the
    /// destination beyond "is a token account" — same-mint check is done
    /// by the SPL transfer.
    #[account(mut)]
    pub destination_usdc: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_withdraw(ctx: Context<AdapterWithdraw>, amount: u64) -> Result<()> {
    require!(amount > 0, YieldVaultError::InvalidAmount);
    require!(
        ctx.accounts.adapter_usdc_vault.amount >= amount,
        YieldVaultError::InsufficientLiquidity
    );

    let pool = ctx.accounts.adapter_state.pool;
    let usdc_vault_bump = ctx.bumps.adapter_usdc_vault;
    // Stored bumps for signer seeds: arch §4 ("Re-derivation uses
    // create_program_address with the stored bump") — we already verified
    // the bump above via the `seeds = ...` constraint, so reusing
    // `ctx.bumps` is canonical.
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

    // `total_deposited` is a ledger, not a balance — saturate so we never
    // underflow if a tier emits more outflow than inflow during recovery
    // accounting. The vault token balance remains the truth (spec §9.1).
    let state = &mut ctx.accounts.adapter_state;
    state.total_deposited = state.total_deposited.saturating_sub(amount);

    let pool_evt = state.pool;
    let total = state.total_deposited;

    emit!(AdapterWithdrew {
        pool: pool_evt,
        amount,
        total_deposited: total,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}
