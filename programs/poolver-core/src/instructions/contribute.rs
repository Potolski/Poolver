use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, Transfer};

use crate::constants::{
    BPS_DENOMINATOR, COLLATERAL_VAULT_SEED, CORE_INVOKER_SEED, LIQUIDATION_THRESHOLD_SECS,
    PARTICIPANT_SEED, POOL_SIZE, POOL_USDC_VAULT_SEED, PROTOCOL_CONFIG_SEED,
    PROTOCOL_FEE_VAULT_SEED, REPUTATION_SEED, RESERVE_FUND_SEED, RESERVE_VAULT_SEED,
};
use crate::error::CoreError;
use crate::events::Contribution;
use crate::state::{Participant, Pool, ProtocolConfig, Tier, UserReputation};

/// `contribute` — spec §5.1.
///
/// Custody-then-route flow (mirrors `join_pool` so the SPL transfer
/// authority is proven once, then forwarded through both CPIs):
///
///   1. user → `pool_usdc_vault`: `actual_paid_by_user`. User signs.
///   2. `pool_usdc_vault` → `protocol_fee_vault`: `protocol_fee`. Pool
///      USDC vault PDA signs.
///   3. CPI `poolver_reserve::deposit(reserve_fee)` with `source_usdc =
///      pool_usdc_vault`, `source_authority = pool_usdc_vault PDA`.
///      `core_invoker` PDA additionally signs (arch §5.2).
///   4. CPI `poolver_yield_vault::deposit(net_to_pool)` — same pattern.
///   5. (Post-win only) `collateral_vault` → user_usdc:
///      `release_amount`. Collateral vault PDA signs.
///
/// SPEC_QUESTION-10 (Q-10): protocol fee + reserve fee are deducted
/// from the user's contribution; the user pays `actual_paid_by_user`
/// gross which already reflects any bid-credit discount.
///
/// SPEC_QUESTION-1 (Q-1): the `bid_credit_balance` discount is
/// pool-wide and divided pro-rata among the unpaid-this-month
/// participants. In step 5 no winner has been selected yet (winner
/// selection lands in step 7+8) so `bid_credit_balance` is always 0
/// and `credit_per_share` resolves to 0. The math hook is in place;
/// step 8's `claim_winning` is what makes the balance non-zero.
///
/// SPEC_QUESTION-6 (Q-6): late-payment marking + grace period are
/// implemented in step 10 (defaults). Step 5 enforces strict in-window
/// contribution; outside-window calls return `OutsideMonthWindow`.
///
/// SPEC_QUESTION-15: `Pool` is `Box`'d, `protocol_config` is manually
/// deserialized inside the handler to keep `try_accounts` under the
/// 4 KB BPF stack budget. Same trade as `join_pool`.
#[derive(Accounts)]
pub struct Contribute<'info> {
    /// The participant paying their monthly contribution. Pays the SPL
    /// transfer fee from their own USDC ATA.
    #[account(mut)]
    pub user: Signer<'info>,

    /// Protocol config. Manually deserialized in the handler
    /// (SPEC_QUESTION-15). CHECK: PDA seed binding here, owner +
    /// discriminator checked manually in the handler.
    #[account(seeds = [PROTOCOL_CONFIG_SEED], bump)]
    pub protocol_config: UncheckedAccount<'info>,

    /// User's reputation — `total_contributed_lifetime` is incremented
    /// after CPIs succeed.
    #[account(
        mut,
        seeds = [REPUTATION_SEED, user.key().as_ref()],
        bump = user_reputation.bump,
    )]
    pub user_reputation: Box<Account<'info, UserReputation>>,

    /// The pool. Box'd to keep stack pressure low (large
    /// `[MonthWinner; 12]` array; SPEC_QUESTION-15).
    #[account(mut)]
    pub pool: Box<Account<'info, Pool>>,

    /// Per-(pool, user) participant record. The PDA seed binding here
    /// also doubles as the "is the caller a participant?" check —
    /// non-participants can't construct a matching seed.
    #[account(
        mut,
        seeds = [PARTICIPANT_SEED, pool.key().as_ref(), user.key().as_ref()],
        bump = participant.bump,
        constraint = participant.pool == pool.key() @ CoreError::NotAParticipant,
        constraint = participant.user == user.key() @ CoreError::NotAParticipant,
    )]
    pub participant: Box<Account<'info, Participant>>,

    /// User's USDC source. Like `join_pool`, kept as `UncheckedAccount`
    /// to relieve stack pressure; SPL transfer enforces ownership and
    /// balance at CPI time.
    /// CHECK: SPL transfer enforces semantics.
    #[account(mut)]
    pub user_usdc: UncheckedAccount<'info>,

    /// Pool USDC vault — the PDA-owned transit account. Authority
    /// derives from its own seeds (`token::authority = pool_usdc_vault`
    /// at init time).
    /// CHECK: PDA seed binding + key equality with `pool.pool_usdc_vault`.
    #[account(
        mut,
        seeds = [POOL_USDC_VAULT_SEED, pool.key().as_ref()],
        bump,
        constraint = pool_usdc_vault.key() == pool.pool_usdc_vault
            @ CoreError::Unauthorized,
    )]
    pub pool_usdc_vault: UncheckedAccount<'info>,

    /// Collateral vault — used for the post-win release transfer. Mut
    /// because we may move USDC out of it.
    /// CHECK: PDA seed binding + key equality with `pool.collateral_vault`.
    #[account(
        mut,
        seeds = [COLLATERAL_VAULT_SEED, pool.key().as_ref()],
        bump,
        constraint = collateral_vault.key() == pool.collateral_vault
            @ CoreError::Unauthorized,
    )]
    pub collateral_vault: UncheckedAccount<'info>,

    /// Protocol fee SPL vault.
    /// CHECK: PDA seed binding; handler additionally checks equality
    /// with `protocol_config.protocol_fee_vault`.
    #[account(
        mut,
        seeds = [PROTOCOL_FEE_VAULT_SEED],
        bump,
    )]
    pub protocol_fee_vault: UncheckedAccount<'info>,

    /// `core_invoker` PDA, signs the reserve / adapter CPIs.
    /// CHECK: AccountInfo only; bump validated by Anchor seeds.
    #[account(seeds = [CORE_INVOKER_SEED], bump)]
    pub core_invoker: UncheckedAccount<'info>,

    // ───── Reserve CPI accounts ─────
    //
    // Validated INSIDE `poolver_reserve::deposit` via tier-encoded seeds
    // (`[RESERVE_FUND_SEED, &[tier_byte]]`). INV-4 (reserve isolation)
    // is structurally enforced there, not here. SPEC_QUESTION-15.

    /// CHECK: validated by `poolver_reserve::deposit`.
    #[account(mut)]
    pub reserve_fund: UncheckedAccount<'info>,

    /// CHECK: validated by `poolver_reserve::deposit`.
    #[account(mut)]
    pub reserve_usdc_vault: UncheckedAccount<'info>,

    /// CHECK: hardcoded program ID.
    #[account(address = poolver_reserve::ID)]
    pub reserve_program: UncheckedAccount<'info>,

    /// CHECK: validated by the chosen adapter's `deposit`.
    #[account(mut)]
    pub adapter_state: UncheckedAccount<'info>,

    /// CHECK: validated by the chosen adapter's `deposit`.
    #[account(mut)]
    pub adapter_usdc_vault: UncheckedAccount<'info>,

    /// CHECK: SPEC_QUESTION-36 — adapter program ID is validated by
    /// `cpi_adapter_deposit` against `pool.tier`.
    pub yield_adapter_program: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

// ─── CPI helper frames (SPEC_QUESTION-15: split CPIs across stack frames) ─

#[inline(never)]
fn cpi_user_to_pool_vault<'info>(
    token_program: AccountInfo<'info>,
    user_usdc: AccountInfo<'info>,
    pool_usdc_vault: AccountInfo<'info>,
    user: AccountInfo<'info>,
    amount: u64,
) -> Result<()> {
    let cpi_ctx = CpiContext::new(
        token_program.key(),
        Transfer {
            from: user_usdc,
            to: pool_usdc_vault,
            authority: user,
        },
    );
    token::transfer(cpi_ctx, amount)
}

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

// SPEC_QUESTION-36: per-instruction `cpi_adapter_deposit` was deleted in
// step 13. Tier dispatch lives in `crate::adapter_cpi::adapter` and is
// invoked directly from the handler with the pool's tier + the same
// fixed accounts that this helper previously took. Tier 1 callers
// append `adapter_ktoken_vault` to `remaining_accounts[0]`.

#[inline(never)]
fn cpi_collateral_release<'info>(
    token_program: AccountInfo<'info>,
    collateral_vault: AccountInfo<'info>,
    user_usdc: AccountInfo<'info>,
    pool_key: &Pubkey,
    collateral_vault_bump: u8,
    amount: u64,
) -> Result<()> {
    let seeds: &[&[&[u8]]] = &[&[
        COLLATERAL_VAULT_SEED,
        pool_key.as_ref(),
        &[collateral_vault_bump],
    ]];
    let cpi_ctx = CpiContext::new_with_signer(
        token_program.key(),
        Transfer {
            from: collateral_vault.clone(),
            to: user_usdc,
            authority: collateral_vault,
        },
        seeds,
    );
    token::transfer(cpi_ctx, amount)
}

pub fn handle_contribute<'info>(
    ctx: Context<'info, Contribute<'info>>,
) -> Result<()> {
    // ───── 1. Pool / participant pre-checks ────────────────────────────
    let now = Clock::get()?.unix_timestamp;
    let current_month: u8;
    let contribution_amount: u64;
    let pool_tier: Tier;
    let bid_credit_balance: u64;
    let paid_count_for_current_month: u8;

    {
        let pool = &ctx.accounts.pool;
        require!(!pool.is_complete, CoreError::PoolComplete);
        require!(
            pool.current_month >= 1 && pool.current_month <= Pool::TOTAL_MONTHS,
            CoreError::PoolNotStarted
        );
        // SPEC_QUESTION-36: step 13 — Tier dispatch handled in CPI helper
        // below; this handler accepts both `Tier::Vault` and `Tier::DeFi`.
        // SPEC_QUESTION-6 (step 10 cure path): contribution accepted in
        // the strict in-window OR during the day 1..=29 grace/suspension
        // window — i.e. any time before liquidation at day 30. This lets
        // a late/suspended participant cure by paying the contribution +
        // accrued penalty. Liquidation at day 30+ is the hard cutoff.
        let month_end = pool
            .current_month_started_at
            .checked_add(pool.month_duration_seconds)
            .ok_or(CoreError::MathOverflow)?;
        let liquidation_threshold = month_end
            .checked_add(LIQUIDATION_THRESHOLD_SECS)
            .ok_or(CoreError::MathOverflow)?;
        require!(
            now >= pool.current_month_started_at && now < liquidation_threshold,
            CoreError::OutsideMonthWindow
        );

        current_month = pool.current_month;
        contribution_amount = pool.contribution_amount;
        pool_tier = pool.tier;
        bid_credit_balance = pool.bid_credit_balance;
        paid_count_for_current_month = pool.paid_count_for_current_month;
    }

    {
        let participant = &ctx.accounts.participant;
        // SPEC_QUESTION-6 cure-path: `is_late` / `is_suspended` are NOT
        // blockers for contribute (step 10) — paying clears them. Only
        // post-liquidation `is_defaulted` is permanent.
        require!(!participant.is_defaulted, CoreError::Defaulted);
        require!(
            !participant.has_paid_month(current_month),
            CoreError::ContributionAlreadyMade
        );
    }

    // ───── 2. Manual-deserialize protocol_config (SPEC_QUESTION-15) ────
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

    // ───── 2.5 Reserve isolation (INV-4 / arch §11) ────────────────────
    // Re-derive the canonical reserve fund + vault PDAs from this pool's
    // tier and require equality with the supplied accounts. Reserve's
    // `deposit` already constrains its own seed against
    // `reserve_fund.tier`, but the `tier` field could be any tier on a
    // valid `ReserveFund` account; without this check, a Tier-0 pool
    // could route fees into the Tier-1 reserve. The pool-tier-bound
    // derivation here is the structural enforcement promised by arch §11.
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

    // ───── 3. Bid-credit discount (Q-1) ───────────────────────────────
    //
    // SPEC_QUESTION-1: pool-wide `bid_credit_balance` is divided pro-rata
    // among the participants who have NOT YET paid for the current month.
    // Implementation: at each `contribute`, the divisor is
    // `(POOL_SIZE - paid_count_for_current_month)` — the number of users
    // still owing a contribution this month, INCLUDING the current
    // caller. Each call deducts exactly its share from the balance.
    //
    // Why this works:
    //   - Sum across the month equals the original balance (modulo
    //     truncation; see below). If 12 users contribute in month M
    //     after a winner has been selected for month < M, the formula
    //     allocates `B/12 + (B - B/12)/11 + ...` which converges to
    //     drain B almost exactly across 12 calls.
    //   - Late month-rollover: any unspent balance carries forward into
    //     the next month — no token movement, just a virtual ledger.
    //   - Truncation: integer division leaves at most `POOL_SIZE - 1`
    //     dust units stranded in the balance per month. Acceptable —
    //     the dust accumulates and gets paid out during the FINAL
    //     contribute of a future month when the divisor is 1.
    //
    // The check below rejects the impossible-by-construction case where
    // every participant has already paid (`paid_count >= POOL_SIZE`).
    // That can only happen if `mark_month_paid` was bypassed.
    let credit_per_share: u64 = if bid_credit_balance == 0 {
        0
    } else {
        let unpaid_this_month = (POOL_SIZE as u64)
            .checked_sub(paid_count_for_current_month as u64)
            .ok_or(CoreError::MathOverflow)?;
        if unpaid_this_month == 0 {
            // Defensive: every participant already paid this month.
            // Skip the credit (no divisor); next month's calls will
            // resume the pro-rata draw.
            0
        } else {
            bid_credit_balance
                .checked_div(unpaid_this_month)
                .unwrap_or(0)
        }
    };

    // ───── 4. Fee math (Q-10: deducted from gross) ─────────────────────
    let reserve_fee_bps = match pool_tier {
        Tier::Vault => cfg.vault_reserve_fee_bps,
        Tier::DeFi => cfg.defi_reserve_fee_bps,
    } as u64;
    let protocol_fee_bps = cfg.protocol_fee_bps as u64;

    let protocol_fee = contribution_amount
        .checked_mul(protocol_fee_bps)
        .and_then(|v| v.checked_div(BPS_DENOMINATOR))
        .ok_or(CoreError::MathOverflow)?;
    let reserve_fee = contribution_amount
        .checked_mul(reserve_fee_bps)
        .and_then(|v| v.checked_div(BPS_DENOMINATOR))
        .ok_or(CoreError::MathOverflow)?;

    // SPEC_QUESTION-6 (step 10 cure-path): if the participant has any
    // accrued late penalty (from `mark_late_payment`), it is added on top
    // of the contribution and routed to `pool.bid_credit_balance` so the
    // honest participants are made whole. The penalty does NOT pay
    // protocol/reserve fees — it goes 100% to the bid-credit ledger.
    //
    // Token-flow effect:
    //   user → pool_usdc_vault: actual_paid_by_user + accrued_penalty
    //   pool_usdc_vault → protocol_fee_vault: protocol_fee  (only on contribution, not penalty)
    //   reserve::deposit: reserve_fee                       (only on contribution, not penalty)
    //   yield_vault::deposit: net_to_pool                   (only on contribution, not penalty)
    //   The penalty stays in pool_usdc_vault and backs the credit.
    let accrued_penalty = ctx.accounts.participant.late_penalty_accrued;

    // Q-1 application: the user's effective owed amount drops by their
    // share of the bid-credit pool, then bumps back up by accrued penalty.
    let contribution_after_credit = contribution_amount
        .checked_sub(credit_per_share)
        .ok_or(CoreError::MathOverflow)?;
    let actual_paid_by_user = contribution_after_credit
        .checked_add(accrued_penalty)
        .ok_or(CoreError::MathOverflow)?;
    let net_to_pool = contribution_after_credit
        .checked_sub(protocol_fee)
        .and_then(|v| v.checked_sub(reserve_fee))
        .ok_or(CoreError::MathOverflow)?;

    let pool_key = ctx.accounts.pool.key();
    let pool_usdc_vault_bump = ctx.bumps.pool_usdc_vault;
    let core_invoker_bump = ctx.bumps.core_invoker;
    let collateral_vault_bump = ctx.bumps.collateral_vault;

    // ───── 5. user → pool_usdc_vault ───────────────────────────────────
    if actual_paid_by_user > 0 {
        cpi_user_to_pool_vault(
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.user_usdc.to_account_info(),
            ctx.accounts.pool_usdc_vault.to_account_info(),
            ctx.accounts.user.to_account_info(),
            actual_paid_by_user,
        )?;
    }

    // ───── 6. pool_usdc_vault → protocol_fee_vault ─────────────────────
    if protocol_fee > 0 {
        cpi_pool_to_fee_vault(
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.pool_usdc_vault.to_account_info(),
            ctx.accounts.protocol_fee_vault.to_account_info(),
            &pool_key,
            pool_usdc_vault_bump,
            protocol_fee,
        )?;
    }

    // ───── 7. CPI: reserve::deposit(reserve_fee) ───────────────────────
    if reserve_fee > 0 {
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
            reserve_fee,
        )?;
    }

    // ───── 8. CPI: adapter::deposit(net_to_pool) (SPEC_QUESTION-36) ────
    if net_to_pool > 0 {
        let combined_seeds: &[&[&[u8]]] = &[
            &[CORE_INVOKER_SEED, &[core_invoker_bump]],
            &[
                POOL_USDC_VAULT_SEED,
                pool_key.as_ref(),
                &[pool_usdc_vault_bump],
            ],
        ];
        crate::adapter_cpi::adapter::cpi_adapter_deposit(
            pool_tier,
            ctx.accounts.yield_adapter_program.to_account_info(),
            ctx.accounts.core_invoker.to_account_info(),
            ctx.accounts.adapter_state.to_account_info(),
            ctx.accounts.adapter_usdc_vault.to_account_info(),
            ctx.accounts.pool_usdc_vault.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.remaining_accounts,
            combined_seeds,
            net_to_pool,
        )?;
    }

    // ───── 9. Post-win collateral release (spec §4 schedule) ───────────
    let mut collateral_released: u64 = 0;
    {
        let participant = &ctx.accounts.participant;
        if participant.has_won
            && participant.collateral_locked > 0
            && participant.win_month >= 1
            && participant.win_month < Pool::TOTAL_MONTHS
            && current_month > participant.win_month
        {
            let win_month = participant.win_month;
            let months_remaining_at_win = Pool::TOTAL_MONTHS
                .saturating_sub(win_month) as u64;
            let release_per_month = participant.collateral_release_per_month;

            // Final-month true-up: release whatever is left so total
            // released never exceeds `collateral_initial` and the locked
            // balance lands at 0 on the last on-time payment.
            let is_final_payment = months_remaining_at_win > 0
                && current_month
                    .saturating_sub(win_month) as u64
                    >= months_remaining_at_win;

            collateral_released = if is_final_payment {
                participant.collateral_locked
            } else {
                release_per_month.min(participant.collateral_locked)
            };
        }
    }
    if collateral_released > 0 {
        cpi_collateral_release(
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.collateral_vault.to_account_info(),
            ctx.accounts.user_usdc.to_account_info(),
            &pool_key,
            collateral_vault_bump,
            collateral_released,
        )?;
    }

    // ───── 10. State updates (after token movements succeed) ───────────
    let participant = &mut ctx.accounts.participant;
    // INV-3: bits flip 0→1 only. `mark_month_paid` uses `|=` exclusively.
    participant.mark_month_paid(current_month);
    if collateral_released > 0 {
        // INV-4: collateral monotonic decrease — saturating_sub guards
        // against the (impossible-by-construction) underflow case.
        participant.collateral_locked =
            participant.collateral_locked.saturating_sub(collateral_released);
    }

    // SPEC_QUESTION-6 cure-path: paying clears the late / suspended
    // markers. Penalty is consumed (set to 0) — the value moved into
    // `pool_usdc_vault` and backs the bid_credit_balance bump below.
    if participant.is_late || participant.is_suspended || participant.late_penalty_accrued > 0 {
        participant.is_late = false;
        participant.is_suspended = false;
        participant.late_marked_at = 0;
        participant.suspended_at = 0;
        participant.late_penalty_accrued = 0;
    }

    let paid_months_after = participant.paid_months;

    let pool = &mut ctx.accounts.pool;
    pool.total_contributed = pool
        .total_contributed
        .checked_add(actual_paid_by_user)
        .ok_or(CoreError::MathOverflow)?;
    if collateral_released > 0 {
        pool.total_collateral_locked = pool
            .total_collateral_locked
            .saturating_sub(collateral_released);
    }
    // SPEC_QUESTION-1: deduct this caller's share from the virtual
    // credit ledger and bump the per-month paid counter so the next
    // contribute sees a smaller divisor (and thus a larger share if any
    // balance remains).
    if credit_per_share > 0 {
        pool.bid_credit_balance = pool
            .bid_credit_balance
            .checked_sub(credit_per_share)
            .ok_or(CoreError::MathOverflow)?;
    }
    // SPEC_QUESTION-6 cure-path: late penalty bumps `bid_credit_balance`
    // so the honest participants benefit. The matching tokens are
    // already sitting in `pool_usdc_vault` from the user→vault transfer
    // above (we transferred actual_paid_by_user = contribution + penalty,
    // but only forwarded contribution_after_credit downstream).
    if accrued_penalty > 0 {
        pool.bid_credit_balance = pool
            .bid_credit_balance
            .checked_add(accrued_penalty)
            .ok_or(CoreError::MathOverflow)?;
    }
    pool.paid_count_for_current_month = pool
        .paid_count_for_current_month
        .checked_add(1)
        .ok_or(CoreError::MathOverflow)?;

    let rep = &mut ctx.accounts.user_reputation;
    rep.total_contributed_lifetime = rep
        .total_contributed_lifetime
        .checked_add(actual_paid_by_user)
        .ok_or(CoreError::MathOverflow)?;

    emit!(Contribution {
        pool: pool_key,
        user: ctx.accounts.user.key(),
        month: current_month,
        amount: actual_paid_by_user,
        protocol_fee,
        reserve_fee,
        net_to_pool,
        collateral_released,
        paid_months_after,
        timestamp: now,
    });

    Ok(())
}
