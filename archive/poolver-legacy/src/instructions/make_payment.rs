use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::constants::*;
use crate::error::ConsolError;
use crate::events::PaymentMade;
use crate::state::{ConsorcioGroup, GroupStatus, Member, MemberStatus, Round, RoundStatus};

#[derive(Accounts)]
pub struct MakePayment<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        constraint = group.status == GroupStatus::Active @ ConsolError::InvalidGroupState,
    )]
    pub group: Box<Account<'info, ConsorcioGroup>>,

    #[account(
        mut,
        seeds = [MEMBER_SEED, group.key().as_ref(), user.key().as_ref()],
        bump = member.bump,
        constraint = member.wallet == user.key() @ ConsolError::NotMember,
        constraint = member.status == MemberStatus::Active @ ConsolError::MemberDefaulted,
    )]
    pub member: Box<Account<'info, Member>>,

    #[account(
        mut,
        seeds = [ROUND_SEED, group.key().as_ref(), &[group.current_round]],
        bump = round.bump,
        constraint = round.status == RoundStatus::Collecting @ ConsolError::PaymentWindowClosed,
    )]
    pub round: Box<Account<'info, Round>>,

    #[account(address = group.mint)]
    pub mint: Account<'info, Mint>,

    #[account(
        mut,
        token::mint = mint,
        token::authority = user,
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [VAULT_SEED, group.key().as_ref()],
        bump = group.vault_bump,
    )]
    pub vault: Account<'info, TokenAccount>,

    /// Insurance vault receives the insurance portion
    #[account(
        mut,
        seeds = [INSURANCE_SEED, group.key().as_ref()],
        bump = group.insurance_bump,
    )]
    pub insurance_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_make_payment(ctx: Context<MakePayment>) -> Result<()> {
    let clock = Clock::get()?;
    let group = &ctx.accounts.group;
    let member = &ctx.accounts.member;
    let current_round = group.current_round;

    // Check member hasn't already paid this round.
    // payments_made tracks total payments; if it exceeds current_round the member
    // has already paid for this round (rounds are 0-indexed).
    require!(
        member.payments_made <= current_round,
        ConsolError::AlreadyPaid
    );

    // Check if within payment window or grace period
    let elapsed = clock
        .unix_timestamp
        .checked_sub(group.round_started_at)
        .ok_or(ConsolError::MathOverflow)?;
    let window_secs = PAYMENT_WINDOW_DAYS * 24 * 60 * 60;
    let grace_secs = GRACE_PERIOD_DAYS * 24 * 60 * 60;

    require!(
        elapsed <= window_secs + grace_secs,
        ConsolError::PaymentWindowClosed
    );

    let is_late = elapsed > window_secs;

    // Calculate payment: base contribution + late fee if applicable
    let base_amount = group.monthly_contribution;
    let late_fee: u64 = if is_late {
        (base_amount as u128)
            .checked_mul(LATE_FEE_BPS as u128)
            .ok_or(ConsolError::MathOverflow)?
            .checked_div(10_000)
            .ok_or(ConsolError::MathOverflow)?
            .try_into()
            .map_err(|_| ConsolError::MathOverflow)?
    } else {
        0
    };
    let total_payment = base_amount
        .checked_add(late_fee)
        .ok_or(ConsolError::MathOverflow)?;

    // Split: insurance portion goes to insurance vault, rest to main vault
    let insurance_amount: u64 = (base_amount as u128)
        .checked_mul(group.insurance_bps as u128)
        .ok_or(ConsolError::MathOverflow)?
        .checked_div(10_000)
        .ok_or(ConsolError::MathOverflow)?
        .try_into()
        .map_err(|_| ConsolError::MathOverflow)?;

    let vault_amount = total_payment
        .checked_sub(insurance_amount)
        .ok_or(ConsolError::MathOverflow)?;

    // Transfer to main vault
    if vault_amount > 0 {
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.key(),
                Transfer {
                    from: ctx.accounts.user_token_account.to_account_info(),
                    to: ctx.accounts.vault.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            vault_amount,
        )?;
    }

    // Transfer insurance portion
    if insurance_amount > 0 {
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.key(),
                Transfer {
                    from: ctx.accounts.user_token_account.to_account_info(),
                    to: ctx.accounts.insurance_vault.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            insurance_amount,
        )?;
    }

    // Update member
    let member = &mut ctx.accounts.member;
    member.payments_made += 1;
    member.total_paid = member
        .total_paid
        .checked_add(total_payment)
        .ok_or(ConsolError::MathOverflow)?;
    member.last_paid_round = current_round + 1; // mark as paid for this round

    // Update round
    let round = &mut ctx.accounts.round;
    round.total_collected = round
        .total_collected
        .checked_add(vault_amount)
        .ok_or(ConsolError::MathOverflow)?;
    round.payments_received += 1;

    emit!(PaymentMade {
        group: group.key(),
        member: ctx.accounts.user.key(),
        round: current_round,
        amount: total_payment,
        is_late,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
