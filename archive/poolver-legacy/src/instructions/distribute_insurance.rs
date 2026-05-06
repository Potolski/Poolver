use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::constants::*;
use crate::error::ConsolError;
use crate::events::InsuranceDistributed;
use crate::state::{ConsorcioGroup, GroupStatus, Member, MemberStatus};

/// Distribute insurance pool surplus to a non-defaulted member after group completion.
/// Each member gets: insurance_balance / active_members_at_completion.
/// Called once per eligible member. Anyone can crank it.
#[derive(Accounts)]
pub struct DistributeInsurance<'info> {
    pub caller: Signer<'info>,

    #[account(
        mut,
        constraint = group.status == GroupStatus::Completed || group.status == GroupStatus::Cancelled @ ConsolError::InvalidGroupState,
    )]
    pub group: Box<Account<'info, ConsorcioGroup>>,

    #[account(
        mut,
        seeds = [MEMBER_SEED, group.key().as_ref(), member.wallet.as_ref()],
        bump = member.bump,
        constraint = member.status == MemberStatus::Active @ ConsolError::MemberDefaulted,
        constraint = !member.insurance_claimed @ ConsolError::AlreadyReceived,
    )]
    pub member: Box<Account<'info, Member>>,

    #[account(address = group.mint)]
    pub mint: Account<'info, Mint>,

    /// Member's token account to receive their share
    #[account(
        mut,
        token::mint = mint,
        token::authority = member.wallet,
    )]
    pub member_token_account: Account<'info, TokenAccount>,

    /// Insurance vault holding the surplus
    #[account(
        mut,
        seeds = [INSURANCE_SEED, group.key().as_ref()],
        bump = group.insurance_bump,
    )]
    pub insurance_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_distribute_insurance(ctx: Context<DistributeInsurance>) -> Result<()> {
    let group = &ctx.accounts.group;

    // active_members tracks non-defaulted members — use it as the divisor
    require!(group.active_members > 0, ConsolError::NoEligibleMembers);

    // Calculate this member's share of the insurance surplus.
    // Last claimant gets the full remaining balance to avoid dust lockup.
    let insurance_balance = ctx.accounts.insurance_vault.amount;
    let share = if group.active_members == 1 {
        insurance_balance
    } else {
        insurance_balance
            .checked_div(group.active_members as u64)
            .ok_or(ConsolError::MathOverflow)?
    };

    if share == 0 {
        return Ok(());
    }

    // Transfer from insurance vault (PDA signer)
    let group_key = group.key();
    let insurance_seeds: &[&[u8]] = &[
        INSURANCE_SEED,
        group_key.as_ref(),
        &[group.insurance_bump],
    ];

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            Transfer {
                from: ctx.accounts.insurance_vault.to_account_info(),
                to: ctx.accounts.member_token_account.to_account_info(),
                authority: ctx.accounts.insurance_vault.to_account_info(),
            },
            &[insurance_seeds],
        ),
        share,
    )?;

    // Mark member as having claimed insurance
    let member = &mut ctx.accounts.member;
    member.insurance_claimed = true;

    // Decrement active_members so subsequent calls get correct share
    // (insurance_balance shrinks, active_members shrinks → share stays proportional)
    let group = &mut ctx.accounts.group;
    group.active_members -= 1;

    let clock = Clock::get()?;
    emit!(InsuranceDistributed {
        group: group.key(),
        member: member.wallet,
        amount: share,
        remaining_members: group.active_members,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
