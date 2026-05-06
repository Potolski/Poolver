use anchor_lang::prelude::*;

use crate::constants::*;
use crate::error::ConsolError;
use crate::state::{ConsorcioGroup, GroupStatus, Round, RoundStatus};
use crate::switchboard::RandomnessAccountData;

/// Commit phase: bundled in the same tx as Switchboard's `randomness.commitIx()`.
/// Stores the randomness account reference and commit slot for later verification.
#[derive(Accounts)]
pub struct CommitRound<'info> {
    pub caller: Signer<'info>,

    #[account(
        constraint = group.status == GroupStatus::Active @ ConsolError::InvalidGroupState,
    )]
    pub group: Box<Account<'info, ConsorcioGroup>>,

    #[account(
        mut,
        seeds = [ROUND_SEED, group.key().as_ref(), &[group.current_round]],
        bump = round.bump,
        constraint = round.status == RoundStatus::Selecting @ ConsolError::InvalidRoundState,
    )]
    pub round: Box<Account<'info, Round>>,

    /// The Switchboard randomness account (created off-chain, committed in same tx)
    /// CHECK: Validated manually via RandomnessAccountData::parse
    pub randomness_account_data: UncheckedAccount<'info>,
}

pub fn handle_commit_round(ctx: Context<CommitRound>) -> Result<()> {
    let clock = Clock::get()?;

    // Parse and validate the randomness account
    let randomness_data = RandomnessAccountData::parse(
        ctx.accounts.randomness_account_data.try_borrow_data()?,
    )?;

    // Ensure randomness was committed in this slot (bundled in same tx as commitIx)
    require!(
        randomness_data.seed_slot == clock.slot - 1,
        ConsolError::VrfNotResolved
    );

    // Ensure randomness hasn't been revealed yet
    require!(
        !randomness_data.is_revealed(),
        ConsolError::VrfNotResolved
    );

    // Store commit info on the round
    let round = &mut ctx.accounts.round;
    round.commit_slot = randomness_data.seed_slot;
    round.randomness_account = ctx.accounts.randomness_account_data.key();

    Ok(())
}
