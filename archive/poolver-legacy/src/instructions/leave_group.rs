use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::constants::*;
use crate::error::ConsolError;
use crate::events::MemberLeft;
use crate::state::{ConsorcioGroup, GroupStatus, Member, MemberStatus};

#[derive(Accounts)]
pub struct LeaveGroup<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        constraint = group.status == GroupStatus::Forming @ ConsolError::InvalidGroupState,
    )]
    pub group: Account<'info, ConsorcioGroup>,

    #[account(
        mut,
        seeds = [MEMBER_SEED, group.key().as_ref(), user.key().as_ref()],
        bump = member.bump,
        constraint = member.wallet == user.key() @ ConsolError::NotMember,
        constraint = member.status == MemberStatus::Active @ ConsolError::MemberDefaulted,
        close = user,
    )]
    pub member: Account<'info, Member>,

    #[account(address = group.mint)]
    pub mint: Account<'info, Mint>,

    /// User's USDC token account (receives refund)
    #[account(
        mut,
        token::mint = mint,
        token::authority = user,
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    /// Group vault (source of refund)
    #[account(
        mut,
        seeds = [VAULT_SEED, group.key().as_ref()],
        bump = group.vault_bump,
    )]
    pub vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handle_leave_group(ctx: Context<LeaveGroup>) -> Result<()> {
    let clock = Clock::get()?;

    let refund_amount = ctx.accounts.member.collateral_deposited;

    // Transfer collateral back from vault to user (PDA signer)
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
                to: ctx.accounts.user_token_account.to_account_info(),
                authority: ctx.accounts.vault.to_account_info(),
            },
            &[vault_seeds],
        ),
        refund_amount,
    )?;

    // Update group counts
    let group = &mut ctx.accounts.group;
    group.current_members -= 1;
    group.active_members -= 1;

    // member account is closed via `close = user` constraint, rent returned

    emit!(MemberLeft {
        group: group.key(),
        member: ctx.accounts.user.key(),
        refund_amount,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
