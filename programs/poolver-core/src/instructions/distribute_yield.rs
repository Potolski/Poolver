//! `distribute_yield` — spec §5.1, step 9.
//!
//! ## Architectural shape (spec §5.1 + arch §5.1 CPI matrix)
//!
//! Permissionless instruction (anyone can call; in production a keeper
//! bot drives this on a schedule) that:
//!
//!   1. CPIs the pool's yield adapter `harvest()` to crystallize accrued
//!      yield. Returns `u64` USDC via `set_return_data`/`get_return_data`
//!      (arch §13). For Tier 0 (Vault) this always returns 0 (spec §5.3 —
//!      a vault generates no yield by definition).
//!   2. Splits the realized yield 70/20/10 per spec §4:
//!        - 70% → participants (virtual credit via `pool.bid_credit_balance`)
//!        - 20% → tier reserve (real CPI deposit)
//!        - 10% → protocol fee vault (real token transfer)
//!   3. Bumps `pool.total_yield_distributed` (monotonic non-decreasing).
//!   4. Emits `YieldHarvested` + `YieldDistributed` (one summary event
//!      per Q-17, never per-participant).
//!
//! ## V1 Tier 0 reality
//!
//! Tier 0 `harvest()` returns 0 unconditionally. So in V1 this ix is
//! effectively a no-op for every pool: it emits the two events with
//! zeroes, leaves all balances unchanged, and returns `Ok(())`. Step 12
//! relaxes the tier check and adds the Tier 1 dispatch branch.
//!
//! ## Step-12 evolution (do NOT modify until then)
//!
//! When `poolver-yield-defi` ships:
//!   - Drop the `require!(matches!(pool.tier, Tier::Vault), ...)` check
//!     in this file.
//!   - Add `poolver_yield_defi_program: UncheckedAccount<'info>` to the
//!     account context with `address = poolver_yield_defi::ID`.
//!   - Branch the `harvest` and `withdraw` CPIs on `pool.tier`.
//!   - The math, event shapes, ledger updates, and reserve-isolation
//!     check below all stay identical — only the CPI target switches.
//!
//! ## Token-flow contract (INV-1 / INV "Yield monotonic")
//!
//! When `yield_amount > 0` (Tier 1 only — V1 Tier 0 never reaches here):
//!
//!   a) `yield_vault::withdraw(yield_amount)` → `pool_usdc_vault`. The
//!      adapter's USDC vault is the source-of-truth balance (arch §3.8 /
//!      INV-21); we drain the entire harvested amount into the pool's
//!      USDC vault before splitting.
//!   b) `pool_usdc_vault → protocol_fee_vault`: `protocol_share` (10%).
//!      Pool USDC vault PDA signs.
//!   c) `reserve::deposit(reserve_share)` source = `pool_usdc_vault` (20%).
//!      Signed by `core_invoker` + `pool_usdc_vault` PDA.
//!   d) Virtual: `pool.bid_credit_balance += participant_share` (70%).
//!      Tokens stay in `pool_usdc_vault` and discount future `contribute`
//!      calls via the Q-1 pro-rata formula. NO token movement.
//!
//! ### Solvency proof (INV-1)
//!
//! Yield is NEW value entering the pool from outside (Kamino interest).
//! Net delta across all USDC custody endpoints in V1 Tier 0:
//!
//! ```text
//!   Tier 0 (yield_amount = 0): all deltas = 0. No-op.
//! ```
//!
//! For Tier 1 (step 12), `yield_amount` lands in the adapter's USDC
//! vault BEFORE the call (a real Kamino interest accrual). The
//! end-to-end delta inside this ix is:
//!
//! ```text
//!   Δadapter_usdc_vault    = −yield_amount             (a)
//!   Δpool_usdc_vault       = +yield_amount             (a)
//!                           − protocol_share           (b)
//!                           − reserve_share            (c)
//!                          = +participant_share
//!   Δprotocol_fee_vault    = +protocol_share           (b)
//!   Δreserve_usdc_vault    = +reserve_share            (c)
//!   Δbid_credit_balance    = +participant_share        (d, virtual)
//! ```
//!
//! Sum across token endpoints = 0; the participant share is virtually
//! tracked in `bid_credit_balance` (NOT a token move) — same convention
//! as `claim_winning`'s 75% bid credit.
//!
//! ## Errors (no new errors needed in this step)
//!
//!   - `ProtocolPaused`        — if `protocol_config.paused == true`
//!   - `PoolComplete`          — if `pool.is_complete == true`
//!   - `TierNotYetSupported`   — if `pool.tier != Tier::Vault` (until step 12)
//!   - `MathOverflow`          — checked-arith failures

use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::get_return_data;
use anchor_spl::token::{self, Token, Transfer};

use crate::constants::{
    BPS_DENOMINATOR, CORE_INVOKER_SEED, POOL_USDC_VAULT_SEED, PROTOCOL_CONFIG_SEED,
    PROTOCOL_FEE_VAULT_SEED, RESERVE_FUND_SEED, RESERVE_VAULT_SEED,
};
use crate::error::CoreError;
use crate::events::{YieldDistributed, YieldHarvested};
use crate::state::{Pool, ProtocolConfig, Tier};

/// Yield distribution split constants (spec §4).
/// Sum is 10_000 bps (100%); the participant share is computed by
/// subtraction so any BPS rounding error stays inside the participant
/// pool (rounding goes INTO the bid_credit_balance, never out).
const PROTOCOL_YIELD_SHARE_BPS: u64 = 1_000; // 10%
const RESERVE_YIELD_SHARE_BPS: u64 = 2_000; // 20%
// Participant share = 7_000 bps (70%) — derived by subtraction.

/// Pure helper: split `yield_amount` into (participant, reserve, protocol)
/// per spec §4 (70/20/10). Subtraction-based for the participant share so
/// any BPS rounding error stays solvent (rounding goes INTO the pool, not
/// OUT of it). Public so step-9 unit tests can drive it without LiteSVM.
///
/// Returns `Err(CoreError::MathOverflow)` only if `yield_amount` exceeds
/// `u64::MAX / 2_000`, which is ~2.3e15 USDC — far beyond any realistic
/// harvest.
pub fn compute_yield_splits(yield_amount: u64) -> Result<(u64, u64, u64)> {
    let protocol_share = yield_amount
        .checked_mul(PROTOCOL_YIELD_SHARE_BPS)
        .and_then(|v| v.checked_div(BPS_DENOMINATOR))
        .ok_or(CoreError::MathOverflow)?;
    let reserve_share = yield_amount
        .checked_mul(RESERVE_YIELD_SHARE_BPS)
        .and_then(|v| v.checked_div(BPS_DENOMINATOR))
        .ok_or(CoreError::MathOverflow)?;
    let participant_share = yield_amount
        .checked_sub(protocol_share)
        .and_then(|v| v.checked_sub(reserve_share))
        .ok_or(CoreError::MathOverflow)?;
    Ok((participant_share, reserve_share, protocol_share))
}

/// SPEC_QUESTION-15: `Pool` is `Box`'d. `protocol_config` is
/// `UncheckedAccount` and manually deserialized in the handler to keep
/// the `try_accounts`-time stack frame under the 4 KB BPF budget. Same
/// trade-off as `claim_winning` / `select_winner` / `contribute`.
#[derive(Accounts)]
pub struct DistributeYield<'info> {
    /// Permissionless caller — anyone can pay tx fees and trigger a
    /// harvest. In production this is a keeper bot; nothing special is
    /// required of the signer beyond rent-paying ability.
    #[account(mut)]
    pub caller: Signer<'info>,

    /// Protocol config — manually deserialized. CHECK: PDA seed binding
    /// here, owner+discriminator validated in handler.
    #[account(seeds = [PROTOCOL_CONFIG_SEED], bump)]
    pub protocol_config: UncheckedAccount<'info>,

    /// The pool. Mut because we write `total_yield_distributed` and
    /// `bid_credit_balance` on positive-yield paths.
    #[account(mut)]
    pub pool: Box<Account<'info, Pool>>,

    /// Pool USDC vault. PDA-owned token account; signs the protocol-fee
    /// transfer and the reserve-deposit CPI.
    /// CHECK: PDA seed binding + key equality with `pool.pool_usdc_vault`.
    #[account(
        mut,
        seeds = [POOL_USDC_VAULT_SEED, pool.key().as_ref()],
        bump,
        constraint = pool_usdc_vault.key() == pool.pool_usdc_vault
            @ CoreError::Unauthorized,
    )]
    pub pool_usdc_vault: UncheckedAccount<'info>,

    /// Protocol fee SPL vault. Receives 10% of `yield_amount`.
    /// CHECK: PDA seed binding; equality with `protocol_config.protocol_fee_vault`
    /// validated in handler after manual deser.
    #[account(mut, seeds = [PROTOCOL_FEE_VAULT_SEED], bump)]
    pub protocol_fee_vault: UncheckedAccount<'info>,

    /// `core_invoker` PDA — co-signs reserve + yield-vault CPIs (arch §5.2).
    /// CHECK: AccountInfo only; bump validated by Anchor seeds.
    #[account(seeds = [CORE_INVOKER_SEED], bump)]
    pub core_invoker: UncheckedAccount<'info>,

    // ───── Reserve CPI accounts (validated by reserve via tier seed) ─────
    /// CHECK: validated by `poolver_reserve::deposit`; we additionally
    /// re-derive in the handler against `pool.tier` for INV-4 (tier
    /// isolation — a Tier 0 pool MUST NOT distribute yield into the
    /// Tier 1 reserve).
    #[account(mut)]
    pub reserve_fund: UncheckedAccount<'info>,

    /// CHECK: validated by `poolver_reserve::deposit`.
    #[account(mut)]
    pub reserve_usdc_vault: UncheckedAccount<'info>,

    /// CHECK: hardcoded program ID.
    #[account(address = poolver_reserve::ID)]
    pub reserve_program: UncheckedAccount<'info>,

    // ───── Yield-adapter CPI accounts (SPEC_QUESTION-36 — both tiers) ─────
    //
    // Pre-step-13 these had `seeds::program = poolver_yield_vault::ID`
    // baked in. Step 13 unlocked Tier 1 so we drop the per-program seed
    // binding here and re-derive in the handler against the canonical
    // tier-specific seed (`VAULT_ADAPTER_*` for Tier 0, `DEFI_ADAPTER_*`
    // for Tier 1). The adapter program itself ALSO validates seeds on
    // its side (defense-in-depth), so a wrong-tier account combo fails
    // at the CPI boundary even if our re-derivation is bypassed.

    /// CHECK: validated by the chosen adapter's `harvest` / `withdraw`
    /// via PDA seeds + handler-side tier-aware re-derivation.
    #[account(mut)]
    pub adapter_state: UncheckedAccount<'info>,

    /// CHECK: validated by the chosen adapter's `harvest` / `withdraw`
    /// via PDA seeds + handler-side tier-aware re-derivation.
    #[account(mut)]
    pub adapter_usdc_vault: UncheckedAccount<'info>,

    /// CHECK: SPEC_QUESTION-36 — adapter program ID validated against
    /// `pool.tier` in the handler via `cpi_adapter_harvest` /
    /// `cpi_adapter_withdraw`.
    pub yield_adapter_program: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

// ─── CPI helper frames (SPEC_QUESTION-15: split CPIs across stack frames) ─

// SPEC_QUESTION-36: per-instruction `cpi_yield_harvest` /
// `cpi_yield_withdraw` removed in step 13. Tier dispatch lives in
// `crate::adapter_cpi::adapter::cpi_adapter_harvest` /
// `cpi_adapter_withdraw`.

#[inline(never)]
fn cpi_pool_to_fee_vault<'info>(
    token_program: AccountInfo<'info>,
    pool_usdc_vault: AccountInfo<'info>,
    protocol_fee_vault: AccountInfo<'info>,
    pool_key: &Pubkey,
    pool_usdc_vault_bump: u8,
    amount: u64,
) -> Result<()> {
    let seeds: &[&[&[u8]]] = &[&[
        POOL_USDC_VAULT_SEED,
        pool_key.as_ref(),
        &[pool_usdc_vault_bump],
    ]];
    let cpi_ctx = CpiContext::new_with_signer(
        token_program.key(),
        Transfer {
            from: pool_usdc_vault.clone(),
            to: protocol_fee_vault,
            authority: pool_usdc_vault,
        },
        seeds,
    );
    token::transfer(cpi_ctx, amount)
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn cpi_reserve_deposit<'info>(
    reserve_program: AccountInfo<'info>,
    core_invoker: AccountInfo<'info>,
    reserve_fund: AccountInfo<'info>,
    reserve_usdc_vault: AccountInfo<'info>,
    pool_usdc_vault: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    pool_key: &Pubkey,
    core_invoker_bump: u8,
    pool_usdc_vault_bump: u8,
    amount: u64,
) -> Result<()> {
    let cpi_accounts = poolver_reserve::cpi::accounts::ReserveDepositCtx {
        core_invoker,
        reserve_fund,
        reserve_usdc_vault,
        source_usdc: pool_usdc_vault.clone(),
        source_authority: pool_usdc_vault,
        token_program,
    };
    let combined_seeds: &[&[&[u8]]] = &[
        &[CORE_INVOKER_SEED, &[core_invoker_bump]],
        &[
            POOL_USDC_VAULT_SEED,
            pool_key.as_ref(),
            &[pool_usdc_vault_bump],
        ],
    ];
    let cpi_ctx = CpiContext::new_with_signer(
        reserve_program.key(),
        cpi_accounts,
        combined_seeds,
    );
    poolver_reserve::cpi::deposit(cpi_ctx, amount)
}

pub fn handle_distribute_yield<'info>(
    ctx: Context<'info, DistributeYield<'info>>,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let pool_key = ctx.accounts.pool.key();

    // ───── 1. Pool gates ───────────────────────────────────────────────
    let pool_tier: Tier;
    {
        let pool = &ctx.accounts.pool;
        require!(!pool.is_complete, CoreError::PoolComplete);
        // SPEC_QUESTION-36: step 13 — Tier 1 dispatch unlocked. Both
        // tiers route through the dispatcher below; only Tier 1 ever
        // produces a non-zero `yield_amount` from harvest (Tier 0 is a
        // no-op by definition per spec §5.3).
        pool_tier = pool.tier;
    }

    // ───── 2. Manual-deserialize protocol_config (Q-15) ────────────────
    let cfg: ProtocolConfig = {
        let acct = &ctx.accounts.protocol_config;
        require_keys_eq!(*acct.owner, crate::ID, CoreError::Unauthorized);
        let mut data: &[u8] = &acct.try_borrow_data()?;
        ProtocolConfig::try_deserialize(&mut data)?
    };
    require!(!cfg.paused, CoreError::ProtocolPaused);
    require_keys_eq!(
        ctx.accounts.protocol_fee_vault.key(),
        cfg.protocol_fee_vault,
        CoreError::Unauthorized
    );

    // ───── 3. Reserve isolation (INV-4) ────────────────────────────────
    // Defence-in-depth: poolver-reserve's own `seeds` constraint catches
    // a wrong-tier reserve, but re-deriving here gives us a clear
    // CoreError::Unauthorized at the call site instead of a deeper
    // ConstraintSeeds error.
    let tier_seed = [pool_tier.as_u8()];
    let (expected_reserve_fund, _) = Pubkey::find_program_address(
        &[RESERVE_FUND_SEED, &tier_seed],
        &poolver_reserve::ID,
    );
    require_keys_eq!(
        ctx.accounts.reserve_fund.key(),
        expected_reserve_fund,
        CoreError::Unauthorized
    );
    let (expected_reserve_vault, _) = Pubkey::find_program_address(
        &[RESERVE_VAULT_SEED, &tier_seed],
        &poolver_reserve::ID,
    );
    require_keys_eq!(
        ctx.accounts.reserve_usdc_vault.key(),
        expected_reserve_vault,
        CoreError::Unauthorized
    );

    let core_invoker_bump = ctx.bumps.core_invoker;
    let pool_usdc_vault_bump = ctx.bumps.pool_usdc_vault;

    // ───── 4. CPI: adapter::harvest() (SPEC_QUESTION-36) ───────────────
    // Returns yield_amount via Anchor's set_return_data plumbing. Tier 0
    // always returns 0 (spec §5.3); Tier 1 returns the realized delta vs
    // the adapter's `last_recorded_balance`. Both adapters expose the
    // same `harvest()` discriminator (arch §13.1).
    crate::adapter_cpi::adapter::cpi_adapter_harvest(
        pool_tier,
        ctx.accounts.yield_adapter_program.to_account_info(),
        ctx.accounts.core_invoker.to_account_info(),
        ctx.accounts.adapter_state.to_account_info(),
        ctx.accounts.adapter_usdc_vault.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        ctx.remaining_accounts,
        core_invoker_bump,
    )?;

    // Read the harvest return value. Defence-in-depth: if the callee
    // didn't set return data, treat as 0 (Tier 0 currently doesn't call
    // `set_return_data` — its `Ok(0)` Result flows through Anchor's
    // typed return-data plumbing on the caller side, which we don't use
    // directly here).
    //
    // SPEC_QUESTION-36: program-ID check now branches on `pool.tier`.
    let expected_adapter_id = match pool_tier {
        Tier::Vault => poolver_yield_vault::ID,
        Tier::DeFi => poolver_yield_defi::ID,
    };
    let yield_amount: u64 = match get_return_data() {
        Some((program_id, bytes)) => {
            require_keys_eq!(
                program_id,
                expected_adapter_id,
                CoreError::Unauthorized
            );
            if bytes.len() == 8 {
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&bytes[..8]);
                u64::from_le_bytes(buf)
            } else {
                0
            }
        }
        None => 0,
    };

    // ───── 5. Compute splits ───────────────────────────────────────────
    let (participant_share, reserve_share, protocol_share) =
        compute_yield_splits(yield_amount)?;

    // ───── 6. Zero-yield short-circuit (V1 Tier 0 happy path) ──────────
    if yield_amount == 0 {
        // Emit both events with zeroes so indexers see a uniform schema
        // regardless of yield outcome. State stays untouched; no token
        // movements; no further CPIs.
        emit!(YieldHarvested {
            pool: pool_key,
            tier: pool_tier.as_u8(),
            yield_amount: 0,
            timestamp: now,
        });
        let pool_snapshot = &ctx.accounts.pool;
        emit!(YieldDistributed {
            pool: pool_key,
            total_yield: 0,
            participant_share: 0,
            reserve_share: 0,
            protocol_share: 0,
            bid_credit_balance_after: pool_snapshot.bid_credit_balance,
            total_yield_distributed_after: pool_snapshot.total_yield_distributed,
            timestamp: now,
        });
        return Ok(());
    }

    // ───── 7. Positive-yield path (Tier 1 only in V1; reserved here
    //         for step 12) ─────────────────────────────────────────────
    //
    // (a) Withdraw the full yield_amount from adapter to pool_usdc_vault.
    crate::adapter_cpi::adapter::cpi_adapter_withdraw(
        pool_tier,
        ctx.accounts.yield_adapter_program.to_account_info(),
        ctx.accounts.core_invoker.to_account_info(),
        ctx.accounts.adapter_state.to_account_info(),
        ctx.accounts.adapter_usdc_vault.to_account_info(),
        ctx.accounts.pool_usdc_vault.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        ctx.remaining_accounts,
        core_invoker_bump,
        yield_amount,
    )?;

    // (b) pool_usdc_vault → protocol_fee_vault: protocol_share (10%).
    if protocol_share > 0 {
        cpi_pool_to_fee_vault(
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.pool_usdc_vault.to_account_info(),
            ctx.accounts.protocol_fee_vault.to_account_info(),
            &pool_key,
            pool_usdc_vault_bump,
            protocol_share,
        )?;
    }

    // (c) reserve::deposit(reserve_share) source = pool_usdc_vault (20%).
    if reserve_share > 0 {
        cpi_reserve_deposit(
            ctx.accounts.reserve_program.to_account_info(),
            ctx.accounts.core_invoker.to_account_info(),
            ctx.accounts.reserve_fund.to_account_info(),
            ctx.accounts.reserve_usdc_vault.to_account_info(),
            ctx.accounts.pool_usdc_vault.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            &pool_key,
            core_invoker_bump,
            pool_usdc_vault_bump,
            reserve_share,
        )?;
    }

    // (d) participant_share — virtual credit. No token movement; tokens
    //     stay in pool_usdc_vault and back the bid_credit_balance ledger.

    // ───── 8. State updates ────────────────────────────────────────────
    let bid_credit_balance_after: u64;
    let total_yield_distributed_after: u64;
    {
        let pool = &mut ctx.accounts.pool;
        // INV "Yield monotonic": total_yield_distributed only grows.
        pool.total_yield_distributed = pool
            .total_yield_distributed
            .checked_add(yield_amount)
            .ok_or(CoreError::MathOverflow)?;
        // SPEC_QUESTION-1: pool-wide credit ledger; consumed pro-rata in
        // subsequent contribute calls (same pattern as claim_winning).
        pool.bid_credit_balance = pool
            .bid_credit_balance
            .checked_add(participant_share)
            .ok_or(CoreError::MathOverflow)?;
        bid_credit_balance_after = pool.bid_credit_balance;
        total_yield_distributed_after = pool.total_yield_distributed;
    }

    // ───── 9. Events ───────────────────────────────────────────────────
    // SPEC_QUESTION-17: one summary event for the distribution (NOT
    // per-participant). Indexers reconstruct the 70/20/10 split from this
    // single record.
    emit!(YieldHarvested {
        pool: pool_key,
        tier: pool_tier.as_u8(),
        yield_amount,
        timestamp: now,
    });
    emit!(YieldDistributed {
        pool: pool_key,
        total_yield: yield_amount,
        participant_share,
        reserve_share,
        protocol_share,
        bid_credit_balance_after,
        total_yield_distributed_after,
        timestamp: now,
    });

    Ok(())
}
