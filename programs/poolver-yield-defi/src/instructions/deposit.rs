use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::constants::{
    BPS_DENOMINATOR, CORE_INVOKER_SEED, DEFI_ADAPTER_KTOKEN_SEED, DEFI_ADAPTER_SEED,
    DEFI_ADAPTER_USDC_SEED, KAMINO_DEPLOYED_BPS,
};
use crate::error::YieldDefiError;
use crate::events::AdapterDeposited;
use crate::state::DefiAdapterState;
use crate::POOLVER_CORE_ID;

// Account layout follows arch §13.2. The leading account positions
// (`core_invoker`, `adapter_state`, `adapter_usdc_vault`, `source_*`)
// MUST be byte-identical to `poolver-yield-vault`'s `AdapterDeposit`
// so core can build a single CPI shape per spec §5.3 / INV-21. The
// trailing `adapter_ktoken_vault` is the Tier-1 addition; core's
// dispatch passes it for Tier 1 pools and skips it for Tier 0.
#[derive(Accounts)]
pub struct AdapterDeposit<'info> {
    #[account(
        seeds = [CORE_INVOKER_SEED],
        seeds::program = POOLVER_CORE_ID,
        bump,
    )]
    pub core_invoker: Signer<'info>,

    #[account(
        mut,
        seeds = [DEFI_ADAPTER_SEED, adapter_state.pool.as_ref()],
        bump = adapter_state.bump,
    )]
    pub adapter_state: Account<'info, DefiAdapterState>,

    #[account(
        mut,
        seeds = [DEFI_ADAPTER_USDC_SEED, adapter_state.pool.as_ref()],
        bump,
        constraint = adapter_usdc_vault.key() == adapter_state.usdc_vault
            @ YieldDefiError::Unauthorized,
    )]
    pub adapter_usdc_vault: Account<'info, TokenAccount>,

    /// Source of funds. Core passes the pool's PoolUsdcVault here.
    /// Same trade as Tier 0: we don't constrain `source_usdc.owner`
    /// because core handles that on its side (arch §5.1).
    #[account(mut)]
    pub source_usdc: Account<'info, TokenAccount>,

    /// Authority over `source_usdc`. In production this is the pool
    /// USDC vault PDA owned by core; passed in raw because Anchor
    /// can't type a foreign-program PDA here.
    /// CHECK: signature presence is enforced by the SPL token program
    /// at CPI time; no further validation needed in this adapter.
    pub source_authority: Signer<'info>,

    pub token_program: Program<'info, Token>,

    /// Tier-1-specific: the kToken vault (mocked as a USDC token
    /// account in V1 — SPEC_QUESTION-19). The 75% deployed leg lands
    /// here via an internal token transfer that simulates the Kamino
    /// supply CPI.
    #[account(
        mut,
        seeds = [DEFI_ADAPTER_KTOKEN_SEED, adapter_state.pool.as_ref()],
        bump,
        constraint = adapter_ktoken_vault.key() == adapter_state.ktoken_vault
            @ YieldDefiError::Unauthorized,
    )]
    pub adapter_ktoken_vault: Account<'info, TokenAccount>,
}

pub fn handle_deposit(ctx: Context<AdapterDeposit>, amount: u64) -> Result<()> {
    require!(amount > 0, YieldDefiError::InvalidAmount);

    // Circuit-breaker gate. Solana atomicity forces the trip latch to
    // be set in a SUCCESSFUL prior instruction (one of the
    // `mock_set_*` helpers in V1; in production the breaker-eval
    // logic itself runs in a successful "monitor" tx that latches
    // state and lets the next deposit/withdraw observe the gate).
    // Here we only check the latch — mutating state inside an
    // erroring ix would be reverted by the runtime anyway.
    let pool_key = ctx.accounts.adapter_state.pool;
    require!(
        !ctx.accounts.adapter_state.tripped,
        YieldDefiError::CircuitBreakerTripped
    );

    // Step 1: pull the FULL amount from the source vault into the
    // liquid (USDC) vault. We then redistribute internally.
    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.key(),
        Transfer {
            from: ctx.accounts.source_usdc.to_account_info(),
            to: ctx.accounts.adapter_usdc_vault.to_account_info(),
            authority: ctx.accounts.source_authority.to_account_info(),
        },
    );
    token::transfer(cpi_ctx, amount)?;

    // Step 2: compute the 75/25 split. Subtraction-based for the liquid
    // share so any BPS rounding error stays on the liquid side (i.e.
    // the part we still control directly), never deployed.
    let to_kamino = amount
        .checked_mul(KAMINO_DEPLOYED_BPS)
        .and_then(|v| v.checked_div(BPS_DENOMINATOR))
        .ok_or(YieldDefiError::MathOverflow)?;
    let to_liquid = amount
        .checked_sub(to_kamino)
        .ok_or(YieldDefiError::MathOverflow)?;

    // Step 3: simulate the Kamino supply by moving `to_kamino` from
    // the liquid vault to the ktoken vault. SPEC_QUESTION-19: in
    // production this is replaced by a `kamino::supply` CPI that mints
    // kTokens into `adapter_ktoken_vault`. The signer seeds for the
    // liquid vault stay the same — its self-authority pattern.
    if to_kamino > 0 {
        let pool_ref = pool_key;
        let usdc_vault_bump = ctx.bumps.adapter_usdc_vault;
        let signer_seeds: &[&[&[u8]]] = &[&[
            DEFI_ADAPTER_USDC_SEED,
            pool_ref.as_ref(),
            &[usdc_vault_bump],
        ]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            Transfer {
                from: ctx.accounts.adapter_usdc_vault.to_account_info(),
                to: ctx.accounts.adapter_ktoken_vault.to_account_info(),
                authority: ctx.accounts.adapter_usdc_vault.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(cpi_ctx, to_kamino)?;
    }

    // Step 4: bookkeeping.
    let state = &mut ctx.accounts.adapter_state;
    state.total_deposited = state
        .total_deposited
        .checked_add(amount)
        .ok_or(YieldDefiError::MathOverflow)?;
    state.total_deployed_to_kamino = state
        .total_deployed_to_kamino
        .checked_add(to_kamino)
        .ok_or(YieldDefiError::MathOverflow)?;
    state.liquid_reserved = state
        .liquid_reserved
        .checked_add(to_liquid)
        .ok_or(YieldDefiError::MathOverflow)?;

    emit!(AdapterDeposited {
        pool: pool_key,
        amount,
        deployed_to_kamino: to_kamino,
        kept_liquid: to_liquid,
        total_deposited: state.total_deposited,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}
