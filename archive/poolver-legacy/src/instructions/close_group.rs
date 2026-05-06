use anchor_lang::prelude::*;

use crate::constants::*;
use crate::error::ConsolError;
use crate::events::GroupCompleted;
use crate::state::{ConsorcioGroup, GroupStatus};

/// Finalize a group. Permissionless crank. Valid when:
/// 1. All members have received the pool (normal completion)
/// 2. Active members dropped to 0 (everyone defaulted/withdrew)
/// 3. Active members fell below MIN_GROUP_SIZE during active phase (dissolution)
/// 4. Formation timed out without filling all slots
///
/// After closing, members can call `return_collateral` and `distribute_insurance`.
#[derive(Accounts)]
pub struct CloseGroup<'info> {
    pub caller: Signer<'info>,

    #[account(
        mut,
        constraint = group.status == GroupStatus::Active
            || group.status == GroupStatus::Forming
            || group.status == GroupStatus::Completed
            @ ConsolError::InvalidGroupState,
    )]
    pub group: Account<'info, ConsorcioGroup>,
}

pub fn handle_close_group(ctx: Context<CloseGroup>) -> Result<()> {
    let clock = Clock::get()?;
    let group = &mut ctx.accounts.group;

    match group.status {
        GroupStatus::Forming => {
            // Formation timeout — anyone can cancel after deadline
            require!(
                clock.unix_timestamp > group.formation_deadline,
                ConsolError::InvalidGroupState
            );
            group.status = GroupStatus::Cancelled;
        }
        GroupStatus::Active => {
            // Normal completion: all members received
            // Dissolution: no active members left, or fell below minimum viable
            let normal_completion = group.members_received >= group.total_members;
            let no_members = group.active_members == 0;
            let below_minimum = group.active_members < MIN_GROUP_SIZE
                && group.current_round > 0; // only after at least 1 round started

            require!(
                normal_completion || no_members || below_minimum,
                ConsolError::InvalidGroupState
            );

            group.status = GroupStatus::Completed;
        }
        GroupStatus::Completed => {
            // Already completed — no-op, just return ok
            return Ok(());
        }
        GroupStatus::Cancelled => {
            return Ok(());
        }
    }

    emit!(GroupCompleted {
        group: group.key(),
        total_rounds: group.current_round,
        insurance_surplus: 0,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
