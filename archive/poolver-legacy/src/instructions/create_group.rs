use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::constants::*;
use crate::error::ConsolError;
use crate::events::GroupCreated;
use crate::state::{ConsorcioGroup, GroupStatus};

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CreateGroupParams {
    pub monthly_contribution: u64,
    pub total_members: u8,
    pub collateral_bps: u16,
    pub insurance_bps: u16,
    pub description: String,
    /// Unique seed to allow one creator to have multiple groups
    pub group_id: u64,
}

#[derive(Accounts)]
#[instruction(params: CreateGroupParams)]
pub struct CreateGroup<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        init,
        payer = creator,
        space = 8 + ConsorcioGroup::INIT_SPACE,
        seeds = [GROUP_SEED, creator.key().as_ref(), &params.group_id.to_le_bytes()],
        bump,
    )]
    pub group: Account<'info, ConsorcioGroup>,

    /// The USDC (or other SPL token) mint for this group
    pub mint: Account<'info, Mint>,

    #[account(
        init,
        payer = creator,
        seeds = [VAULT_SEED, group.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = vault,
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = creator,
        seeds = [INSURANCE_SEED, group.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = insurance_vault,
    )]
    pub insurance_vault: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = creator,
        seeds = [TREASURY_SEED, group.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = treasury_vault,
    )]
    pub treasury_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handle_create_group(ctx: Context<CreateGroup>, params: CreateGroupParams) -> Result<()> {
    require!(
        params.total_members >= MIN_GROUP_SIZE && params.total_members <= MAX_GROUP_SIZE,
        ConsolError::InvalidGroupSize
    );
    require!(
        params.monthly_contribution >= MIN_CONTRIBUTION,
        ConsolError::ContributionTooLow
    );
    require!(
        params.collateral_bps > 0 && params.collateral_bps <= 10_000,
        ConsolError::InvalidCollateralBps
    );
    require!(
        params.insurance_bps <= 2_000,
        ConsolError::InvalidInsuranceBps
    );
    require!(
        ctx.accounts.mint.decimals == 6,
        ConsolError::InvalidMintDecimals
    );
    require!(
        params.description.len() <= 64,
        ConsolError::DescriptionTooLong
    );

    let clock = Clock::get()?;
    let group = &mut ctx.accounts.group;

    group.creator = ctx.accounts.creator.key();
    group.mint = ctx.accounts.mint.key();
    group.monthly_contribution = params.monthly_contribution;
    group.total_members = params.total_members;
    group.current_members = 0;
    group.current_round = 0;
    group.status = GroupStatus::Forming;
    group.collateral_bps = params.collateral_bps;
    group.insurance_bps = params.insurance_bps;
    group.protocol_fee_bps = PROTOCOL_FEE_BPS;
    group.created_at = clock.unix_timestamp;
    group.formation_deadline =
        clock.unix_timestamp + (FORMATION_TIMEOUT_DAYS * 24 * 60 * 60);
    group.round_started_at = 0;
    group.members_received = 0;
    group.active_members = 0;
    group.bump = ctx.bumps.group;
    group.vault_bump = ctx.bumps.vault;
    group.insurance_bump = ctx.bumps.insurance_vault;
    group.treasury_bump = ctx.bumps.treasury_vault;
    group.description = params.description;

    emit!(GroupCreated {
        group: group.key(),
        creator: ctx.accounts.creator.key(),
        monthly_contribution: params.monthly_contribution,
        total_members: params.total_members,
        collateral_bps: params.collateral_bps,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
