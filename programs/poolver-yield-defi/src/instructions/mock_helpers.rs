//! Mock-only test/dev instructions. Gated by the `mock-yield` Cargo
//! feature so they vanish from the IDL + binary in
//! `--no-default-features` builds (mainnet shape). See arch §10 +
//! INV-26 for the precedent set by `poolver-core::mock_issue_kyc`.
//!
//! SPEC_QUESTION-19 / Q-20 / Q-23: every site here marks where a real
//! Kamino + Pyth integration replaces the mock plumbing.

#![cfg(feature = "mock-yield")]

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::constants::{
    DEFI_ADAPTER_KTOKEN_SEED, DEFI_ADAPTER_SEED, ORACLE_DEVIATION_TRIP_BPS, TRIP_REASON_ORACLE_DEVIATION,
    TRIP_REASON_PAUSED, TRIP_REASON_UTILIZATION, UTILIZATION_TRIP_BPS,
};
use crate::error::YieldDefiError;
use crate::events::{
    CircuitBreakerTripped, MockKaminoPausedSet, MockOracleDeviationSet, MockUtilizationSet,
    MockYieldInjected,
};
use crate::state::DefiAdapterState;

// ──────────────────────────────────────────────────────────────────────
// mock_inject_yield — the analogue of "Kamino interest accrued"
// ──────────────────────────────────────────────────────────────────────

/// Move USDC from `injector_usdc` directly into `adapter_ktoken_vault`
/// to simulate Kamino interest accruing on the deployed leg. Bumps
/// neither `total_deposited` nor `total_deployed_to_kamino` — the new
/// USDC is "yield", not principal, and `harvest()` discovers it via
/// the `last_recorded_balance` delta (arch §13.1).
///
/// SPEC_QUESTION-19: in production this instruction does NOT exist.
/// Real yield arrives implicitly: Kamino's exchange rate ticks up,
/// `kToken × exchange_rate` grows, and the next `harvest()` reads the
/// delta against `last_recorded_balance`. The mock just shortcuts
/// past the exchange-rate machinery.
///
/// SPEC_QUESTION-26: V1 doesn't gate on a real admin signer (any
/// signer with the right ATA can inject). For the hackathon this is
/// fine — yield "injection" can't drain funds, only inflate the
/// reported yield.
#[derive(Accounts)]
pub struct MockInjectYield<'info> {
    /// SPEC_QUESTION-26: any signer in V1.
    #[account(mut)]
    pub injector: Signer<'info>,

    #[account(
        mut,
        seeds = [DEFI_ADAPTER_SEED, adapter_state.pool.as_ref()],
        bump = adapter_state.bump,
    )]
    pub adapter_state: Account<'info, DefiAdapterState>,

    /// Source of the injected USDC. Must be authority-owned by
    /// `injector` (the SPL transfer enforces it).
    #[account(mut)]
    pub injector_usdc: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [DEFI_ADAPTER_KTOKEN_SEED, adapter_state.pool.as_ref()],
        bump,
        constraint = adapter_ktoken_vault.key() == adapter_state.ktoken_vault
            @ YieldDefiError::Unauthorized,
    )]
    pub adapter_ktoken_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_mock_inject_yield(
    ctx: Context<MockInjectYield>,
    amount: u64,
) -> Result<()> {
    require!(amount > 0, YieldDefiError::InvalidAmount);

    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.key(),
        Transfer {
            from: ctx.accounts.injector_usdc.to_account_info(),
            to: ctx.accounts.adapter_ktoken_vault.to_account_info(),
            authority: ctx.accounts.injector.to_account_info(),
        },
    );
    token::transfer(cpi_ctx, amount)?;

    let new_balance = ctx
        .accounts
        .adapter_ktoken_vault
        .amount
        .checked_add(amount)
        .ok_or(YieldDefiError::MathOverflow)?;

    emit!(MockYieldInjected {
        pool: ctx.accounts.adapter_state.pool,
        amount,
        new_ktoken_balance: new_balance,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

// ──────────────────────────────────────────────────────────────────────
// mock_set_utilization / mock_set_oracle_deviation / mock_set_kamino_paused
//
// These three set the breaker-input fields on `DefiAdapterState`. The
// next `deposit` reads them and trips if any threshold is breached.
// SPEC_QUESTION-19/23: in production the fields don't exist — the
// breaker reads Kamino's reserve utilization + Pyth oracle deviation
// directly on each deposit.
// ──────────────────────────────────────────────────────────────────────

#[derive(Accounts)]
pub struct MockSetBreakerInput<'info> {
    /// SPEC_QUESTION-26: any signer in V1.
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [DEFI_ADAPTER_SEED, adapter_state.pool.as_ref()],
        bump = adapter_state.bump,
    )]
    pub adapter_state: Account<'info, DefiAdapterState>,
}

/// Helper: when a mock setter writes a value that breaches a trip
/// threshold, latch the breaker IN THIS instruction (which succeeds)
/// so subsequent `deposit` / `withdraw` / `harvest` calls see a
/// persistent `tripped == true` state.
///
/// This shape is forced by Solana's atomicity: state writes inside an
/// erroring instruction are reverted, so we cannot trip the breaker
/// from inside `deposit`'s pre-check while also returning an error.
/// The mock setters take that role for us — same end-state from a
/// monitoring perspective (the "next deposit fails" gate). Production
/// will collapse this into a single check inside deposit because
/// production reads the breaker inputs from external state (Kamino +
/// Pyth), so the deposit ix observing a breach IS a successful
/// "circuit-breaker tripped" gate that latches state and then errors
/// on the NEXT call.
fn latch_if_breached(state: &mut DefiAdapterState, now: i64) -> Option<u8> {
    if state.tripped {
        return Some(state.tripped_reason);
    }
    if state.mock_utilization_bps > UTILIZATION_TRIP_BPS {
        state.tripped = true;
        state.tripped_reason = TRIP_REASON_UTILIZATION;
        state.tripped_at = now;
        return Some(TRIP_REASON_UTILIZATION);
    }
    if state.mock_oracle_deviation_bps > ORACLE_DEVIATION_TRIP_BPS {
        state.tripped = true;
        state.tripped_reason = TRIP_REASON_ORACLE_DEVIATION;
        state.tripped_at = now;
        return Some(TRIP_REASON_ORACLE_DEVIATION);
    }
    if state.mock_kamino_paused {
        state.tripped = true;
        state.tripped_reason = TRIP_REASON_PAUSED;
        state.tripped_at = now;
        return Some(TRIP_REASON_PAUSED);
    }
    None
}

pub fn handle_mock_set_utilization(
    ctx: Context<MockSetBreakerInput>,
    bps: u16,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let pool_key = ctx.accounts.adapter_state.pool;
    let state = &mut ctx.accounts.adapter_state;
    state.mock_utilization_bps = bps;
    let trip = latch_if_breached(state, now);

    emit!(MockUtilizationSet {
        pool: pool_key,
        bps,
        timestamp: now,
    });
    if let Some(reason) = trip {
        emit!(CircuitBreakerTripped {
            pool: pool_key,
            reason,
            timestamp: now,
        });
    }
    Ok(())
}

pub fn handle_mock_set_oracle_deviation(
    ctx: Context<MockSetBreakerInput>,
    bps: u16,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let pool_key = ctx.accounts.adapter_state.pool;
    let state = &mut ctx.accounts.adapter_state;
    state.mock_oracle_deviation_bps = bps;
    let trip = latch_if_breached(state, now);

    emit!(MockOracleDeviationSet {
        pool: pool_key,
        bps,
        timestamp: now,
    });
    if let Some(reason) = trip {
        emit!(CircuitBreakerTripped {
            pool: pool_key,
            reason,
            timestamp: now,
        });
    }
    Ok(())
}

pub fn handle_mock_set_kamino_paused(
    ctx: Context<MockSetBreakerInput>,
    paused: bool,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let pool_key = ctx.accounts.adapter_state.pool;
    let state = &mut ctx.accounts.adapter_state;
    state.mock_kamino_paused = paused;
    let trip = latch_if_breached(state, now);

    emit!(MockKaminoPausedSet {
        pool: pool_key,
        paused,
        timestamp: now,
    });
    if let Some(reason) = trip {
        emit!(CircuitBreakerTripped {
            pool: pool_key,
            reason,
            timestamp: now,
        });
    }
    Ok(())
}
