use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::constants::{CORE_INVOKER_SEED, VAULT_ADAPTER_SEED, VAULT_ADAPTER_USDC_SEED};
use crate::events::AdapterInitialized;
use crate::state::VaultAdapterState;
use crate::POOLVER_CORE_ID;

// `initialize_adapter` is NOT in the spec §5.3 common interface — that
// section assumes core wires the per-pool token vault during `create_pool`
// (spec §5.1). The cleanest factoring is a CPI into this adapter so the
// PDA-as-signer authority and the seed table (arch §3.8 + §4) stay
// co-located with the program that owns them. See arch §3.8 for the field
// layout this matches.
#[derive(Accounts)]
#[instruction(pool: Pubkey)]
pub struct InitializeAdapter<'info> {
    /// PDA-as-signer proving the call comes from `poolver-core` (arch §5.2).
    /// The `seeds::program` clause anchors the derivation to core's program
    /// ID; no other caller can mint a matching signature.
    #[account(
        seeds = [CORE_INVOKER_SEED],
        seeds::program = POOLVER_CORE_ID,
        bump,
    )]
    pub core_invoker: Signer<'info>,

    /// Pays for both the state account and the token-account rent. Core
    /// proxies this from the pool creator.
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + VaultAdapterState::INIT_SPACE,
        seeds = [VAULT_ADAPTER_SEED, pool.as_ref()],
        bump,
    )]
    pub adapter_state: Account<'info, VaultAdapterState>,

    /// USDC mint (6 decimals; checked at runtime to avoid baking the mint
    /// pubkey into the program — Anchor's mint constraint is sufficient).
    pub usdc_mint: Account<'info, Mint>,

    /// PDA-owned USDC vault. Authority = the token-account itself (so its
    /// own seeds sign for it during `withdraw` / `emergency_unwind`).
    #[account(
        init,
        payer = payer,
        seeds = [VAULT_ADAPTER_USDC_SEED, pool.as_ref()],
        bump,
        token::mint = usdc_mint,
        token::authority = adapter_usdc_vault,
    )]
    pub adapter_usdc_vault: Account<'info, TokenAccount>,

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
    state.total_deposited = 0;
    state.bump = ctx.bumps.adapter_state;

    emit!(AdapterInitialized {
        pool,
        adapter_state: state.key(),
        usdc_vault: state.usdc_vault,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}
