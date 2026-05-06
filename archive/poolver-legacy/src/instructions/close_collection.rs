use anchor_lang::prelude::*;

use crate::constants::*;
use crate::error::ConsolError;
use crate::state::{ConsorcioGroup, GroupStatus, Round, RoundStatus};

#[derive(Accounts)]
pub struct CloseCollection<'info> {
    pub caller: Signer<'info>,

    #[account(
        constraint = group.status == GroupStatus::Active @ ConsolError::InvalidGroupState,
    )]
    pub group: Account<'info, ConsorcioGroup>,

    #[account(
        mut,
        seeds = [ROUND_SEED, group.key().as_ref(), &[group.current_round]],
        bump = round.bump,
        constraint = round.status == RoundStatus::Collecting @ ConsolError::InvalidRoundState,
    )]
    pub round: Account<'info, Round>,
}

pub fn handle_close_collection(ctx: Context<CloseCollection>) -> Result<()> {
    let clock = Clock::get()?;
    let group = &ctx.accounts.group;

    // Can only close after payment window + grace period has elapsed
    let elapsed = clock
        .unix_timestamp
        .checked_sub(group.round_started_at)
        .ok_or(ConsolError::MathOverflow)?;
    let deadline_secs = (PAYMENT_WINDOW_DAYS + GRACE_PERIOD_DAYS) * 24 * 60 * 60;

    require!(elapsed >= deadline_secs, ConsolError::GracePeriodActive);

    let round = &mut ctx.accounts.round;
    round.status = RoundStatus::Selecting;

    Ok(())
}
