use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::constants::*;
use crate::error::ConsolError;
use crate::state::{ConsorcioGroup, GroupStatus, Member, MemberStatus};

/// Return collateral to a non-defaulted member after group completion.
/// Called once per member. Anyone can crank it.
#[derive(Accounts)]
pub struct ReturnCollateral<'info> {
    pub caller: Signer<'info>,

    #[account(
        constraint = group.status == GroupStatus::Completed || group.status == GroupStatus::Cancelled @ ConsolError::InvalidGroupState,
    )]
    pub group: Box<Account<'info, ConsorcioGroup>>,

    #[account(
        mut,
        seeds = [MEMBER_SEED, group.key().as_ref(), member.wallet.as_ref()],
        bump = member.bump,
        constraint = member.status == MemberStatus::Active @ ConsolError::MemberDefaulted,
        constraint = member.collateral_deposited > 0 @ ConsolError::InsufficientCollateral,
    )]
    pub member: Box<Account<'info, Member>>,

    #[account(address = group.mint)]
    pub mint: Account<'info, Mint>,

    /// Member's token account to receive collateral back
    #[account(
        mut,
        token::mint = mint,
        token::authority = member.wallet,
    )]
    pub member_token_account: Account<'info, TokenAccount>,

    /// Group vault holding the collateral
    #[account(
        mut,
        seeds = [VAULT_SEED, group.key().as_ref()],
        bump = group.vault_bump,
    )]
    pub vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_return_collateral(ctx: Context<ReturnCollateral>) -> Result<()> {
    let collateral = ctx.accounts.member.collateral_deposited;

    // Transfer collateral from vault back to member (PDA signer)
    let group_key = ctx.accounts.group.key();
    let vault_seeds: &[&[u8]] = &[
        VAULT_SEED,
        group_key.as_ref(),
        &[ctx.accounts.group.vault_bump],
    ];

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            Transfer {
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.member_token_account.to_account_info(),
                authority: ctx.accounts.vault.to_account_info(),
            },
            &[vault_seeds],
        ),
        collateral,
    )?;

    // Zero out collateral so this can't be called again
    let member = &mut ctx.accounts.member;
    member.collateral_deposited = 0;

    Ok(())
}
