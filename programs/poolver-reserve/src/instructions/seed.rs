use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::constants::{RESERVE_FUND_SEED, RESERVE_VAULT_SEED};
use crate::error::ReserveError;
use crate::events::ReserveSeeded;
use crate::state::ReserveFund;

// `seed` is the admin top-up flow. Functionally identical to `deposit`
// minus the `core_invoker` auth (admin signs directly) and emits a
// distinct event so an indexer can attribute genesis liquidity vs core-
// driven inflows. Spec §5.2.
//
// SPEC_QUESTION-26: V1 accepts any signer because the `admin` field will
// live in `poolver-core::ProtocolConfig`, which doesn't exist yet. Tighten
// to `require_keys_eq!(funder.key(), protocol_config.admin)` when core
// lands. INV-12 is preserved structurally: this instruction can ONLY add
// to a reserve, never withdraw — so even with an unrestricted signer the
// only "harm" is anyone topping up the protocol's safety net at their own
// expense.
#[derive(Accounts)]
pub struct ReserveSeedCtx<'info> {
    /// Admin (or, in V1, anyone) topping up the reserve.
    /// SPEC_QUESTION-26: tighten to ProtocolConfig.admin when core lands.
    pub funder: Signer<'info>,

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

    /// Source USDC. Owned/signed by `funder`.
    #[account(mut)]
    pub source_usdc: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_seed(ctx: Context<ReserveSeedCtx>, amount: u64) -> Result<()> {
    require!(amount > 0, ReserveError::InvalidAmount);

    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.key(),
        Transfer {
            from: ctx.accounts.source_usdc.to_account_info(),
            to: ctx.accounts.reserve_usdc_vault.to_account_info(),
            authority: ctx.accounts.funder.to_account_info(),
        },
    );
    token::transfer(cpi_ctx, amount)?;

    let fund = &mut ctx.accounts.reserve_fund;
    fund.total_balance = fund
        .total_balance
        .checked_add(amount)
        .ok_or(ReserveError::MathOverflow)?;
    // INV-3: same identity discipline as `deposit`. `seed` is an inflow.
    fund.total_inflows = fund
        .total_inflows
        .checked_add(amount)
        .ok_or(ReserveError::MathOverflow)?;

    emit!(ReserveSeeded {
        tier: fund.tier,
        amount,
        total_balance: fund.total_balance,
        total_inflows: fund.total_inflows,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}
