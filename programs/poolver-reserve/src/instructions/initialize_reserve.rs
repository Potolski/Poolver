use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::constants::{RESERVE_FUND_SEED, RESERVE_VAULT_SEED};
use crate::events::ReserveInitialized;
use crate::state::{ReserveFund, Tier};

// `initialize_reserve` is admin-only and runs ONCE GLOBALLY per tier at
// deployment. Unlike the per-pool adapter inits, this is NOT a CPI from
// core — protocol admin calls it directly during bootstrap (spec §5.2).
//
// SPEC_QUESTION-26: until `poolver-core::ProtocolConfig` lands, we accept
// any signer here. When core ships, tighten the constraint to require the
// admin pubkey from `ProtocolConfig`. The single-init guarantee (Anchor's
// `init` + tier-encoded seed) means a hostile early caller cannot steal
// the slot from a legitimate admin without admin first being asleep at
// the wheel — initialisation order is a deployment concern, not a runtime
// one.
#[derive(Accounts)]
#[instruction(tier: Tier)]
pub struct InitializeReserve<'info> {
    /// Pays for both the state account and the token-account rent.
    /// SPEC_QUESTION-26: this signer becomes "must equal ProtocolConfig.admin"
    /// once core lands.
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + ReserveFund::INIT_SPACE,
        // INV-4: the tier byte IS the seed. Anchor refuses to derive a
        // second account with the same tier, so re-init is structurally
        // impossible.
        seeds = [RESERVE_FUND_SEED, &[tier.as_u8()]],
        bump,
    )]
    pub reserve_fund: Account<'info, ReserveFund>,

    /// USDC mint. The constraint that this is in fact the canonical USDC
    /// mint will land with `poolver-core::ProtocolConfig` — until then we
    /// accept whatever mint admin passes (the tests rely on this for the
    /// fake-USDC fixture).
    /// SPEC_QUESTION-26: validate against `ProtocolConfig.usdc_mint` once
    /// core lands.
    pub usdc_mint: Account<'info, Mint>,

    /// PDA-owned USDC vault. Authority = the token account itself; its own
    /// seeds sign for it during `draw` (matches the yield-vault pattern).
    #[account(
        init,
        payer = admin,
        seeds = [RESERVE_VAULT_SEED, &[tier.as_u8()]],
        bump,
        token::mint = usdc_mint,
        token::authority = reserve_usdc_vault,
    )]
    pub reserve_usdc_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_initialize_reserve(ctx: Context<InitializeReserve>, tier: Tier) -> Result<()> {
    let fund = &mut ctx.accounts.reserve_fund;
    fund.tier = tier;
    fund.total_balance = 0;
    fund.total_inflows = 0;
    fund.total_outflows = 0;
    fund.usdc_vault = ctx.accounts.reserve_usdc_vault.key();
    fund.bump = ctx.bumps.reserve_fund;
    fund._reserved = [0u8; 32];

    emit!(ReserveInitialized {
        tier,
        reserve_fund: fund.key(),
        usdc_vault: fund.usdc_vault,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}
