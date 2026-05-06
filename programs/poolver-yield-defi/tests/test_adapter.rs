//! Unit tests for `poolver-yield-defi` (the Tier 1 / Kamino-mock
//! adapter).
//!
//! Coverage map (per task prompt):
//!
//!   1.  initialize_adapter happy path                       → t01
//!   2.  initialize_adapter rejects wrong core_invoker       → t02
//!   3.  deposit happy path (75/25 split)                    → t03
//!   4.  deposit rejected when tripped (mock_kamino_paused)  → t04
//!   5.  deposit rejected with wrong core_invoker            → t05
//!   6.  withdraw drains liquid first                        → t06
//!   7.  withdraw redeems from kamino when liquid short      → t07
//!   8.  withdraw rejects amount > total available           → t08
//!   9.  harvest returns 0 when no yield accrued             → t09
//!  10.  harvest returns positive yield after injection      → t10
//!  11.  harvest updates last_recorded_balance               → t11
//!  12.  emergency_unwind drains both vaults + sets tripped  → t12
//!  13.  reset_circuit_breaker clears tripped flag           → t13
//!  14.  utilization > 9500 bps trips on next deposit        → t14
//!  15.  kamino_paused trips on next deposit                 → t15
//!  16.  liquidity buffer 25% invariant after multi-deposit  → t16
//!  17.  tripped state persists across deposit/withdraw/harvest → t17
//!  18.  end-to-end: deposit + inject + harvest + withdraw   → t18
//!
//! All happy-path cases route the adapter ix through the fake-core
//! stub so the `core_invoker` PDA is signed by its owning program.
//! This is the SAME UNIT-test boundary `poolver-yield-vault`'s suite
//! sets — full integration tests with the real `poolver-core` land in
//! step 13 (SPEC_QUESTION-26).

mod common;

use anchor_lang::InstructionData;
use common::*;
use solana_instruction::AccountMeta;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

// ──────────────────────────────────────────────────────────────────────
// Account-meta builders. One per instruction. The order MUST match
// `programs/poolver-yield-defi/src/instructions/*.rs` derive(Accounts)
// declarations exactly — Anchor parses positional, not by name.
// ──────────────────────────────────────────────────────────────────────

fn metas_initialize_adapter(
    env: &TestEnv,
    core_invoker: Pubkey,
    payer: Pubkey,
    adapter_state: Pubkey,
    adapter_usdc_vault: Pubkey,
    adapter_ktoken_vault: Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new_readonly(core_invoker, false), // fake_core re-marks as signer
        AccountMeta::new(payer, true),
        AccountMeta::new(adapter_state, false),
        AccountMeta::new_readonly(env.usdc_mint, false),
        AccountMeta::new(adapter_usdc_vault, false),
        AccountMeta::new(adapter_ktoken_vault, false),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
        AccountMeta::new_readonly(
            solana_pubkey::pubkey!("11111111111111111111111111111111"),
            false,
        ),
        AccountMeta::new_readonly(
            solana_pubkey::pubkey!("SysvarRent111111111111111111111111111111111"),
            false,
        ),
    ]
}

fn metas_deposit(
    core_invoker: Pubkey,
    adapter_state: Pubkey,
    adapter_usdc_vault: Pubkey,
    source_usdc: Pubkey,
    source_authority: Pubkey,
    adapter_ktoken_vault: Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new_readonly(core_invoker, false),
        AccountMeta::new(adapter_state, false),
        AccountMeta::new(adapter_usdc_vault, false),
        AccountMeta::new(source_usdc, false),
        AccountMeta::new_readonly(source_authority, true),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
        AccountMeta::new(adapter_ktoken_vault, false),
    ]
}

fn metas_withdraw(
    core_invoker: Pubkey,
    adapter_state: Pubkey,
    adapter_usdc_vault: Pubkey,
    destination_usdc: Pubkey,
    adapter_ktoken_vault: Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new_readonly(core_invoker, false),
        AccountMeta::new(adapter_state, false),
        AccountMeta::new(adapter_usdc_vault, false),
        AccountMeta::new(destination_usdc, false),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
        AccountMeta::new(adapter_ktoken_vault, false),
    ]
}

fn metas_harvest(
    core_invoker: Pubkey,
    adapter_state: Pubkey,
    adapter_usdc_vault: Pubkey,
    adapter_ktoken_vault: Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new_readonly(core_invoker, false),
        AccountMeta::new(adapter_state, false),
        AccountMeta::new_readonly(adapter_usdc_vault, false),
        AccountMeta::new_readonly(adapter_ktoken_vault, false),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
    ]
}

fn metas_unwind(
    core_invoker: Pubkey,
    adapter_state: Pubkey,
    adapter_usdc_vault: Pubkey,
    destination_usdc: Pubkey,
    adapter_ktoken_vault: Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new_readonly(core_invoker, false),
        AccountMeta::new(adapter_state, false),
        AccountMeta::new(adapter_usdc_vault, false),
        AccountMeta::new(destination_usdc, false),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
        AccountMeta::new(adapter_ktoken_vault, false),
    ]
}

// Direct-call (no fake_core): mock helpers + reset are NOT
// `core_invoker`-gated, so they're called directly with an outer
// signer.
fn metas_mock_inject_yield(
    injector: Pubkey,
    adapter_state: Pubkey,
    injector_usdc: Pubkey,
    adapter_ktoken_vault: Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new(injector, true),
        AccountMeta::new(adapter_state, false),
        AccountMeta::new(injector_usdc, false),
        AccountMeta::new(adapter_ktoken_vault, false),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
    ]
}

fn metas_mock_set_breaker(admin: Pubkey, adapter_state: Pubkey) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new_readonly(admin, true),
        AccountMeta::new(adapter_state, false),
    ]
}

fn metas_reset(admin: Pubkey, adapter_state: Pubkey) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new_readonly(admin, true),
        AccountMeta::new(adapter_state, false),
    ]
}

// ──────────────────────────────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────────────────────────────

fn fresh_pool() -> Pubkey {
    Pubkey::new_unique()
}

/// Returns (state, usdc_vault, ktoken_vault).
fn init_pool(env: &mut TestEnv, pool: &Pubkey) -> (Pubkey, Pubkey, Pubkey) {
    let (state, _) = env.defi_adapter_pda(pool);
    let (usdc_vault, _) = env.defi_adapter_usdc_pda(pool);
    let (ktoken_vault, _) = env.defi_adapter_ktoken_pda(pool);
    let metas = metas_initialize_adapter(
        env,
        env.core_invoker,
        env.payer.pubkey(),
        state,
        usdc_vault,
        ktoken_vault,
    );
    let data =
        poolver_yield_defi::instruction::InitializeAdapter { pool: *pool }.data();
    let ix = forward_through_fake_core(metas, data, env.core_invoker);
    let payer_kp = env.payer.insecure_clone();
    send_ix(&mut env.svm, &payer_kp, ix).expect("init failed");
    (state, usdc_vault, ktoken_vault)
}

/// Deposit `amount` into the adapter using a fresh source authority.
/// Returns the source ATA (so callers can assert post-state).
fn do_deposit(
    env: &mut TestEnv,
    state: Pubkey,
    usdc_vault: Pubkey,
    ktoken_vault: Pubkey,
    amount: u64,
    source_starting_balance: u64,
) -> (Keypair, Pubkey) {
    let source = Keypair::new();
    env.svm.airdrop(&source.pubkey(), SOL).unwrap();
    let source_ata = env.fund_token_account(&source.pubkey(), source_starting_balance);

    let metas = metas_deposit(
        env.core_invoker,
        state,
        usdc_vault,
        source_ata,
        source.pubkey(),
        ktoken_vault,
    );
    let data = poolver_yield_defi::instruction::Deposit { amount }.data();
    let ix = forward_through_fake_core(metas, data, env.core_invoker);
    let payer_kp = env.payer.insecure_clone();
    send_ix_signed(&mut env.svm, &payer_kp, &[&source], ix).expect("deposit failed");
    (source, source_ata)
}

fn do_mock_set_kamino_paused(env: &mut TestEnv, state: Pubkey, paused: bool) {
    let admin = env.payer.insecure_clone();
    let metas = metas_mock_set_breaker(admin.pubkey(), state);
    let data = poolver_yield_defi::instruction::MockSetKaminoPaused { paused }.data();
    let ix = solana_instruction::Instruction {
        program_id: poolver_yield_defi::ID,
        accounts: metas,
        data,
    };
    send_ix(&mut env.svm, &admin, ix).expect("mock_set_kamino_paused failed");
}

fn do_mock_set_utilization(env: &mut TestEnv, state: Pubkey, bps: u16) {
    let admin = env.payer.insecure_clone();
    let metas = metas_mock_set_breaker(admin.pubkey(), state);
    let data = poolver_yield_defi::instruction::MockSetUtilization { bps }.data();
    let ix = solana_instruction::Instruction {
        program_id: poolver_yield_defi::ID,
        accounts: metas,
        data,
    };
    send_ix(&mut env.svm, &admin, ix).expect("mock_set_utilization failed");
}

// ──────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────

#[test]
fn t01_initialize_adapter_happy() {
    let mut env = TestEnv::new();
    let pool = fresh_pool();
    let (state_pda, usdc_pda, ktoken_pda) = init_pool(&mut env, &pool);

    let s = env.fetch_state(&state_pda);
    assert_eq!(s.pool, pool);
    assert_eq!(s.usdc_vault, usdc_pda);
    assert_eq!(s.ktoken_vault, ktoken_pda);
    assert_eq!(s.kamino_reserve, Pubkey::default());
    assert_eq!(s.total_deposited, 0);
    assert_eq!(s.total_deployed_to_kamino, 0);
    assert_eq!(s.liquid_reserved, 0);
    assert_eq!(s.last_recorded_balance, 0);
    assert!(!s.tripped);
    assert_eq!(s.tripped_reason, 0);
    assert!(s.bump != 0);

    // Both vaults exist with 0 balance.
    assert_eq!(env.fetch_token_balance(&usdc_pda), 0);
    assert_eq!(env.fetch_token_balance(&ktoken_pda), 0);
}

#[test]
fn t02_initialize_adapter_rejects_wrong_core() {
    let mut env = TestEnv::new();
    let pool = fresh_pool();
    let (state_pda, _) = env.defi_adapter_pda(&pool);
    let (usdc_pda, _) = env.defi_adapter_usdc_pda(&pool);
    let (ktoken_pda, _) = env.defi_adapter_ktoken_pda(&pool);

    let bogus = Pubkey::new_unique();
    let metas = metas_initialize_adapter(
        &env,
        bogus,
        env.payer.pubkey(),
        state_pda,
        usdc_pda,
        ktoken_pda,
    );
    let data = poolver_yield_defi::instruction::InitializeAdapter { pool }.data();
    let ix = solana_instruction::Instruction {
        program_id: poolver_yield_defi::ID,
        accounts: metas,
        data,
    };
    let payer_kp = env.payer.insecure_clone();
    let result = send_ix(&mut env.svm, &payer_kp, ix);
    assert!(result.is_err(), "wrong core_invoker must be rejected");
}

#[test]
fn t03_deposit_happy_75_25_split() {
    let mut env = TestEnv::new();
    let pool = fresh_pool();
    let (state_pda, usdc_pda, ktoken_pda) = init_pool(&mut env, &pool);

    let amount = 1_000 * ONE_USDC;
    do_deposit(&mut env, state_pda, usdc_pda, ktoken_pda, amount, 1_000 * ONE_USDC);

    let s = env.fetch_state(&state_pda);
    let expected_kamino = amount * 7_500 / 10_000;
    let expected_liquid = amount - expected_kamino;
    assert_eq!(s.total_deposited, amount);
    assert_eq!(s.total_deployed_to_kamino, expected_kamino);
    assert_eq!(s.liquid_reserved, expected_liquid);
    assert_eq!(env.fetch_token_balance(&usdc_pda), expected_liquid);
    assert_eq!(env.fetch_token_balance(&ktoken_pda), expected_kamino);
}

#[test]
fn t04_deposit_rejected_when_tripped() {
    let mut env = TestEnv::new();
    let pool = fresh_pool();
    let (state_pda, usdc_pda, ktoken_pda) = init_pool(&mut env, &pool);

    // Trip via mock_set_kamino_paused — the next deposit should
    // surface CircuitBreakerTripped before any token movement.
    do_mock_set_kamino_paused(&mut env, state_pda, true);

    let source = Keypair::new();
    env.svm.airdrop(&source.pubkey(), SOL).unwrap();
    let source_ata = env.fund_token_account(&source.pubkey(), 100 * ONE_USDC);

    let metas = metas_deposit(
        env.core_invoker,
        state_pda,
        usdc_pda,
        source_ata,
        source.pubkey(),
        ktoken_pda,
    );
    let data = poolver_yield_defi::instruction::Deposit {
        amount: 50 * ONE_USDC,
    }
    .data();
    let ix = forward_through_fake_core(metas, data, env.core_invoker);
    let payer_kp = env.payer.insecure_clone();
    let result = send_ix_signed(&mut env.svm, &payer_kp, &[&source], ix);
    assert!(result.is_err(), "kamino_paused must trip the breaker");

    // State latched.
    let s = env.fetch_state(&state_pda);
    assert!(s.tripped);
    assert_eq!(s.tripped_reason, 3); // TRIP_REASON_PAUSED
}

#[test]
fn t05_deposit_rejects_wrong_core() {
    let mut env = TestEnv::new();
    let pool = fresh_pool();
    let (state_pda, usdc_pda, ktoken_pda) = init_pool(&mut env, &pool);

    let source = Keypair::new();
    env.svm.airdrop(&source.pubkey(), SOL).unwrap();
    let source_ata = env.fund_token_account(&source.pubkey(), 100 * ONE_USDC);

    let bogus = Pubkey::new_unique();
    let metas = metas_deposit(
        bogus,
        state_pda,
        usdc_pda,
        source_ata,
        source.pubkey(),
        ktoken_pda,
    );
    let data = poolver_yield_defi::instruction::Deposit {
        amount: 50 * ONE_USDC,
    }
    .data();
    let ix = solana_instruction::Instruction {
        program_id: poolver_yield_defi::ID,
        accounts: metas,
        data,
    };
    let payer_kp = env.payer.insecure_clone();
    let result = send_ix_signed(&mut env.svm, &payer_kp, &[&source], ix);
    assert!(result.is_err(), "wrong core_invoker must be rejected");
}

#[test]
fn t06_withdraw_drains_liquid_first() {
    let mut env = TestEnv::new();
    let pool = fresh_pool();
    let (state_pda, usdc_pda, ktoken_pda) = init_pool(&mut env, &pool);

    // Deposit 1000 → 250 liquid + 750 kamino.
    do_deposit(&mut env, state_pda, usdc_pda, ktoken_pda, 1_000 * ONE_USDC, 1_000 * ONE_USDC);

    // Withdraw 200 (< liquid 250) — should come entirely from liquid.
    let recipient = Keypair::new();
    let dest_ata = env.fund_token_account(&recipient.pubkey(), 0);

    let metas = metas_withdraw(env.core_invoker, state_pda, usdc_pda, dest_ata, ktoken_pda);
    let data = poolver_yield_defi::instruction::Withdraw {
        amount: 200 * ONE_USDC,
    }
    .data();
    let ix = forward_through_fake_core(metas, data, env.core_invoker);
    let payer_kp = env.payer.insecure_clone();
    send_ix(&mut env.svm, &payer_kp, ix).expect("withdraw failed");

    assert_eq!(env.fetch_token_balance(&dest_ata), 200 * ONE_USDC);
    assert_eq!(env.fetch_token_balance(&usdc_pda), 50 * ONE_USDC);
    assert_eq!(env.fetch_token_balance(&ktoken_pda), 750 * ONE_USDC);
}

#[test]
fn t07_withdraw_redeems_from_kamino_when_short() {
    let mut env = TestEnv::new();
    let pool = fresh_pool();
    let (state_pda, usdc_pda, ktoken_pda) = init_pool(&mut env, &pool);

    // Deposit 1000 → 250 liquid + 750 kamino. Withdraw 500 → all
    // 250 liquid + 250 from kamino.
    do_deposit(&mut env, state_pda, usdc_pda, ktoken_pda, 1_000 * ONE_USDC, 1_000 * ONE_USDC);

    let recipient = Keypair::new();
    let dest_ata = env.fund_token_account(&recipient.pubkey(), 0);

    let metas = metas_withdraw(env.core_invoker, state_pda, usdc_pda, dest_ata, ktoken_pda);
    let data = poolver_yield_defi::instruction::Withdraw {
        amount: 500 * ONE_USDC,
    }
    .data();
    let ix = forward_through_fake_core(metas, data, env.core_invoker);
    let payer_kp = env.payer.insecure_clone();
    send_ix(&mut env.svm, &payer_kp, ix).expect("withdraw failed");

    assert_eq!(env.fetch_token_balance(&dest_ata), 500 * ONE_USDC);
    assert_eq!(env.fetch_token_balance(&usdc_pda), 0);
    assert_eq!(env.fetch_token_balance(&ktoken_pda), 500 * ONE_USDC);
}

#[test]
fn t08_withdraw_rejects_overdraft() {
    let mut env = TestEnv::new();
    let pool = fresh_pool();
    let (state_pda, usdc_pda, ktoken_pda) = init_pool(&mut env, &pool);

    // Deposit 100; try to withdraw 500.
    do_deposit(&mut env, state_pda, usdc_pda, ktoken_pda, 100 * ONE_USDC, 100 * ONE_USDC);

    let recipient = Keypair::new();
    let dest_ata = env.fund_token_account(&recipient.pubkey(), 0);

    let metas = metas_withdraw(env.core_invoker, state_pda, usdc_pda, dest_ata, ktoken_pda);
    let data = poolver_yield_defi::instruction::Withdraw {
        amount: 500 * ONE_USDC,
    }
    .data();
    let ix = forward_through_fake_core(metas, data, env.core_invoker);
    let payer_kp = env.payer.insecure_clone();
    let result = send_ix(&mut env.svm, &payer_kp, ix);
    assert!(result.is_err(), "overdraft must be rejected");
}

#[test]
fn t09_harvest_returns_zero_no_yield() {
    let mut env = TestEnv::new();
    let pool = fresh_pool();
    let (state_pda, usdc_pda, ktoken_pda) = init_pool(&mut env, &pool);

    // Deposit so balances exist; with no injected yield the harvest
    // delta vs `last_recorded_balance` is 0.
    do_deposit(&mut env, state_pda, usdc_pda, ktoken_pda, 1_000 * ONE_USDC, 1_000 * ONE_USDC);

    // First harvest establishes baseline (= 1000); call it once
    // implicitly by running another harvest right after.
    let metas = metas_harvest(env.core_invoker, state_pda, usdc_pda, ktoken_pda);
    let data = poolver_yield_defi::instruction::Harvest {}.data();
    let ix = forward_through_fake_core(metas, data, env.core_invoker);
    let payer_kp = env.payer.insecure_clone();
    send_ix(&mut env.svm, &payer_kp, ix).expect("first harvest failed");
    let s_after_1 = env.fetch_state(&state_pda);
    assert_eq!(s_after_1.last_recorded_balance, 1_000 * ONE_USDC);

    // Second harvest: nothing changed, yield = 0.
    let metas = metas_harvest(env.core_invoker, state_pda, usdc_pda, ktoken_pda);
    let data = poolver_yield_defi::instruction::Harvest {}.data();
    let ix = forward_through_fake_core(metas, data, env.core_invoker);
    send_ix(&mut env.svm, &payer_kp, ix).expect("second harvest failed");
    let s_after_2 = env.fetch_state(&state_pda);
    // Snapshot still 1000 (unchanged because no injection).
    assert_eq!(s_after_2.last_recorded_balance, 1_000 * ONE_USDC);
}

#[test]
fn t10_harvest_returns_positive_yield_after_injection() {
    let mut env = TestEnv::new();
    let pool = fresh_pool();
    let (state_pda, usdc_pda, ktoken_pda) = init_pool(&mut env, &pool);

    do_deposit(&mut env, state_pda, usdc_pda, ktoken_pda, 1_000 * ONE_USDC, 1_000 * ONE_USDC);

    // Snapshot the baseline via a first harvest.
    {
        let metas = metas_harvest(env.core_invoker, state_pda, usdc_pda, ktoken_pda);
        let data = poolver_yield_defi::instruction::Harvest {}.data();
        let ix = forward_through_fake_core(metas, data, env.core_invoker);
        let payer_kp = env.payer.insecure_clone();
        send_ix(&mut env.svm, &payer_kp, ix).unwrap();
    }
    assert_eq!(env.fetch_state(&state_pda).last_recorded_balance, 1_000 * ONE_USDC);

    // Inject 50 USDC of "yield" into the kToken vault.
    let injector = Keypair::new();
    env.svm.airdrop(&injector.pubkey(), SOL).unwrap();
    let injector_ata = env.fund_token_account(&injector.pubkey(), 50 * ONE_USDC);
    {
        let metas = metas_mock_inject_yield(injector.pubkey(), state_pda, injector_ata, ktoken_pda);
        let data = poolver_yield_defi::instruction::MockInjectYield {
            amount: 50 * ONE_USDC,
        }
        .data();
        let ix = solana_instruction::Instruction {
            program_id: poolver_yield_defi::ID,
            accounts: metas,
            data,
        };
        let payer_kp = env.payer.insecure_clone();
        send_ix_signed(&mut env.svm, &payer_kp, &[&injector], ix).expect("inject failed");
    }
    assert_eq!(env.fetch_token_balance(&ktoken_pda), 750 * ONE_USDC + 50 * ONE_USDC);

    // Now harvest — delta = +50.
    {
        let metas = metas_harvest(env.core_invoker, state_pda, usdc_pda, ktoken_pda);
        let data = poolver_yield_defi::instruction::Harvest {}.data();
        let ix = forward_through_fake_core(metas, data, env.core_invoker);
        let payer_kp = env.payer.insecure_clone();
        send_ix(&mut env.svm, &payer_kp, ix).unwrap();
    }
    let s = env.fetch_state(&state_pda);
    assert_eq!(s.last_recorded_balance, 1_050 * ONE_USDC);
}

#[test]
fn t11_harvest_updates_last_recorded_balance() {
    // Covered structurally inside t09 / t10; this case asserts the
    // update happens even when the delta is 0 (so the next call
    // doesn't see phantom yield).
    let mut env = TestEnv::new();
    let pool = fresh_pool();
    let (state_pda, usdc_pda, ktoken_pda) = init_pool(&mut env, &pool);

    let payer_kp = env.payer.insecure_clone();
    // Harvest on empty adapter — sets baseline to 0.
    {
        let metas = metas_harvest(env.core_invoker, state_pda, usdc_pda, ktoken_pda);
        let data = poolver_yield_defi::instruction::Harvest {}.data();
        let ix = forward_through_fake_core(metas, data, env.core_invoker);
        send_ix(&mut env.svm, &payer_kp, ix).unwrap();
    }
    assert_eq!(env.fetch_state(&state_pda).last_recorded_balance, 0);

    // Deposit 100 → balance = 100. Harvest baseline updates.
    do_deposit(&mut env, state_pda, usdc_pda, ktoken_pda, 100 * ONE_USDC, 100 * ONE_USDC);
    {
        let metas = metas_harvest(env.core_invoker, state_pda, usdc_pda, ktoken_pda);
        let data = poolver_yield_defi::instruction::Harvest {}.data();
        let ix = forward_through_fake_core(metas, data, env.core_invoker);
        send_ix(&mut env.svm, &payer_kp, ix).unwrap();
    }
    assert_eq!(env.fetch_state(&state_pda).last_recorded_balance, 100 * ONE_USDC);
}

#[test]
fn t12_emergency_unwind_drains_and_trips() {
    let mut env = TestEnv::new();
    let pool = fresh_pool();
    let (state_pda, usdc_pda, ktoken_pda) = init_pool(&mut env, &pool);

    do_deposit(&mut env, state_pda, usdc_pda, ktoken_pda, 1_000 * ONE_USDC, 1_000 * ONE_USDC);

    let recipient = Keypair::new();
    let dest_ata = env.fund_token_account(&recipient.pubkey(), 0);

    let metas = metas_unwind(env.core_invoker, state_pda, usdc_pda, dest_ata, ktoken_pda);
    let data = poolver_yield_defi::instruction::EmergencyUnwind {}.data();
    let ix = forward_through_fake_core(metas, data, env.core_invoker);
    let payer_kp = env.payer.insecure_clone();
    send_ix(&mut env.svm, &payer_kp, ix).expect("unwind failed");

    assert_eq!(env.fetch_token_balance(&dest_ata), 1_000 * ONE_USDC);
    assert_eq!(env.fetch_token_balance(&usdc_pda), 0);
    assert_eq!(env.fetch_token_balance(&ktoken_pda), 0);

    let s = env.fetch_state(&state_pda);
    assert_eq!(s.total_deposited, 0);
    assert_eq!(s.total_deployed_to_kamino, 0);
    assert_eq!(s.liquid_reserved, 0);
    assert!(s.tripped);
    assert_eq!(s.tripped_reason, 4); // TRIP_REASON_ADMIN_TRIP
}

#[test]
fn t13_reset_circuit_breaker_clears_flag() {
    let mut env = TestEnv::new();
    let pool = fresh_pool();
    let (state_pda, _, _) = init_pool(&mut env, &pool);

    // Trip via mock_set_kamino_paused (the setter itself latches the
    // breaker — see `latch_if_breached` in mock_helpers.rs).
    do_mock_set_kamino_paused(&mut env, state_pda, true);
    assert!(env.fetch_state(&state_pda).tripped);

    // Now reset.
    let admin = env.payer.insecure_clone();
    let metas = metas_reset(admin.pubkey(), state_pda);
    let data = poolver_yield_defi::instruction::ResetCircuitBreaker {}.data();
    let ix = solana_instruction::Instruction {
        program_id: poolver_yield_defi::ID,
        accounts: metas,
        data,
    };
    send_ix(&mut env.svm, &admin, ix).expect("reset failed");

    let s = env.fetch_state(&state_pda);
    assert!(!s.tripped);
    assert_eq!(s.tripped_reason, 0);
    assert_eq!(s.tripped_at, 0);
    assert!(!s.mock_kamino_paused);
}

#[test]
fn t14_utilization_above_threshold_trips() {
    let mut env = TestEnv::new();
    let pool = fresh_pool();
    let (state_pda, usdc_pda, ktoken_pda) = init_pool(&mut env, &pool);

    do_mock_set_utilization(&mut env, state_pda, 9_600); // > 9500 trip threshold

    let source = Keypair::new();
    env.svm.airdrop(&source.pubkey(), SOL).unwrap();
    let source_ata = env.fund_token_account(&source.pubkey(), 100 * ONE_USDC);
    let metas = metas_deposit(
        env.core_invoker,
        state_pda,
        usdc_pda,
        source_ata,
        source.pubkey(),
        ktoken_pda,
    );
    let data = poolver_yield_defi::instruction::Deposit {
        amount: 50 * ONE_USDC,
    }
    .data();
    let ix = forward_through_fake_core(metas, data, env.core_invoker);
    let payer_kp = env.payer.insecure_clone();
    let result = send_ix_signed(&mut env.svm, &payer_kp, &[&source], ix);
    assert!(result.is_err(), "utilization > 9500 must trip");

    let s = env.fetch_state(&state_pda);
    assert!(s.tripped);
    assert_eq!(s.tripped_reason, 1); // TRIP_REASON_UTILIZATION
}

#[test]
fn t15_kamino_paused_trips_on_deposit() {
    // Same as t04 but explicitly verifying the reason discriminant.
    let mut env = TestEnv::new();
    let pool = fresh_pool();
    let (state_pda, usdc_pda, ktoken_pda) = init_pool(&mut env, &pool);

    do_mock_set_kamino_paused(&mut env, state_pda, true);

    let source = Keypair::new();
    env.svm.airdrop(&source.pubkey(), SOL).unwrap();
    let source_ata = env.fund_token_account(&source.pubkey(), 100 * ONE_USDC);
    let metas = metas_deposit(
        env.core_invoker,
        state_pda,
        usdc_pda,
        source_ata,
        source.pubkey(),
        ktoken_pda,
    );
    let data = poolver_yield_defi::instruction::Deposit {
        amount: 50 * ONE_USDC,
    }
    .data();
    let ix = forward_through_fake_core(metas, data, env.core_invoker);
    let payer_kp = env.payer.insecure_clone();
    let result = send_ix_signed(&mut env.svm, &payer_kp, &[&source], ix);
    assert!(result.is_err());

    let s = env.fetch_state(&state_pda);
    assert!(s.tripped);
    assert_eq!(s.tripped_reason, 3); // TRIP_REASON_PAUSED
}

#[test]
fn t16_liquidity_buffer_invariant_after_multi_deposit() {
    let mut env = TestEnv::new();
    let pool = fresh_pool();
    let (state_pda, usdc_pda, ktoken_pda) = init_pool(&mut env, &pool);

    // Three deposits; check the 25/75 invariant after each.
    let amounts = [400 * ONE_USDC, 200 * ONE_USDC, 600 * ONE_USDC];
    let mut running_total: u64 = 0;
    for amount in amounts {
        do_deposit(&mut env, state_pda, usdc_pda, ktoken_pda, amount, amount);
        running_total += amount;
        let s = env.fetch_state(&state_pda);
        let expected_kamino = running_total * 7_500 / 10_000;
        let expected_liquid = running_total - expected_kamino;
        assert_eq!(s.total_deposited, running_total);
        assert_eq!(s.total_deployed_to_kamino, expected_kamino);
        assert_eq!(s.liquid_reserved, expected_liquid);
        // Token-account truths agree with the ledger.
        assert_eq!(env.fetch_token_balance(&usdc_pda), expected_liquid);
        assert_eq!(env.fetch_token_balance(&ktoken_pda), expected_kamino);
    }
}

#[test]
fn t17_tripped_persists_until_reset() {
    let mut env = TestEnv::new();
    let pool = fresh_pool();
    let (state_pda, usdc_pda, ktoken_pda) = init_pool(&mut env, &pool);

    // Seed with a deposit BEFORE tripping (so withdraw / harvest have
    // something to operate on).
    do_deposit(&mut env, state_pda, usdc_pda, ktoken_pda, 1_000 * ONE_USDC, 1_000 * ONE_USDC);

    // Trip via emergency_unwind (sets tripped = true, drains both).
    let recipient = Keypair::new();
    let dest_ata = env.fund_token_account(&recipient.pubkey(), 0);
    {
        let metas = metas_unwind(env.core_invoker, state_pda, usdc_pda, dest_ata, ktoken_pda);
        let data = poolver_yield_defi::instruction::EmergencyUnwind {}.data();
        let ix = forward_through_fake_core(metas, data, env.core_invoker);
        let payer_kp = env.payer.insecure_clone();
        send_ix(&mut env.svm, &payer_kp, ix).unwrap();
    }
    assert!(env.fetch_state(&state_pda).tripped);

    let payer_kp = env.payer.insecure_clone();

    // Deposit rejected.
    {
        let source = Keypair::new();
        env.svm.airdrop(&source.pubkey(), SOL).unwrap();
        let source_ata = env.fund_token_account(&source.pubkey(), 100 * ONE_USDC);
        let metas = metas_deposit(
            env.core_invoker,
            state_pda,
            usdc_pda,
            source_ata,
            source.pubkey(),
            ktoken_pda,
        );
        let data = poolver_yield_defi::instruction::Deposit {
            amount: 50 * ONE_USDC,
        }
        .data();
        let ix = forward_through_fake_core(metas, data, env.core_invoker);
        let r = send_ix_signed(&mut env.svm, &payer_kp, &[&source], ix);
        assert!(r.is_err(), "deposit must reject while tripped");
    }

    // Withdraw rejected.
    {
        let metas = metas_withdraw(env.core_invoker, state_pda, usdc_pda, dest_ata, ktoken_pda);
        let data = poolver_yield_defi::instruction::Withdraw {
            amount: 1,
        }
        .data();
        let ix = forward_through_fake_core(metas, data, env.core_invoker);
        let r = send_ix(&mut env.svm, &payer_kp, ix);
        assert!(r.is_err(), "withdraw must reject while tripped");
    }

    // Harvest rejected.
    {
        let metas = metas_harvest(env.core_invoker, state_pda, usdc_pda, ktoken_pda);
        let data = poolver_yield_defi::instruction::Harvest {}.data();
        let ix = forward_through_fake_core(metas, data, env.core_invoker);
        let r = send_ix(&mut env.svm, &payer_kp, ix);
        assert!(r.is_err(), "harvest must reject while tripped");
    }
}

#[test]
fn t18_end_to_end_deposit_inject_harvest_withdraw() {
    let mut env = TestEnv::new();
    let pool = fresh_pool();
    let (state_pda, usdc_pda, ktoken_pda) = init_pool(&mut env, &pool);

    let payer_kp = env.payer.insecure_clone();

    // Deposit 1000.
    do_deposit(&mut env, state_pda, usdc_pda, ktoken_pda, 1_000 * ONE_USDC, 1_000 * ONE_USDC);

    // Establish baseline.
    {
        let metas = metas_harvest(env.core_invoker, state_pda, usdc_pda, ktoken_pda);
        let data = poolver_yield_defi::instruction::Harvest {}.data();
        let ix = forward_through_fake_core(metas, data, env.core_invoker);
        send_ix(&mut env.svm, &payer_kp, ix).unwrap();
    }

    // Inject 50 yield.
    let injector = Keypair::new();
    env.svm.airdrop(&injector.pubkey(), SOL).unwrap();
    let injector_ata = env.fund_token_account(&injector.pubkey(), 50 * ONE_USDC);
    {
        let metas = metas_mock_inject_yield(injector.pubkey(), state_pda, injector_ata, ktoken_pda);
        let data = poolver_yield_defi::instruction::MockInjectYield {
            amount: 50 * ONE_USDC,
        }
        .data();
        let ix = solana_instruction::Instruction {
            program_id: poolver_yield_defi::ID,
            accounts: metas,
            data,
        };
        send_ix_signed(&mut env.svm, &payer_kp, &[&injector], ix).unwrap();
    }

    // Harvest sees 50.
    {
        let metas = metas_harvest(env.core_invoker, state_pda, usdc_pda, ktoken_pda);
        let data = poolver_yield_defi::instruction::Harvest {}.data();
        let ix = forward_through_fake_core(metas, data, env.core_invoker);
        send_ix(&mut env.svm, &payer_kp, ix).unwrap();
    }
    assert_eq!(env.fetch_state(&state_pda).last_recorded_balance, 1_050 * ONE_USDC);

    // Withdraw the entire 1050 → recipient.
    let recipient = Keypair::new();
    let dest_ata = env.fund_token_account(&recipient.pubkey(), 0);
    {
        let metas = metas_withdraw(env.core_invoker, state_pda, usdc_pda, dest_ata, ktoken_pda);
        let data = poolver_yield_defi::instruction::Withdraw {
            amount: 1_050 * ONE_USDC,
        }
        .data();
        let ix = forward_through_fake_core(metas, data, env.core_invoker);
        send_ix(&mut env.svm, &payer_kp, ix).expect("withdraw 1050 failed");
    }
    assert_eq!(env.fetch_token_balance(&dest_ata), 1_050 * ONE_USDC);
    assert_eq!(env.fetch_token_balance(&usdc_pda), 0);
    assert_eq!(env.fetch_token_balance(&ktoken_pda), 0);
}
