use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

use crate::constants::{CORE_INVOKER_SEED, VAULT_ADAPTER_SEED, VAULT_ADAPTER_USDC_SEED};
use crate::error::YieldVaultError;
use crate::events::AdapterHarvested;
use crate::state::VaultAdapterState;
use crate::POOLVER_CORE_ID;

// Tier 0 generates no yield by definition (spec §5.3.`poolver-yield-vault`).
// We still expose this entrypoint with the same signature as Tier 1's so
// core's `distribute_yield` flow has a single CPI shape (arch §13.1). The
// return value is written via Anchor's `Return` codepath — core reads it
// with `get_return_data` after the CPI.
#[derive(Accounts)]
pub struct AdapterHarvest<'info> {
    #[account(
        seeds = [CORE_INVOKER_SEED],
        seeds::program = POOLVER_CORE_ID,
        bump,
    )]
    pub core_invoker: Signer<'info>,

    #[account(
        seeds = [VAULT_ADAPTER_SEED, adapter_state.pool.as_ref()],
        bump = adapter_state.bump,
    )]
    pub adapter_state: Account<'info, VaultAdapterState>,

    #[account(
        seeds = [VAULT_ADAPTER_USDC_SEED, adapter_state.pool.as_ref()],
        bump,
        constraint = adapter_usdc_vault.key() == adapter_state.usdc_vault
            @ YieldVaultError::Unauthorized,
    )]
    pub adapter_usdc_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_harvest(ctx: Context<AdapterHarvest>) -> Result<u64> {
    let pool = ctx.accounts.adapter_state.pool;

    emit!(AdapterHarvested {
        pool,
        yield_amount: 0,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(0)
}
