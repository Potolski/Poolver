use anchor_lang::prelude::*;

use crate::constants::*;
use crate::error::ConsolError;
use crate::state::{ConsorcioGroup, GroupStatus, Round, RoundStatus};

/// Skip a round when no eligible members exist for selection.
/// This happens when all active members have either already received the pool
/// or all members defaulted during this round.
///
/// Permissionless crank. Advances the round without a winner.
#[derive(Accounts)]
pub struct SkipRound<'info> {
    pub caller: Signer<'info>,

    #[account(
        mut,
        constraint = group.status == GroupStatus::Active @ ConsolError::InvalidGroupState,
    )]
    pub group: Account<'info, ConsorcioGroup>,

    #[account(
        mut,
        seeds = [ROUND_SEED, group.key().as_ref(), &[group.current_round]],
        bump = round.bump,
        constraint = round.status == RoundStatus::Selecting @ ConsolError::InvalidRoundState,
    )]
    pub round: Account<'info, Round>,
}

pub fn handle_skip_round(ctx: Context<SkipRound>) -> Result<()> {
    let group = &ctx.accounts.group;

    // Can only skip if there are truly no eligible winners:
    // 1. No active members remain (everyone defaulted/withdrew)
    // 2. All active members have already received the pool in prior rounds
    // 3. No payments were collected this round (collection already closed via Selecting constraint)
    let no_active = group.active_members == 0;
    let all_received = group.active_members > 0
        && group.members_received >= group.active_members;
    let no_payments = ctx.accounts.round.total_collected == 0;

    require!(no_active || all_received || no_payments, ConsolError::NoEligibleMembers);

    // Mark round as completed with no winner
    let round = &mut ctx.accounts.round;
    round.status = RoundStatus::Completed;
    round.winner_selected = false;

    // Advance group to next round
    let group = &mut ctx.accounts.group;
    group.current_round += 1;

    // If we've exhausted all rounds, complete the group
    if group.current_round >= group.total_members || group.active_members == 0 {
        group.status = GroupStatus::Completed;
    }

    Ok(())
}
