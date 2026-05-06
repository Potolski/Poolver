use anchor_lang::prelude::*;

use crate::constants::*;
use crate::error::ConsolError;
use crate::events::WinnerSelected;
use crate::state::{ConsorcioGroup, GroupStatus, Member, MemberStatus, Round, RoundStatus};
use crate::switchboard::RandomnessAccountData;

/// Reveal phase: bundled in the same tx as Switchboard's `randomness.revealIx()`.
/// Reads the revealed random value, selects a winner from eligible members.
///
/// Eligible members are passed as remaining_accounts. The instruction verifies
/// each is a valid Member PDA, is Active, and hasn't received yet.
#[derive(Accounts)]
pub struct ResolveRound<'info> {
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
        constraint = round.status == RoundStatus::Selecting @ ConsolError::InvalidRoundState,
        constraint = round.commit_slot > 0 @ ConsolError::VrfNotResolved,
    )]
    pub round: Box<Account<'info, Round>>,

    /// The same Switchboard randomness account that was committed
    /// CHECK: Validated via key match + RandomnessAccountData::parse
    #[account(
        constraint = randomness_account_data.key() == round.randomness_account @ ConsolError::VrfNotResolved,
    )]
    pub randomness_account_data: UncheckedAccount<'info>,
    // remaining_accounts: eligible Member accounts (Active, !has_received)
}

pub fn handle_resolve_round(ctx: Context<ResolveRound>) -> Result<()> {
    let clock = Clock::get()?;

    // Parse randomness and get the revealed value
    let randomness_data = RandomnessAccountData::parse(
        ctx.accounts.randomness_account_data.try_borrow_data()?,
    )?;

    // Verify the seed_slot matches what we committed
    require!(
        randomness_data.seed_slot == ctx.accounts.round.commit_slot,
        ConsolError::VrfNotResolved
    );

    // Get the 32-byte random value (fails if not yet revealed by oracle)
    let random_value = randomness_data.get_value(clock.slot)?;

    // Collect eligible members from remaining_accounts
    let group_key = ctx.accounts.group.key();
    let mut eligible_wallets: Vec<Pubkey> = Vec::new();

    for account_info in ctx.remaining_accounts.iter() {
        // Deserialize as Member account
        let member_data = Account::<Member>::try_from(account_info)
            .map_err(|_| ConsolError::NotMember)?;

        // Verify it belongs to this group
        require!(member_data.group == group_key, ConsolError::NotMember);

        // Verify PDA
        let (expected_pda, _) = Pubkey::find_program_address(
            &[MEMBER_SEED, group_key.as_ref(), member_data.wallet.as_ref()],
            ctx.program_id,
        );
        require!(
            account_info.key() == expected_pda,
            ConsolError::NotMember
        );

        // Must be active and not yet received
        if member_data.status == MemberStatus::Active && !member_data.has_received {
            eligible_wallets.push(member_data.wallet);
        }
    }

    require!(!eligible_wallets.is_empty(), ConsolError::NoEligibleMembers);

    // Verify ALL eligible members were provided — prevents callers from omitting
    // members to manipulate the lottery outcome.
    let expected_eligible = (ctx.accounts.group.active_members as u16)
        .checked_sub(ctx.accounts.group.members_received as u16)
        .ok_or(ConsolError::MathOverflow)? as usize;
    require!(
        eligible_wallets.len() == expected_eligible,
        ConsolError::NoEligibleMembers
    );

    // Use randomness to select winner: first 8 bytes as u64, mod eligible count
    let random_u64 = u64::from_le_bytes(random_value[0..8].try_into().unwrap());
    let winner_index = (random_u64 % eligible_wallets.len() as u64) as usize;
    let winner_wallet = eligible_wallets[winner_index];

    // Update round
    let round = &mut ctx.accounts.round;
    round.lottery_winner = winner_wallet;
    round.winner_selected = true;
    round.vrf_result = random_value;
    round.status = RoundStatus::Distributing;

    emit!(WinnerSelected {
        group: group_key,
        round: round.round_number,
        winner: winner_wallet,
        amount: round.total_collected,
        vrf_proof: random_value,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
