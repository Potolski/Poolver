use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::constants::{CORE_INVOKER_SEED, RESERVE_FUND_SEED, RESERVE_VAULT_SEED};
use crate::error::ReserveError;
use crate::events::ReserveDeposit;
use crate::state::ReserveFund;
use crate::POOLVER_CORE_ID;

// `deposit` is CPI-only from core. Triggered on the inflow side of every
// fee-collecting instruction (`join_pool`, `contribute`, `claim_winning`'s
// 20% bid carve-out, `liquidate_default` forfeit, `distribute_yield`'s 20%
// carve-out). Arch §5.1 matrix.
//
// The reserve identified is structurally tied to its tier through the
// PDA seed — there is no `tier` argument here. Core derives the seed from
// `pool.tier` on its side; we simply accept the account that comes in,
// and Anchor rejects any seed mismatch with `ConstraintSeeds`. INV-4.
#[derive(Accounts)]
pub struct ReserveDepositCtx<'info> {
    /// PDA-as-signer proving the call comes from `poolver-core` (arch §5.2).
    /// `seeds::program = POOLVER_CORE_ID` anchors the derivation to core's
    /// program ID; no other caller can mint a matching signature.
    #[account(
        seeds = [CORE_INVOKER_SEED],
        seeds::program = POOLVER_CORE_ID,
        bump,
    )]
    pub core_invoker: Signer<'info>,

    /// The reserve fund itself. INV-4: tier comes from the seed, not from
    /// instruction args. Caller passing the wrong-tier reserve gets
    /// `ConstraintSeeds`; this is the structural enforcement promised by
    /// arch §11.
    #[account(
        mut,
        seeds = [RESERVE_FUND_SEED, &[reserve_fund.tier.as_u8()]],
        bump = reserve_fund.bump,
    )]
    pub reserve_fund: Account<'info, ReserveFund>,

    /// PDA-owned USDC vault for this tier. Same tier-encoded seed.
    #[account(
        mut,
        seeds = [RESERVE_VAULT_SEED, &[reserve_fund.tier.as_u8()]],
        bump,
        constraint = reserve_usdc_vault.key() == reserve_fund.usdc_vault
            @ ReserveError::Unauthorized,
    )]
    pub reserve_usdc_vault: Account<'info, TokenAccount>,

    /// Source of funds. Core passes the pool's `PoolUsdcVault` (or another
    /// core-controlled token account) here. The authority signing the SPL
    /// transfer is forwarded as `source_authority`.
    #[account(mut)]
    pub source_usdc: Account<'info, TokenAccount>,

    /// Authority over `source_usdc`. Required to sign the SPL transfer —
    /// in practice this is a core-owned PDA (e.g. the pool USDC vault PDA).
    /// We don't constrain this further because core handles ownership on
    /// its side (arch §5.1).
    pub source_authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_deposit(ctx: Context<ReserveDepositCtx>, amount: u64) -> Result<()> {
    require!(amount > 0, ReserveError::InvalidAmount);

    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.key(),
        Transfer {
            from: ctx.accounts.source_usdc.to_account_info(),
            to: ctx.accounts.reserve_usdc_vault.to_account_info(),
            authority: ctx.accounts.source_authority.to_account_info(),
        },
    );
    token::transfer(cpi_ctx, amount)?;

    let fund = &mut ctx.accounts.reserve_fund;
    fund.total_balance = fund
        .total_balance
        .checked_add(amount)
        .ok_or(ReserveError::MathOverflow)?;
    // INV-3: lifetime counters monotonic non-decreasing. Inflow goes up by
    // exactly `amount`; the post-mutation identity
    // `total_balance == total_inflows − total_outflows` is preserved.
    fund.total_inflows = fund
        .total_inflows
        .checked_add(amount)
        .ok_or(ReserveError::MathOverflow)?;

    emit!(ReserveDeposit {
        tier: fund.tier,
        amount,
        total_balance: fund.total_balance,
        total_inflows: fund.total_inflows,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}
