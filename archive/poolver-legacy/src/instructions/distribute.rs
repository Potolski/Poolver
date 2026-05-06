use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::constants::*;
use crate::error::ConsolError;
use crate::events::DistributionClaimed;
use crate::state::{ConsorcioGroup, GroupStatus, Member, MemberStatus, Round, RoundStatus};

#[derive(Accounts)]
pub struct Distribute<'info> {
    pub caller: Signer<'info>,

    #[account(
        mut,
        constraint = group.status == GroupStatus::Active @ ConsolError::InvalidGroupState,
    )]
    pub group: Box<Account<'info, ConsorcioGroup>>,

    #[account(
        mut,
        seeds = [ROUND_SEED, group.key().as_ref(), &[group.current_round]],
        bump = round.bump,
        constraint = round.status == RoundStatus::Distributing @ ConsolError::InvalidRoundState,
        constraint = round.winner_selected @ ConsolError::VrfNotResolved,
    )]
    pub round: Box<Account<'info, Round>>,

    /// The winner's member account
    #[account(
        mut,
        seeds = [MEMBER_SEED, group.key().as_ref(), winner.wallet.as_ref()],
        bump = winner.bump,
        constraint = winner.status == MemberStatus::Active @ ConsolError::MemberDefaulted,
        constraint = !winner.has_received @ ConsolError::AlreadyReceived,
        constraint = winner.wallet == round.lottery_winner @ ConsolError::NotEligible,
    )]
    pub winner: Box<Account<'info, Member>>,

    #[account(address = group.mint)]
    pub mint: Account<'info, Mint>,

    /// Winner's token account to receive the pool
    #[account(
        mut,
        token::mint = mint,
        token::authority = winner.wallet,
    )]
    pub winner_token_account: Account<'info, TokenAccount>,

    /// Main vault (source of distribution)
    #[account(
        mut,
        seeds = [VAULT_SEED, group.key().as_ref()],
        bump = group.vault_bump,
    )]
    pub vault: Account<'info, TokenAccount>,

    /// Protocol treasury vault (receives protocol fees)
    #[account(
        mut,
        seeds = [TREASURY_SEED, group.key().as_ref()],
        bump = group.treasury_bump,
    )]
    pub treasury_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_distribute(ctx: Context<Distribute>) -> Result<()> {
    let clock = Clock::get()?;
    let group = &ctx.accounts.group;
    let round = &ctx.accounts.round;

    // Calculate distribution: total_collected minus protocol fee
    let protocol_fee: u64 = (round.total_collected as u128)
        .checked_mul(group.protocol_fee_bps as u128)
        .ok_or(ConsolError::MathOverflow)?
        .checked_div(10_000)
        .ok_or(ConsolError::MathOverflow)?
        .try_into()
        .map_err(|_| ConsolError::MathOverflow)?;

    let distribution_amount = round
        .total_collected
        .checked_sub(protocol_fee)
        .ok_or(ConsolError::MathOverflow)?;

    // Transfer from vault to winner (PDA signer)
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
                to: ctx.accounts.winner_token_account.to_account_info(),
                authority: ctx.accounts.vault.to_account_info(),
            },
            &[vault_seeds],
        ),
        distribution_amount,
    )?;

    // Transfer protocol fee to treasury vault
    if protocol_fee > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.key(),
                Transfer {
                    from: ctx.accounts.vault.to_account_info(),
                    to: ctx.accounts.treasury_vault.to_account_info(),
                    authority: ctx.accounts.vault.to_account_info(),
                },
                &[vault_seeds],
            ),
            protocol_fee,
        )?;
    }

    // Update winner member
    let winner = &mut ctx.accounts.winner;
    winner.has_received = true;
    winner.received_round = group.current_round;

    // Update round
    let round = &mut ctx.accounts.round;
    round.distribution_amount = distribution_amount;
    round.distribution_claimed = true;
    round.status = RoundStatus::Completed;

    // Update group
    let group = &mut ctx.accounts.group;
    group.members_received += 1;
    group.current_round += 1;

    // Check if all members have received → complete the group
    if group.members_received >= group.total_members {
        group.status = GroupStatus::Completed;
    }

    emit!(DistributionClaimed {
        group: group.key(),
        round: round.round_number,
        member: winner.wallet,
        amount: distribution_amount,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
