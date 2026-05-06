use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::constants::{CORE_INVOKER_SEED, RESERVE_FUND_SEED, RESERVE_VAULT_SEED};
use crate::error::ReserveError;
use crate::events::ReserveDraw;
use crate::state::ReserveFund;
use crate::POOLVER_CORE_ID;

// `draw` is CPI-only from core, called during `liquidate_default` when
// the seized collateral cannot fully cover the participant's outstanding
// obligations. Arch §5.1 matrix + spec §5.2.
//
// IMPORTANT (arch §5.4): core's `liquidate_default` is responsible for
// catching `ReserveInsufficient` gracefully and applying partial coverage.
// The reserve simply enforces the invariant — INV-2: balance never goes
// negative.
#[derive(Accounts)]
pub struct ReserveDrawCtx<'info> {
    #[account(
        seeds = [CORE_INVOKER_SEED],
        seeds::program = POOLVER_CORE_ID,
        bump,
    )]
    pub core_invoker: Signer<'info>,

    #[account(
        mut,
        seeds = [RESERVE_FUND_SEED, &[reserve_fund.tier.as_u8()]],
        bump = reserve_fund.bump,
    )]
    pub reserve_fund: Account<'info, ReserveFund>,

    #[account(
        mut,
        seeds = [RESERVE_VAULT_SEED, &[reserve_fund.tier.as_u8()]],
        bump,
        constraint = reserve_usdc_vault.key() == reserve_fund.usdc_vault
            @ ReserveError::Unauthorized,
    )]
    pub reserve_usdc_vault: Account<'info, TokenAccount>,

    /// Where to send the drawn USDC. Core's `liquidate_default` flow
    /// chooses this; we don't constrain the destination beyond "is a token
    /// account" — the SPL transfer enforces same-mint.
    #[account(mut)]
    pub destination_usdc: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_draw(ctx: Context<ReserveDrawCtx>, amount: u64) -> Result<()> {
    require!(amount > 0, ReserveError::InvalidAmount);

    // INV-2: pre-flight balance check. The `checked_sub` below would also
    // catch underflow, but a pre-check produces the canonical error code
    // (`ReserveInsufficient`) that core's `try_draw_or_partial` wrapper
    // (arch §5.4) keys off.
    require!(
        ctx.accounts.reserve_fund.total_balance >= amount,
        ReserveError::ReserveInsufficient
    );

    let tier = ctx.accounts.reserve_fund.tier;
    let usdc_vault_bump = ctx.bumps.reserve_usdc_vault;
    // Stored bumps for signer seeds (arch §4 bump policy). The bump was
    // already verified by the `seeds = ...` constraint above; reusing
    // `ctx.bumps` is canonical and saves the ~1500 CU `find_program_address`
    // would cost.
    let tier_byte = [tier.as_u8()];
    let signer_seeds: &[&[&[u8]]] = &[&[
        RESERVE_VAULT_SEED,
        &tier_byte,
        &[usdc_vault_bump],
    ]];

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        Transfer {
            from: ctx.accounts.reserve_usdc_vault.to_account_info(),
            to: ctx.accounts.destination_usdc.to_account_info(),
            authority: ctx.accounts.reserve_usdc_vault.to_account_info(),
        },
        signer_seeds,
    );
    token::transfer(cpi_ctx, amount)?;

    let fund = &mut ctx.accounts.reserve_fund;
    // Defensive: the require!() above already proved this won't underflow,
    // so reaching the `MathOverflow` branch here is a logic-bug signal.
    fund.total_balance = fund
        .total_balance
        .checked_sub(amount)
        .ok_or(ReserveError::MathOverflow)?;
    // INV-3: lifetime counters monotonic non-decreasing. Outflow goes up
    // by exactly `amount`.
    fund.total_outflows = fund
        .total_outflows
        .checked_add(amount)
        .ok_or(ReserveError::MathOverflow)?;

    emit!(ReserveDraw {
        tier: fund.tier,
        amount,
        total_balance: fund.total_balance,
        total_outflows: fund.total_outflows,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}
