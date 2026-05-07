use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Token};

use crate::constants::{RESERVE_FUND_SEED, RESERVE_VAULT_SEED};
use crate::events::ReserveClosed;
use crate::state::{ReserveFund, Tier};

// SPEC_QUESTION-26: tier-scoped tear-down of `ReserveFund` + its USDC vault
// so the next `initialize_reserve(tier)` can rebind to a fresh USDC mint.
// Used post-deploy when the protocol's USDC binding turns out to be wrong
// (devnet fix-up). Mirrors the permissiveness of `initialize_reserve` —
// any signer for V1 — because reserves at this point are guaranteed empty
// (the SPL `CloseAccount` CPI fails non-empty token accounts, which is the
// structural guarantee no funds are silently destroyed).
//
// V2 multi-sig should tighten the auth model here in lock-step with
// `initialize_reserve` (validate against `ProtocolConfig.admin` via CPI).
#[derive(Accounts)]
#[instruction(tier: Tier)]
pub struct AdminCloseReserve<'info> {
    /// Receives the rent refund from both the `ReserveFund` PDA and the
    /// `reserve_usdc_vault` token account. SPEC_QUESTION-26: V1 accepts
    /// any signer (matches `initialize_reserve`).
    #[account(mut)]
    pub caller: Signer<'info>,

    #[account(
        mut,
        close = caller,
        seeds = [RESERVE_FUND_SEED, &[tier.as_u8()]],
        bump = reserve_fund.bump,
    )]
    pub reserve_fund: Account<'info, ReserveFund>,

    /// CHECK: closed via SPL Token CPI in handler. PDA-derived; self-authority.
    /// We don't deserialize as `TokenAccount` because the close CPI inside
    /// the handler invalidates the discriminator before Anchor's drop-time
    /// re-serialise check would run.
    #[account(
        mut,
        seeds = [RESERVE_VAULT_SEED, &[tier.as_u8()]],
        bump,
    )]
    pub reserve_usdc_vault: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_admin_close_reserve(ctx: Context<AdminCloseReserve>, tier: Tier) -> Result<()> {
    // The reserve_usdc_vault is its own authority — sign with its own seeds.
    let bump = ctx.bumps.reserve_usdc_vault;
    let tier_byte = [tier.as_u8()];
    let signer_seeds: &[&[&[u8]]] = &[&[RESERVE_VAULT_SEED, &tier_byte, &[bump]]];

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        CloseAccount {
            account: ctx.accounts.reserve_usdc_vault.to_account_info(),
            destination: ctx.accounts.caller.to_account_info(),
            authority: ctx.accounts.reserve_usdc_vault.to_account_info(),
        },
        signer_seeds,
    );
    token::close_account(cpi_ctx)?;

    emit!(ReserveClosed {
        tier,
        reserve_fund: ctx.accounts.reserve_fund.key(),
        reserve_usdc_vault: ctx.accounts.reserve_usdc_vault.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });

    // The `ReserveFund` PDA is closed by Anchor's `close = caller` constraint
    // automatically at drop time (data zeroed, lamports refunded).
    Ok(())
}
