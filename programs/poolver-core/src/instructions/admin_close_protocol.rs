use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Token};

use crate::constants::{PROTOCOL_CONFIG_SEED, PROTOCOL_FEE_VAULT_SEED};
use crate::error::CoreError;
use crate::events::ProtocolClosed;
use crate::state::ProtocolConfig;

// SPEC_QUESTION-26: admin-only escape hatch to close `ProtocolConfig` and
// `protocol_fee_vault` so the next `initialize_protocol` call can install a
// fresh USDC mint binding. Used post-deploy when the baked-in mint turns out
// to be unmintable / wrong (devnet fix-up). V2 multi-sig will gate this
// instruction; V1 trusts the single admin pubkey (already invariant for the
// rest of the admin surface).
//
// The protocol is required to be empty at the call site: the
// `protocol_fee_vault` close CPI fails if the vault still holds any tokens,
// which is the structural guarantee that no protocol fees are silently
// destroyed by this rotation. Likewise the `ProtocolConfig` close just
// recovers rent and zeroes the discriminator — every dependent account
// (Pool, Participant, KYC, Reputation) becomes orphaned but unmodified, so
// they remain readable on-chain for indexer history.
#[derive(Accounts)]
pub struct AdminCloseProtocol<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        close = admin,
        seeds = [PROTOCOL_CONFIG_SEED],
        bump = protocol_config.bump,
        has_one = admin @ CoreError::Unauthorized,
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,

    /// CHECK: closed via SPL Token CPI in handler. PDA-derived; self-authority.
    /// We don't deserialize as `TokenAccount` because the closed-state of the
    /// account post-CPI would prevent Anchor's account-info drop check from
    /// matching. The `seeds + bump` constraint is the only validation needed
    /// — any account at this PDA is, by construction, the protocol fee vault.
    #[account(
        mut,
        seeds = [PROTOCOL_FEE_VAULT_SEED],
        bump,
    )]
    pub protocol_fee_vault: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_admin_close_protocol(ctx: Context<AdminCloseProtocol>) -> Result<()> {
    // Sign with the protocol_fee_vault PDA itself — it is its own authority
    // (token::authority = protocol_fee_vault in initialize_protocol).
    let bump = ctx.bumps.protocol_fee_vault;
    let signer_seeds: &[&[&[u8]]] = &[&[PROTOCOL_FEE_VAULT_SEED, &[bump]]];

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        CloseAccount {
            account: ctx.accounts.protocol_fee_vault.to_account_info(),
            destination: ctx.accounts.admin.to_account_info(),
            authority: ctx.accounts.protocol_fee_vault.to_account_info(),
        },
        signer_seeds,
    );
    token::close_account(cpi_ctx)?;

    emit!(ProtocolClosed {
        admin: ctx.accounts.admin.key(),
        protocol_config: ctx.accounts.protocol_config.key(),
        protocol_fee_vault: ctx.accounts.protocol_fee_vault.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });

    // The `ProtocolConfig` PDA itself is closed via the `close = admin`
    // constraint at the end of the instruction (Anchor handles the data
    // zeroing + lamport refund automatically).
    Ok(())
}
