//! Tier-aware adapter CPI helpers (SPEC_QUESTION-36).
//!
//! Each public helper here matches one of the four adapter-touching ix
//! verbs (`initialize`, `deposit`, `withdraw`, `harvest`) and dispatches
//! to the correct adapter program (`poolver-yield-vault` for
//! [`Tier::Vault`], `poolver-yield-defi` for [`Tier::DeFi`]) based on
//! the pool's tier.
//!
//! ## `remaining_accounts` ordering (for client SDK writers)
//!
//! All four ix share the leading prefix that arch §13.2 promises is
//! byte-identical between adapters:
//!
//! ```text
//!   [core_invoker, adapter_state, adapter_usdc_vault, ...]
//! ```
//!
//! These leading accounts are passed as fixed account-context fields in
//! `create_pool` / `contribute` / `claim_winning` / `distribute_yield`.
//! The Tier-1 surplus is appended via `remaining_accounts`:
//!
//! | Verb         | Tier 0 (Vault) | Tier 1 (DeFi) — after fixed ctx        |
//! |--------------|----------------|----------------------------------------|
//! | `initialize` | (empty)        | `[adapter_ktoken_vault]`               |
//! | `deposit`    | (empty)        | `[adapter_ktoken_vault]`               |
//! | `withdraw`   | (empty)        | `[adapter_ktoken_vault]`               |
//! | `harvest`    | (empty)        | `[adapter_ktoken_vault]`               |
//!
//! Tier 0 callers may pass a 0-length `remaining_accounts`. Tier 1
//! callers MUST pass exactly the listed extras, in order. The helpers
//! below `require!` that the slot count matches; mismatches surface as
//! [`CoreError::Unauthorized`] rather than a deeper Anchor/SPL error.
//!
//! ## Adapter-program validation
//!
//! Each helper asserts the supplied `yield_adapter_program` AccountInfo
//! key matches the canonical adapter ID for the pool's tier. The fixed
//! Anchor `address = poolver_yield_vault::ID` constraint that pre-step-13
//! handlers carried has been dropped from the contexts so Tier 1 calls
//! can pass `poolver_yield_defi::ID`; the per-tier check below is the
//! replacement structural enforcement.

use anchor_lang::prelude::*;

use crate::constants::CORE_INVOKER_SEED;
use crate::error::CoreError;
use crate::state::Tier;

// ─────────────────────────────────────────────────────────────────────
// Adapter program-ID validation
// ─────────────────────────────────────────────────────────────────────

/// Validate the supplied adapter program AccountInfo matches the
/// canonical program ID for the given tier.
#[inline]
pub fn require_adapter_program_for_tier(
    yield_adapter_program: &AccountInfo,
    tier: Tier,
) -> Result<()> {
    match tier {
        Tier::Vault => require_keys_eq!(
            yield_adapter_program.key(),
            poolver_yield_vault::ID,
            CoreError::Unauthorized
        ),
        Tier::DeFi => require_keys_eq!(
            yield_adapter_program.key(),
            poolver_yield_defi::ID,
            CoreError::Unauthorized
        ),
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────
// `initialize_adapter` dispatch — called from `create_pool`
// ─────────────────────────────────────────────────────────────────────

/// Dispatch `initialize_adapter` to the right tier. Tier 1 reads the
/// extra `adapter_ktoken_vault` from `remaining_accounts[0]`.
#[allow(clippy::too_many_arguments)]
pub fn cpi_adapter_initialize<'info>(
    tier: Tier,
    yield_adapter_program: AccountInfo<'info>,
    core_invoker: AccountInfo<'info>,
    payer: AccountInfo<'info>,
    adapter_state: AccountInfo<'info>,
    usdc_mint: AccountInfo<'info>,
    adapter_usdc_vault: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    system_program: AccountInfo<'info>,
    rent: AccountInfo<'info>,
    remaining_accounts: &[AccountInfo<'info>],
    pool_key: Pubkey,
    core_invoker_bump: u8,
) -> Result<()> {
    require_adapter_program_for_tier(&yield_adapter_program, tier)?;

    let signer_seeds: &[&[&[u8]]] = &[&[CORE_INVOKER_SEED, &[core_invoker_bump]]];
    let program_id = yield_adapter_program.key();

    match tier {
        Tier::Vault => {
            // Tier 0: no remaining_accounts expected.
            require!(remaining_accounts.is_empty(), CoreError::Unauthorized);
            let cpi_accounts = poolver_yield_vault::cpi::accounts::InitializeAdapter {
                core_invoker,
                payer,
                adapter_state,
                usdc_mint,
                adapter_usdc_vault,
                token_program,
                system_program,
                rent,
            };
            let cpi_ctx = CpiContext::new_with_signer(program_id, cpi_accounts, signer_seeds);
            poolver_yield_vault::cpi::initialize_adapter(cpi_ctx, pool_key)
        }
        Tier::DeFi => {
            // Tier 1: remaining_accounts[0] = adapter_ktoken_vault.
            require!(remaining_accounts.len() == 1, CoreError::Unauthorized);
            let adapter_ktoken_vault = remaining_accounts[0].clone();
            let cpi_accounts = poolver_yield_defi::cpi::accounts::InitializeAdapter {
                core_invoker,
                payer,
                adapter_state,
                usdc_mint,
                adapter_usdc_vault,
                adapter_ktoken_vault,
                token_program,
                system_program,
                rent,
            };
            let cpi_ctx = CpiContext::new_with_signer(program_id, cpi_accounts, signer_seeds);
            poolver_yield_defi::cpi::initialize_adapter(cpi_ctx, pool_key)
        }
    }
}

// ─────────────────────────────────────────────────────────────────────
// `deposit` dispatch — called from `contribute`
// ─────────────────────────────────────────────────────────────────────

/// Dispatch `deposit(amount)` to the right tier. The `source_usdc` and
/// `source_authority` are core's `pool_usdc_vault` (signed by its own
/// seeds + `core_invoker`).
#[allow(clippy::too_many_arguments)]
pub fn cpi_adapter_deposit<'info>(
    tier: Tier,
    yield_adapter_program: AccountInfo<'info>,
    core_invoker: AccountInfo<'info>,
    adapter_state: AccountInfo<'info>,
    adapter_usdc_vault: AccountInfo<'info>,
    pool_usdc_vault: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    remaining_accounts: &[AccountInfo<'info>],
    combined_seeds: &[&[&[u8]]],
    amount: u64,
) -> Result<()> {
    require_adapter_program_for_tier(&yield_adapter_program, tier)?;
    let program_id = yield_adapter_program.key();

    match tier {
        Tier::Vault => {
            require!(remaining_accounts.is_empty(), CoreError::Unauthorized);
            let cpi_accounts = poolver_yield_vault::cpi::accounts::AdapterDeposit {
                core_invoker,
                adapter_state,
                adapter_usdc_vault,
                source_usdc: pool_usdc_vault.clone(),
                source_authority: pool_usdc_vault,
                token_program,
            };
            let cpi_ctx = CpiContext::new_with_signer(program_id, cpi_accounts, combined_seeds);
            poolver_yield_vault::cpi::deposit(cpi_ctx, amount)
        }
        Tier::DeFi => {
            require!(remaining_accounts.len() == 1, CoreError::Unauthorized);
            let adapter_ktoken_vault = remaining_accounts[0].clone();
            let cpi_accounts = poolver_yield_defi::cpi::accounts::AdapterDeposit {
                core_invoker,
                adapter_state,
                adapter_usdc_vault,
                source_usdc: pool_usdc_vault.clone(),
                source_authority: pool_usdc_vault,
                token_program,
                adapter_ktoken_vault,
            };
            let cpi_ctx = CpiContext::new_with_signer(program_id, cpi_accounts, combined_seeds);
            poolver_yield_defi::cpi::deposit(cpi_ctx, amount)
        }
    }
}

// ─────────────────────────────────────────────────────────────────────
// `withdraw` dispatch — called from `claim_winning` and `distribute_yield`
// ─────────────────────────────────────────────────────────────────────

/// Dispatch `withdraw(amount)` to the right tier. Core signs as
/// `core_invoker`. `destination_usdc` is supplied by the caller (winner's
/// ATA in claim_winning, pool_usdc_vault in distribute_yield).
#[allow(clippy::too_many_arguments)]
pub fn cpi_adapter_withdraw<'info>(
    tier: Tier,
    yield_adapter_program: AccountInfo<'info>,
    core_invoker: AccountInfo<'info>,
    adapter_state: AccountInfo<'info>,
    adapter_usdc_vault: AccountInfo<'info>,
    destination_usdc: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    remaining_accounts: &[AccountInfo<'info>],
    core_invoker_bump: u8,
    amount: u64,
) -> Result<()> {
    require_adapter_program_for_tier(&yield_adapter_program, tier)?;

    let signer_seeds: &[&[&[u8]]] = &[&[CORE_INVOKER_SEED, &[core_invoker_bump]]];
    let program_id = yield_adapter_program.key();

    match tier {
        Tier::Vault => {
            require!(remaining_accounts.is_empty(), CoreError::Unauthorized);
            let cpi_accounts = poolver_yield_vault::cpi::accounts::AdapterWithdraw {
                core_invoker,
                adapter_state,
                adapter_usdc_vault,
                destination_usdc,
                token_program,
            };
            let cpi_ctx = CpiContext::new_with_signer(program_id, cpi_accounts, signer_seeds);
            poolver_yield_vault::cpi::withdraw(cpi_ctx, amount)
        }
        Tier::DeFi => {
            require!(remaining_accounts.len() == 1, CoreError::Unauthorized);
            let adapter_ktoken_vault = remaining_accounts[0].clone();
            let cpi_accounts = poolver_yield_defi::cpi::accounts::AdapterWithdraw {
                core_invoker,
                adapter_state,
                adapter_usdc_vault,
                destination_usdc,
                token_program,
                adapter_ktoken_vault,
            };
            let cpi_ctx = CpiContext::new_with_signer(program_id, cpi_accounts, signer_seeds);
            poolver_yield_defi::cpi::withdraw(cpi_ctx, amount)
        }
    }
}

// ─────────────────────────────────────────────────────────────────────
// `harvest` dispatch — called from `distribute_yield`
// ─────────────────────────────────────────────────────────────────────

/// Dispatch `harvest()` to the right tier. The `Result<u64>` return
/// value flows back via Anchor's `set_return_data` plumbing — caller
/// reads it with `get_return_data()`.
#[allow(clippy::too_many_arguments)]
pub fn cpi_adapter_harvest<'info>(
    tier: Tier,
    yield_adapter_program: AccountInfo<'info>,
    core_invoker: AccountInfo<'info>,
    adapter_state: AccountInfo<'info>,
    adapter_usdc_vault: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    remaining_accounts: &[AccountInfo<'info>],
    core_invoker_bump: u8,
) -> Result<()> {
    require_adapter_program_for_tier(&yield_adapter_program, tier)?;

    let signer_seeds: &[&[&[u8]]] = &[&[CORE_INVOKER_SEED, &[core_invoker_bump]]];
    let program_id = yield_adapter_program.key();

    match tier {
        Tier::Vault => {
            require!(remaining_accounts.is_empty(), CoreError::Unauthorized);
            let cpi_accounts = poolver_yield_vault::cpi::accounts::AdapterHarvest {
                core_invoker,
                adapter_state,
                adapter_usdc_vault,
                token_program,
            };
            let cpi_ctx = CpiContext::new_with_signer(program_id, cpi_accounts, signer_seeds);
            let _: poolver_yield_vault::cpi::Return<u64> =
                poolver_yield_vault::cpi::harvest(cpi_ctx)?;
            Ok(())
        }
        Tier::DeFi => {
            require!(remaining_accounts.len() == 1, CoreError::Unauthorized);
            let adapter_ktoken_vault = remaining_accounts[0].clone();
            let cpi_accounts = poolver_yield_defi::cpi::accounts::AdapterHarvest {
                core_invoker,
                adapter_state,
                adapter_usdc_vault,
                adapter_ktoken_vault,
                token_program,
            };
            let cpi_ctx = CpiContext::new_with_signer(program_id, cpi_accounts, signer_seeds);
            let _: poolver_yield_defi::cpi::Return<u64> =
                poolver_yield_defi::cpi::harvest(cpi_ctx)?;
            Ok(())
        }
    }
}
