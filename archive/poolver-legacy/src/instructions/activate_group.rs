use anchor_lang::prelude::*;

use crate::error::ConsolError;
use crate::events::GroupActivated;
use crate::state::{ConsorcioGroup, GroupStatus};

#[derive(Accounts)]
pub struct ActivateGroup<'info> {
    #[account(
        constraint = caller.key() == group.creator @ ConsolError::InvalidGroupState,
    )]
    pub caller: Signer<'info>,

    #[account(
        mut,
        constraint = group.status == GroupStatus::Forming @ ConsolError::InvalidGroupState,
        constraint = group.current_members == group.total_members @ ConsolError::InvalidGroupState,
    )]
    pub group: Account<'info, ConsorcioGroup>,
}

pub fn handle_activate_group(ctx: Context<ActivateGroup>) -> Result<()> {
    let clock = Clock::get()?;
    let group = &mut ctx.accounts.group;

    group.status = GroupStatus::Active;
    group.round_started_at = clock.unix_timestamp;

    emit!(GroupActivated {
        group: group.key(),
        total_members: group.total_members,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
