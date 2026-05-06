use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::constants::*;
use crate::error::ConsolError;
use crate::events::DefaultMarked;
use crate::state::{ConsorcioGroup, GroupStatus, Member, MemberStatus, Round, RoundStatus};

#[derive(Accounts)]
pub struct MarkDefault<'info> {
    pub caller: Signer<'info>,

    #[account(
        mut,
        constraint = group.status == GroupStatus::Active @ ConsolError::InvalidGroupState,
    )]
    pub group: Box<Account<'info, ConsorcioGroup>>,

    /// The member being marked as defaulting — anyone can call this (permissionless crank)
    #[account(
        mut,
        seeds = [MEMBER_SEED, group.key().as_ref(), member.wallet.as_ref()],
        bump = member.bump,
        constraint = member.status == MemberStatus::Active @ ConsolError::MemberDefaulted,
    )]
    pub member: Box<Account<'info, Member>>,

    #[account(
        seeds = [ROUND_SEED, group.key().as_ref(), &[group.current_round]],
        bump = round.bump,
        constraint = round.status == RoundStatus::Selecting || round.status == RoundStatus::Distributing @ ConsolError::InvalidRoundState,
    )]
    pub round: Box<Account<'info, Round>>,

    #[account(address = group.mint)]
    pub mint: Account<'info, Mint>,

    /// Vault holding collateral (source of slash)
    #[account(
        mut,
        seeds = [VAULT_SEED, group.key().as_ref()],
        bump = group.vault_bump,
    )]
    pub vault: Account<'info, TokenAccount>,

    /// Insurance vault (receives slashed collateral)
    #[account(
        mut,
        seeds = [INSURANCE_SEED, group.key().as_ref()],
        bump = group.insurance_bump,
    )]
    pub insurance_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_mark_default(ctx: Context<MarkDefault>) -> Result<()> {
    let clock = Clock::get()?;
    let member = &ctx.accounts.member;
    let group = &ctx.accounts.group;
    let current_round = group.current_round;

    // Verify member hasn't paid this round
    // last_paid_round uses current_round + 1 as the "paid" marker
    require!(
        member.last_paid_round <= current_round,
        ConsolError::AlreadyPaid
    );

    // Prevent double-marking in the same round
    require!(
        member.last_default_round != current_round,
        ConsolError::AlreadyPaid
    );

    let new_missed = member.payments_missed + 1;

    // Calculate collateral slash based on offense count
    // 1st: 10%, 2nd: 25%, 3rd+: 100% (full default)
    let slash_bps: u64 = match new_missed {
        1 => 1_000,  // 10%
        2 => 2_500,  // 25%
        _ => 10_000, // 100%
    };

    let collateral = member.collateral_deposited;
    let slash_amount: u64 = (collateral as u128)
        .checked_mul(slash_bps as u128)
        .ok_or(ConsolError::MathOverflow)?
        .checked_div(10_000)
        .ok_or(ConsolError::MathOverflow)?
        .try_into()
        .map_err(|_| ConsolError::MathOverflow)?;

    // Cap slash to remaining collateral
    let actual_slash = slash_amount.min(collateral);

    // Transfer slashed collateral from vault to insurance vault (PDA signer)
    if actual_slash > 0 {
        let group_key = group.key();
        let vault_seeds: &[&[u8]] = &[
            VAULT_SEED,
            group_key.as_ref(),
            &[group.vault_bump],
        ];

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.key(),
                Transfer {
                    from: ctx.accounts.vault.to_account_info(),
                    to: ctx.accounts.insurance_vault.to_account_info(),
                    authority: ctx.accounts.vault.to_account_info(),
                },
                &[vault_seeds],
            ),
            actual_slash,
        )?;
    }

    // Update member
    let member = &mut ctx.accounts.member;
    member.payments_missed = new_missed;
    member.last_default_round = current_round;
    member.collateral_deposited = collateral
        .checked_sub(actual_slash)
        .ok_or(ConsolError::MathOverflow)?;

    // Full default if threshold reached
    if new_missed >= MAX_MISSED_PAYMENTS {
        member.status = MemberStatus::Defaulted;
        let group = &mut ctx.accounts.group;
        group.active_members -= 1;
    }

    emit!(DefaultMarked {
        group: ctx.accounts.group.key(),
        member: member.wallet,
        round: current_round,
        collateral_slashed: actual_slash,
        total_missed: new_missed,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
