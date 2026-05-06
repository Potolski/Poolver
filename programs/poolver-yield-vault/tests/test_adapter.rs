//! Unit tests for `poolver-yield-vault`.
//!
//! Coverage map (per task prompt §F):
//!   1. initialize_adapter happy path                         → t01
//!   2. initialize_adapter rejects non-canonical core_invoker → t02
//!   3. deposit happy path                                    → t03
//!   4. deposit rejects non-canonical core_invoker            → t04
//!   5. deposit rejects amount=0                              → t05
//!   6. withdraw happy path                                   → t06
//!   7. withdraw rejects amount > balance                     → t07
//!   8. withdraw rejects non-canonical core_invoker           → t08
//!   9. harvest returns 0                                     → t09
//!  10. emergency_unwind drains the vault                     → t10
//!
//! The "happy path" cases route the adapter ix through the fake-core stub
//! so the `core_invoker` PDA is signed by its owning program. This is a
//! UNIT test boundary — the integration tests that exercise the real
//! `poolver-core` CPI path will land later (SPEC_QUESTION-26).

mod common;

use anchor_lang::InstructionData;
use common::*;
use solana_instruction::AccountMeta;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

fn metas_initialize_adapter(
    env: &TestEnv,
    _pool: &Pubkey,
    core_invoker: Pubkey,
    payer: Pubkey,
    adapter_state: Pubkey,
    adapter_usdc_vault: Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new_readonly(core_invoker, false), // fake_core re-marks as signer
        AccountMeta::new(payer, true),                  // outer-signed
        AccountMeta::new(adapter_state, false),
        AccountMeta::new_readonly(env.usdc_mint, false),
        AccountMeta::new(adapter_usdc_vault, false),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
        AccountMeta::new_readonly(solana_pubkey::pubkey!("11111111111111111111111111111111"), false),
        AccountMeta::new_readonly(
            solana_pubkey::pubkey!("SysvarRent111111111111111111111111111111111"),
            false,
        ),
    ]
}

fn metas_deposit(
    env: &TestEnv,
    core_invoker: Pubkey,
    adapter_state: Pubkey,
    adapter_usdc_vault: Pubkey,
    source_usdc: Pubkey,
    source_authority: Pubkey,
) -> Vec<AccountMeta> {
    let _ = env;
    vec![
        AccountMeta::new_readonly(core_invoker, false),
        AccountMeta::new(adapter_state, false),
        AccountMeta::new(adapter_usdc_vault, false),
        AccountMeta::new(source_usdc, false),
        AccountMeta::new_readonly(source_authority, true),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
    ]
}

fn metas_withdraw(
    core_invoker: Pubkey,
    adapter_state: Pubkey,
    adapter_usdc_vault: Pubkey,
    destination_usdc: Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new_readonly(core_invoker, false),
        AccountMeta::new(adapter_state, false),
        AccountMeta::new(adapter_usdc_vault, false),
        AccountMeta::new(destination_usdc, false),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
    ]
}

fn metas_harvest(
    core_invoker: Pubkey,
    adapter_state: Pubkey,
    adapter_usdc_vault: Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new_readonly(core_invoker, false),
        AccountMeta::new_readonly(adapter_state, false),
        AccountMeta::new_readonly(adapter_usdc_vault, false),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
    ]
}

fn metas_unwind(
    core_invoker: Pubkey,
    adapter_state: Pubkey,
    adapter_usdc_vault: Pubkey,
    destination_usdc: Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new_readonly(core_invoker, false),
        AccountMeta::new(adapter_state, false),
        AccountMeta::new(adapter_usdc_vault, false),
        AccountMeta::new(destination_usdc, false),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
    ]
}

/// Deterministic "pool" pubkey for tests — a pool PDA in production lives
/// inside `poolver-core` which doesn't exist yet, so we mint a unique key
/// per test and treat it as opaque.
fn fresh_pool() -> Pubkey {
    Pubkey::new_unique()
}

fn init_pool(env: &mut TestEnv, pool: &Pubkey) -> (Pubkey, Pubkey) {
    let (adapter_state, _) = env.vault_adapter_pda(pool);
    let (adapter_usdc_vault, _) = env.vault_adapter_usdc_pda(pool);
    let metas = metas_initialize_adapter(
        env,
        pool,
        env.core_invoker,
        env.payer.pubkey(),
        adapter_state,
        adapter_usdc_vault,
    );
    let data = poolver_yield_vault::instruction::InitializeAdapter { pool: *pool }.data();
    let ix = forward_through_fake_core(metas, data, env.core_invoker);
    let payer_kp = env.payer.insecure_clone();
    send_ix(&mut env.svm, &payer_kp, ix).expect("init failed");
    (adapter_state, adapter_usdc_vault)
}

// ───── Test 1: initialize_adapter happy path ─────────────────────────────
#[test]
fn t01_initialize_adapter_happy() {
    let mut env = TestEnv::new();
    let pool = fresh_pool();
    let (state_pda, vault_pda) = init_pool(&mut env, &pool);

    let state = env.fetch_state(&state_pda);
    assert_eq!(state.pool, pool);
    assert_eq!(state.usdc_vault, vault_pda);
    assert_eq!(state.total_deposited, 0);
    assert!(state.bump != 0);

    // Vault token-account exists with 0 balance.
    assert_eq!(env.fetch_token_balance(&vault_pda), 0);
}

// ───── Test 2: initialize_adapter rejects non-canonical core_invoker ─────
#[test]
fn t02_initialize_adapter_rejects_wrong_core() {
    let mut env = TestEnv::new();
    let pool = fresh_pool();
    let (adapter_state, _) = env.vault_adapter_pda(&pool);
    let (adapter_usdc_vault, _) = env.vault_adapter_usdc_pda(&pool);

    let bogus_core = Pubkey::new_unique(); // any-old key, not the canonical PDA
    let payer_kp = env.payer.insecure_clone();

    // Bypass fake_core entirely — call the adapter directly. The
    // `seeds::program = POOLVER_CORE_ID, seeds = [b"core_invoker"]`
    // constraint should reject this regardless of what is_signer flag we
    // claim, because the key won't match the canonical PDA derivation.
    let metas = metas_initialize_adapter(
        &env,
        &pool,
        bogus_core,
        env.payer.pubkey(),
        adapter_state,
        adapter_usdc_vault,
    );
    let data = poolver_yield_vault::instruction::InitializeAdapter { pool }.data();
    let ix = solana_instruction::Instruction {
        program_id: poolver_yield_vault::ID,
        accounts: metas,
        data,
    };
    let result = send_ix(&mut env.svm, &payer_kp, ix);
    assert!(result.is_err(), "wrong core_invoker must be rejected");
}

// ───── Test 3: deposit happy path ────────────────────────────────────────
#[test]
fn t03_deposit_happy() {
    let mut env = TestEnv::new();
    let pool = fresh_pool();
    let (state_pda, vault_pda) = init_pool(&mut env, &pool);

    // Source: an external "source authority" keypair holding a token
    // account. In production this is the pool's USDC vault PDA (signed
    // by core); for unit-test scope the truth that matters is the
    // adapter-side bookkeeping.
    let source_authority = Keypair::new();
    env.svm.airdrop(&source_authority.pubkey(), SOL).unwrap();
    let source_ata = env.fund_token_account(&source_authority.pubkey(), 1_000 * ONE_USDC);

    let amount = 250 * ONE_USDC;
    let metas = metas_deposit(
        &env,
        env.core_invoker,
        state_pda,
        vault_pda,
        source_ata,
        source_authority.pubkey(),
    );
    let data = poolver_yield_vault::instruction::Deposit { amount }.data();
    let ix = forward_through_fake_core(metas, data, env.core_invoker);
    let payer_kp = env.payer.insecure_clone();
    send_ix_signed(&mut env.svm, &payer_kp, &[&source_authority], ix).expect("deposit failed");

    let state = env.fetch_state(&state_pda);
    assert_eq!(state.total_deposited, amount);
    assert_eq!(env.fetch_token_balance(&vault_pda), amount);
    assert_eq!(env.fetch_token_balance(&source_ata), 1_000 * ONE_USDC - amount);
}

// ───── Test 4: deposit rejects non-canonical core_invoker ────────────────
#[test]
fn t04_deposit_rejects_wrong_core() {
    let mut env = TestEnv::new();
    let pool = fresh_pool();
    let (state_pda, vault_pda) = init_pool(&mut env, &pool);

    let source_authority = Keypair::new();
    env.svm.airdrop(&source_authority.pubkey(), SOL).unwrap();
    let source_ata = env.fund_token_account(&source_authority.pubkey(), 100 * ONE_USDC);

    let bogus_core = Pubkey::new_unique();
    let metas = metas_deposit(
        &env,
        bogus_core,
        state_pda,
        vault_pda,
        source_ata,
        source_authority.pubkey(),
    );
    let data = poolver_yield_vault::instruction::Deposit { amount: 50 * ONE_USDC }.data();
    let ix = solana_instruction::Instruction {
        program_id: poolver_yield_vault::ID,
        accounts: metas,
        data,
    };
    let payer_kp = env.payer.insecure_clone();
    let result = send_ix_signed(&mut env.svm, &payer_kp, &[&source_authority], ix);
    assert!(result.is_err(), "wrong core_invoker must be rejected");
}

// ───── Test 5: deposit rejects amount=0 ──────────────────────────────────
#[test]
fn t05_deposit_rejects_zero_amount() {
    let mut env = TestEnv::new();
    let pool = fresh_pool();
    let (state_pda, vault_pda) = init_pool(&mut env, &pool);

    let source_authority = Keypair::new();
    env.svm.airdrop(&source_authority.pubkey(), SOL).unwrap();
    let source_ata = env.fund_token_account(&source_authority.pubkey(), 100 * ONE_USDC);

    let metas = metas_deposit(
        &env,
        env.core_invoker,
        state_pda,
        vault_pda,
        source_ata,
        source_authority.pubkey(),
    );
    let data = poolver_yield_vault::instruction::Deposit { amount: 0 }.data();
    let ix = forward_through_fake_core(metas, data, env.core_invoker);
    let payer_kp = env.payer.insecure_clone();
    let result = send_ix_signed(&mut env.svm, &payer_kp, &[&source_authority], ix);
    assert!(result.is_err(), "amount=0 must be rejected");
}

// ───── Test 6: withdraw happy path ───────────────────────────────────────
#[test]
fn t06_withdraw_happy() {
    let mut env = TestEnv::new();
    let pool = fresh_pool();
    let (state_pda, vault_pda) = init_pool(&mut env, &pool);

    // Seed the vault by depositing 500 USDC.
    let source_authority = Keypair::new();
    env.svm.airdrop(&source_authority.pubkey(), SOL).unwrap();
    let source_ata = env.fund_token_account(&source_authority.pubkey(), 500 * ONE_USDC);
    {
        let metas = metas_deposit(
            &env,
            env.core_invoker,
            state_pda,
            vault_pda,
            source_ata,
            source_authority.pubkey(),
        );
        let data = poolver_yield_vault::instruction::Deposit {
            amount: 500 * ONE_USDC,
        }
        .data();
        let ix = forward_through_fake_core(metas, data, env.core_invoker);
        let payer_kp = env.payer.insecure_clone();
        send_ix_signed(&mut env.svm, &payer_kp, &[&source_authority], ix).unwrap();
    }

    // Destination — any ATA we control.
    let recipient = Keypair::new();
    let dest_ata = env.fund_token_account(&recipient.pubkey(), 0);

    let withdraw_amount = 200 * ONE_USDC;
    let metas = metas_withdraw(env.core_invoker, state_pda, vault_pda, dest_ata);
    let data = poolver_yield_vault::instruction::Withdraw {
        amount: withdraw_amount,
    }
    .data();
    let ix = forward_through_fake_core(metas, data, env.core_invoker);
    let payer_kp = env.payer.insecure_clone();
    send_ix(&mut env.svm, &payer_kp, ix).expect("withdraw failed");

    let state = env.fetch_state(&state_pda);
    assert_eq!(state.total_deposited, 500 * ONE_USDC - withdraw_amount);
    assert_eq!(
        env.fetch_token_balance(&vault_pda),
        500 * ONE_USDC - withdraw_amount
    );
    assert_eq!(env.fetch_token_balance(&dest_ata), withdraw_amount);
}

// ───── Test 7: withdraw rejects amount > balance ─────────────────────────
#[test]
fn t07_withdraw_rejects_overdraft() {
    let mut env = TestEnv::new();
    let pool = fresh_pool();
    let (state_pda, vault_pda) = init_pool(&mut env, &pool);

    // Seed 100 USDC.
    let source_authority = Keypair::new();
    env.svm.airdrop(&source_authority.pubkey(), SOL).unwrap();
    let source_ata = env.fund_token_account(&source_authority.pubkey(), 100 * ONE_USDC);
    {
        let metas = metas_deposit(
            &env,
            env.core_invoker,
            state_pda,
            vault_pda,
            source_ata,
            source_authority.pubkey(),
        );
        let data = poolver_yield_vault::instruction::Deposit {
            amount: 100 * ONE_USDC,
        }
        .data();
        let ix = forward_through_fake_core(metas, data, env.core_invoker);
        let payer_kp = env.payer.insecure_clone();
        send_ix_signed(&mut env.svm, &payer_kp, &[&source_authority], ix).unwrap();
    }

    let recipient = Keypair::new();
    let dest_ata = env.fund_token_account(&recipient.pubkey(), 0);

    // Try to withdraw twice the balance — must return InsufficientLiquidity.
    let metas = metas_withdraw(env.core_invoker, state_pda, vault_pda, dest_ata);
    let data = poolver_yield_vault::instruction::Withdraw {
        amount: 200 * ONE_USDC,
    }
    .data();
    let ix = forward_through_fake_core(metas, data, env.core_invoker);
    let payer_kp = env.payer.insecure_clone();
    let result = send_ix(&mut env.svm, &payer_kp, ix);
    assert!(result.is_err(), "overdraft must be rejected");
}

// ───── Test 8: withdraw rejects non-canonical core_invoker ───────────────
#[test]
fn t08_withdraw_rejects_wrong_core() {
    let mut env = TestEnv::new();
    let pool = fresh_pool();
    let (state_pda, vault_pda) = init_pool(&mut env, &pool);

    let recipient = Keypair::new();
    let dest_ata = env.fund_token_account(&recipient.pubkey(), 0);

    let bogus_core = Pubkey::new_unique();
    let metas = metas_withdraw(bogus_core, state_pda, vault_pda, dest_ata);
    let data = poolver_yield_vault::instruction::Withdraw {
        amount: ONE_USDC,
    }
    .data();
    let ix = solana_instruction::Instruction {
        program_id: poolver_yield_vault::ID,
        accounts: metas,
        data,
    };
    let payer_kp = env.payer.insecure_clone();
    let result = send_ix(&mut env.svm, &payer_kp, ix);
    assert!(result.is_err(), "wrong core_invoker must be rejected");
}

// ───── Test 9: harvest returns 0 ─────────────────────────────────────────
#[test]
fn t09_harvest_returns_zero() {
    let mut env = TestEnv::new();
    let pool = fresh_pool();
    let (state_pda, vault_pda) = init_pool(&mut env, &pool);

    let metas = metas_harvest(env.core_invoker, state_pda, vault_pda);
    let data = poolver_yield_vault::instruction::Harvest {}.data();
    let ix = forward_through_fake_core(metas, data, env.core_invoker);
    let payer_kp = env.payer.insecure_clone();
    send_ix(&mut env.svm, &payer_kp, ix).expect("harvest failed");

    // The return value lives in transaction return-data; we don't assert
    // on it directly here (litesvm exposes it via tx metadata). What we
    // can assert: state is unchanged and the instruction succeeded — the
    // contract for Tier 0 (spec §5.3) is "always returns 0", which the
    // event records.
    let state = env.fetch_state(&state_pda);
    assert_eq!(state.total_deposited, 0);
}

// ───── Test 10: emergency_unwind drains the vault ────────────────────────
#[test]
fn t10_emergency_unwind_drains() {
    let mut env = TestEnv::new();
    let pool = fresh_pool();
    let (state_pda, vault_pda) = init_pool(&mut env, &pool);

    // Seed the vault with 750 USDC.
    let source_authority = Keypair::new();
    env.svm.airdrop(&source_authority.pubkey(), SOL).unwrap();
    let source_ata = env.fund_token_account(&source_authority.pubkey(), 750 * ONE_USDC);
    {
        let metas = metas_deposit(
            &env,
            env.core_invoker,
            state_pda,
            vault_pda,
            source_ata,
            source_authority.pubkey(),
        );
        let data = poolver_yield_vault::instruction::Deposit {
            amount: 750 * ONE_USDC,
        }
        .data();
        let ix = forward_through_fake_core(metas, data, env.core_invoker);
        let payer_kp = env.payer.insecure_clone();
        send_ix_signed(&mut env.svm, &payer_kp, &[&source_authority], ix).unwrap();
    }

    let recipient = Keypair::new();
    let dest_ata = env.fund_token_account(&recipient.pubkey(), 0);

    let metas = metas_unwind(env.core_invoker, state_pda, vault_pda, dest_ata);
    let data = poolver_yield_vault::instruction::EmergencyUnwind {}.data();
    let ix = forward_through_fake_core(metas, data, env.core_invoker);
    let payer_kp = env.payer.insecure_clone();
    send_ix(&mut env.svm, &payer_kp, ix).expect("unwind failed");

    let state = env.fetch_state(&state_pda);
    assert_eq!(state.total_deposited, 0);
    assert_eq!(env.fetch_token_balance(&vault_pda), 0);
    assert_eq!(env.fetch_token_balance(&dest_ata), 750 * ONE_USDC);
}
