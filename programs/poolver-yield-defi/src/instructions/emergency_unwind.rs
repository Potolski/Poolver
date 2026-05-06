use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::constants::{
    CORE_INVOKER_SEED, DEFI_ADAPTER_KTOKEN_SEED, DEFI_ADAPTER_SEED, DEFI_ADAPTER_USDC_SEED,
    TRIP_REASON_ADMIN_TRIP,
};
use crate::error::YieldDefiError;
use crate::events::{AdapterUnwound, CircuitBreakerTripped};
use crate::state::DefiAdapterState;
use crate::POOLVER_CORE_ID;

// `emergency_unwind` drains BOTH vaults to a single destination and
// latches the breaker (`tripped = true`, reason = AdminTrip). After
// this fires, every state-changing instruction rejects until
// `reset_circuit_breaker` is called. SPEC_QUESTION-19: in production
// this drains the kToken position via a Kamino redeem CPI before
// the SPL transfer; the mock just transfers the simulated USDC out.
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

pub fn handle_emergency_unwind(ctx: Context<AdapterUnwind>) -> Result<()> {
    let pool_key = ctx.accounts.adapter_state.pool;
    let from_liquid = ctx.accounts.adapter_usdc_vault.amount;
    let from_kamino = ctx.accounts.adapter_ktoken_vault.amount;
    let total = from_liquid
        .checked_add(from_kamino)
        .ok_or(YieldDefiError::MathOverflow)?;

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

    if from_kamino > 0 {
        // SPEC_QUESTION-19: real Kamino redeem CPI replaces this; the
        // mock simulates by moving the underlying USDC out 1:1.
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

    let now = Clock::get()?.unix_timestamp;
    let state = &mut ctx.accounts.adapter_state;
    state.total_deposited = 0;
    state.total_deployed_to_kamino = 0;
    state.liquid_reserved = 0;
    // Snapshot resets too — after an unwind there's nothing left to
    // accrue against.
    state.last_recorded_balance = 0;
    // Latch the breaker. Caller (admin or core) must call
    // `reset_circuit_breaker` to bring the adapter back online.
    state.tripped = true;
    state.tripped_reason = TRIP_REASON_ADMIN_TRIP;
    state.tripped_at = now;

    emit!(AdapterUnwound {
        pool: pool_key,
        amount_unwound: total,
        from_liquid,
        from_kamino,
        timestamp: now,
    });
    emit!(CircuitBreakerTripped {
        pool: pool_key,
        reason: TRIP_REASON_ADMIN_TRIP,
        timestamp: now,
    });

    Ok(())
}
