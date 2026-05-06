use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::constants::*;
use crate::error::ConsolError;
use crate::events::MemberJoined;
use crate::state::{ConsorcioGroup, GroupStatus, Member, MemberStatus};

#[derive(Accounts)]
pub struct JoinGroup<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        constraint = group.status == GroupStatus::Forming @ ConsolError::InvalidGroupState,
        constraint = group.current_members < group.total_members @ ConsolError::GroupFull,
    )]
    pub group: Account<'info, ConsorcioGroup>,

    #[account(
        init,
        payer = user,
        space = 8 + Member::INIT_SPACE,
        seeds = [MEMBER_SEED, group.key().as_ref(), user.key().as_ref()],
        bump,
    )]
    pub member: Account<'info, Member>,

    #[account(address = group.mint)]
    pub mint: Account<'info, Mint>,

    /// User's USDC token account (source of collateral)
    #[account(
        mut,
        token::mint = mint,
        token::authority = user,
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    /// Group vault to receive collateral
    #[account(
        mut,
        seeds = [VAULT_SEED, group.key().as_ref()],
        bump = group.vault_bump,
    )]
    pub vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handle_join_group(ctx: Context<JoinGroup>) -> Result<()> {
    let clock = Clock::get()?;

    require!(
        clock.unix_timestamp <= ctx.accounts.group.formation_deadline,
        ConsolError::FormationTimeout
    );

    let group = &ctx.accounts.group;

    // collateral = (monthly_contribution * total_members * collateral_bps) / 10_000
    let total_obligation = (group.monthly_contribution as u128)
        .checked_mul(group.total_members as u128)
        .ok_or(ConsolError::MathOverflow)?;
    let collateral_amount: u64 = total_obligation
        .checked_mul(group.collateral_bps as u128)
        .ok_or(ConsolError::MathOverflow)?
        .checked_div(10_000)
        .ok_or(ConsolError::MathOverflow)?
        .try_into()
        .map_err(|_| ConsolError::MathOverflow)?;

    // Transfer collateral from user to vault
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            Transfer {
                from: ctx.accounts.user_token_account.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        collateral_amount,
    )?;

    // Initialize member account
    let member = &mut ctx.accounts.member;
    member.group = ctx.accounts.group.key();
    member.wallet = ctx.accounts.user.key();
    member.collateral_deposited = collateral_amount;
    member.payments_made = 0;
    member.payments_missed = 0;
    member.has_received = false;
    member.received_round = u8::MAX;
    member.total_paid = 0;
    member.last_paid_round = 0;
    member.last_default_round = u8::MAX; // sentinel: never defaulted
    member.insurance_claimed = false;
    member.status = MemberStatus::Active;
    member.joined_at = clock.unix_timestamp;
    member.bump = ctx.bumps.member;

    // Update group counts
    let group = &mut ctx.accounts.group;
    group.current_members += 1;
    group.active_members += 1;

    emit!(MemberJoined {
        group: group.key(),
        member: ctx.accounts.user.key(),
        collateral_deposited: collateral_amount,
        current_members: group.current_members,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
