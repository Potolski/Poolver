use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::constants::{CORE_INVOKER_SEED, VAULT_ADAPTER_SEED, VAULT_ADAPTER_USDC_SEED};
use crate::error::YieldVaultError;
use crate::events::AdapterDeposited;
use crate::state::VaultAdapterState;
use crate::POOLVER_CORE_ID;

// Account layout follows arch §13.2 verbatim — the leading account positions
// MUST be byte-identical to `poolver-yield-defi`'s `AdapterDeposit` so core
// can build a single CPI shape per spec §5.3 / INV-21.
#[derive(Accounts)]
pub struct AdapterDeposit<'info> {
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

    /// Source of funds. Core passes the pool's PoolUsdcVault here. The
    /// authority signing the SPL transfer is forwarded by the caller; we
    /// don't constrain `source_usdc.owner` because core handles that on
    /// its side (arch §5.1).
    #[account(mut)]
    pub source_usdc: Account<'info, TokenAccount>,

    /// Authority over `source_usdc`. Required to sign the SPL transfer —
    /// in practice this is the pool USDC vault PDA owned by core. Passed
    /// in raw because Anchor cannot type a foreign-program PDA here.
    /// CHECK: signature presence is enforced by the SPL token program at
    /// CPI time; no further validation needed in this adapter.
    pub source_authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_deposit(ctx: Context<AdapterDeposit>, amount: u64) -> Result<()> {
    require!(amount > 0, YieldVaultError::InvalidAmount);

    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.key(),
        Transfer {
            from: ctx.accounts.source_usdc.to_account_info(),
            to: ctx.accounts.adapter_usdc_vault.to_account_info(),
            authority: ctx.accounts.source_authority.to_account_info(),
        },
    );
    token::transfer(cpi_ctx, amount)?;

    let state = &mut ctx.accounts.adapter_state;
    state.total_deposited = state
        .total_deposited
        .checked_add(amount)
        .ok_or(YieldVaultError::MathOverflow)?;

    let pool = state.pool;
    let total = state.total_deposited;

    emit!(AdapterDeposited {
        pool,
        amount,
        total_deposited: total,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}
