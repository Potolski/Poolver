use anchor_lang::prelude::*;

use crate::constants::*;
use crate::error::ConsolError;
use crate::events::RoundStarted;
use crate::state::{ConsorcioGroup, GroupStatus, Round, RoundStatus};

#[derive(Accounts)]
pub struct StartRound<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,

    #[account(
        mut,
        constraint = group.status == GroupStatus::Active @ ConsolError::InvalidGroupState,
    )]
    pub group: Account<'info, ConsorcioGroup>,

    #[account(
        init,
        payer = caller,
        space = 8 + Round::INIT_SPACE,
        seeds = [ROUND_SEED, group.key().as_ref(), &[group.current_round]],
        bump,
    )]
    pub round: Account<'info, Round>,

    pub system_program: Program<'info, System>,
}

pub fn handle_start_round(ctx: Context<StartRound>) -> Result<()> {
    let clock = Clock::get()?;
    let group = &mut ctx.accounts.group;

    // Ensure all members have received = group is done, or we haven't exceeded total rounds
    require!(
        group.current_round < group.total_members,
        ConsolError::InvalidGroupState
    );

    let round = &mut ctx.accounts.round;
    round.group = group.key();
    round.round_number = group.current_round;
    round.total_collected = 0;
    round.payments_received = 0;
    round.lottery_winner = Pubkey::default();
    round.winner_selected = false;
    round.distribution_claimed = false;
    round.distribution_amount = 0;
    round.vrf_result = [0u8; 32];
    round.commit_slot = 0;
    round.randomness_account = Pubkey::default();
    round.status = RoundStatus::Collecting;
    round.started_at = clock.unix_timestamp;
    round.bump = ctx.bumps.round;

    group.round_started_at = clock.unix_timestamp;

    emit!(RoundStarted {
        group: group.key(),
        round: group.current_round,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
