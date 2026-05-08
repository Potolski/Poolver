// Refund a participant's locked collateral after pool completion.
// Permissionless — caller can be any signer; the refund always lands
// at the participant's USDC ATA.
//
// Gates:
//   - pool.is_complete == true
//   - participant.is_defaulted == false  (defaulters' collateral is
//     slashed in liquidate_default)
//   - participant.collateral_locked > 0  (idempotent — second call
//     after refund finds nothing to refund and errors)
//
// Token movement: collateral_vault → user_usdc, amount =
// participant.collateral_locked. Sets the field to 0 + decrements
// pool.total_collateral_locked.
//
// Use case: returns the join-collateral that every participant escrowed
// at join_pool, plus any post-win collateral the winner had left over.

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::constants::{COLLATERAL_VAULT_SEED, PARTICIPANT_SEED, POOL_SEED};
use crate::error::CoreError;
use crate::events::CollateralRefunded;
use crate::state::{Participant, Pool};

#[derive(Accounts)]
pub struct RefundCollateral<'info> {
    /// Permissionless caller — pays the tx fee. The refund still goes
    /// to `participant.user`'s ATA, not the caller.
    pub caller: Signer<'info>,

    #[account(
        mut,
        seeds = [POOL_SEED, pool.creator.as_ref(), &pool.pool_id.to_le_bytes()],
        bump = pool.bump,
    )]
    pub pool: Box<Account<'info, Pool>>,

    #[account(
        mut,
        seeds = [PARTICIPANT_SEED, pool.key().as_ref(), participant.user.as_ref()],
        bump = participant.bump,
    )]
    pub participant: Account<'info, Participant>,

    #[account(
        mut,
        constraint = participant_usdc.owner == participant.user
            @ CoreError::Unauthorized,
    )]
    pub participant_usdc: Account<'info, TokenAccount>,

    /// CHECK: PDA-bound; constraint pins to `pool.collateral_vault`.
    #[account(
        mut,
        seeds = [COLLATERAL_VAULT_SEED, pool.key().as_ref()],
        bump,
        constraint = collateral_vault.key() == pool.collateral_vault
            @ CoreError::Unauthorized,
    )]
    pub collateral_vault: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_refund_collateral(ctx: Context<RefundCollateral>) -> Result<()> {
    require!(ctx.accounts.pool.is_complete, CoreError::PoolNotStarted);
    require!(
        !ctx.accounts.participant.is_defaulted,
        CoreError::Defaulted
    );
    let amount = ctx.accounts.participant.collateral_locked;
    require!(amount > 0, CoreError::InvalidAmount);

    let pool_key = ctx.accounts.pool.key();
    let collateral_bump = ctx.bumps.collateral_vault;
    let seeds: &[&[u8]] = &[
        COLLATERAL_VAULT_SEED,
        pool_key.as_ref(),
        &[collateral_bump],
    ];
    let signer_seeds = &[seeds];

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        Transfer {
            from: ctx.accounts.collateral_vault.to_account_info(),
            to: ctx.accounts.participant_usdc.to_account_info(),
            authority: ctx.accounts.collateral_vault.to_account_info(),
        },
        signer_seeds,
    );
    token::transfer(cpi_ctx, amount)?;

    ctx.accounts.participant.collateral_locked = 0;
    let pool = &mut ctx.accounts.pool;
    pool.total_collateral_locked = pool
        .total_collateral_locked
        .saturating_sub(amount);

    emit!(CollateralRefunded {
        pool: pool_key,
        participant: ctx.accounts.participant.user,
        amount,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}
