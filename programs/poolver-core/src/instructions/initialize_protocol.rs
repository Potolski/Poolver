use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::constants::{
    DEFAULT_DEFI_RESERVE_FEE_BPS, DEFAULT_PROTOCOL_FEE_BPS, DEFAULT_VAULT_RESERVE_FEE_BPS,
    PROTOCOL_CONFIG_SEED, PROTOCOL_FEE_VAULT_SEED,
};
use crate::events::ProtocolInitialized;
use crate::state::ProtocolConfig;

/// One-shot. Caller becomes admin and (V1 placeholder) the kyc_oracle.
/// Creates the singleton `ProtocolConfig` PDA and the protocol fee vault
/// SPL token account whose authority is the `protocol_fee_vault` PDA.
///
/// `// MOCK_KYC:` V1 sets `kyc_oracle = admin`. Production must rotate
/// to a dedicated oracle keypair (HSM-backed Idwall integration). See
/// `docs/mock-to-production.md` Site 3.
#[derive(Accounts)]
pub struct InitializeProtocol<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + ProtocolConfig::INIT_SPACE,
        seeds = [PROTOCOL_CONFIG_SEED],
        bump,
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,

    pub usdc_mint: Account<'info, Mint>,

    /// PDA-owned USDC token account that receives every protocol fee.
    /// The token-account itself is its own authority (its seeds sign
    /// for any future protocol-fee withdrawal — those instructions are
    /// not in scope for step 4).
    #[account(
        init,
        payer = admin,
        seeds = [PROTOCOL_FEE_VAULT_SEED],
        bump,
        token::mint = usdc_mint,
        token::authority = protocol_fee_vault,
    )]
    pub protocol_fee_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_initialize_protocol(ctx: Context<InitializeProtocol>) -> Result<()> {
    let cfg = &mut ctx.accounts.protocol_config;
    cfg.admin = ctx.accounts.admin.key();
    // MOCK_KYC: V1 sets kyc_oracle = admin. Production must set to a
    // dedicated oracle keypair. SPEC_QUESTION-26.
    cfg.kyc_oracle = ctx.accounts.admin.key();
    cfg.protocol_fee_vault = ctx.accounts.protocol_fee_vault.key();
    cfg.usdc_mint = ctx.accounts.usdc_mint.key();
    cfg.protocol_fee_bps = DEFAULT_PROTOCOL_FEE_BPS;
    cfg.vault_reserve_fee_bps = DEFAULT_VAULT_RESERVE_FEE_BPS;
    cfg.defi_reserve_fee_bps = DEFAULT_DEFI_RESERVE_FEE_BPS;
    cfg.paused = false;
    cfg.bump = ctx.bumps.protocol_config;
    cfg.version = 1;

    emit!(ProtocolInitialized {
        admin: cfg.admin,
        kyc_oracle: cfg.kyc_oracle,
        usdc_mint: cfg.usdc_mint,
        protocol_fee_vault: cfg.protocol_fee_vault,
        protocol_fee_bps: cfg.protocol_fee_bps,
        vault_reserve_fee_bps: cfg.vault_reserve_fee_bps,
        defi_reserve_fee_bps: cfg.defi_reserve_fee_bps,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}
