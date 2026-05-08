//! Step-5 integration tests for `poolver-core`: `contribute` +
//! `advance_month`.
//!
//! Coverage map (per task prompt §Tests):
//!   1.  contribute happy path (month 2): fee splits, vault balances,
//!       paid_months bit                                              → t30
//!   2.  contribute rejected when caller is not a participant          → t31
//!   3.  contribute rejected when already paid this month              → t32
//!   4.  contribute rejected when protocol is paused                   → t33
//!       (pause is admin-only step-13; we hack the flag in via
//!        ProtocolConfig serialize/set_account.)
//!   5.  contribute rejected when pool is complete                     → t34
//!   6.  contribute rejected when outside month window                 → t35
//!   7.  contribute rejected when defaulted                            → t36
//!   8.  contribute updates user_reputation.total_contributed_lifetime → t37
//!   9.  contribute with has_won releases collateral                   → t38
//!   10. advance_month happy path: month++, started_at + windows reset → t39
//!   11. advance_month rejected when month duration not elapsed        → t40
//!   12. advance_month from month 12 → completes pool                  → t41
//!   13. advance_month rejected when pool already complete             → t42
//!   14. End-to-end 12-month flow with all 12 participants             → t43
//!   15. Reserve isolation: wrong-tier reserve_fund fails seeds        → t44

#![cfg(feature = "mock-kyc")]

mod common;

use anchor_lang::{AccountDeserialize, AccountSerialize, InstructionData};
use common::*;
use solana_account::Account;
use solana_clock::Clock;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_program_pack::Pack;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

// ───── Account-meta builders (mirror Anchor field order) ─────────────────

fn metas_initialize_protocol(
    admin: Pubkey,
    protocol_config: Pubkey,
    usdc_mint: Pubkey,
    protocol_fee_vault: Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new(admin, true),
        AccountMeta::new(protocol_config, false),
        AccountMeta::new_readonly(usdc_mint, false),
        AccountMeta::new(protocol_fee_vault, false),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
        AccountMeta::new_readonly(RENT_SYSVAR, false),
    ]
}

fn metas_mock_issue_kyc(
    admin: Pubkey,
    protocol_config: Pubkey,
    user_pubkey: Pubkey,
    attestation: Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new(admin, true),
        AccountMeta::new_readonly(protocol_config, false),
        AccountMeta::new_readonly(user_pubkey, false),
        AccountMeta::new(attestation, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
    ]
}

fn metas_initialize_user_reputation(user: Pubkey, reputation: Pubkey) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new(user, true),
        AccountMeta::new(reputation, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
    ]
}

#[allow(clippy::too_many_arguments)]
fn metas_create_pool(
    creator: Pubkey,
    protocol_config: Pubkey,
    creator_kyc: Pubkey,
    creator_reputation: Pubkey,
    pool: Pubkey,
    usdc_mint: Pubkey,
    pool_usdc_vault: Pubkey,
    collateral_vault: Pubkey,
    bid_stake_vault: Pubkey,
    core_invoker: Pubkey,
    adapter_state: Pubkey,
    adapter_usdc_vault: Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new(creator, true),
        AccountMeta::new_readonly(protocol_config, false),
        AccountMeta::new_readonly(creator_kyc, false),
        AccountMeta::new_readonly(creator_reputation, false),
        AccountMeta::new(pool, false),
        AccountMeta::new_readonly(usdc_mint, false),
        AccountMeta::new(pool_usdc_vault, false),
        AccountMeta::new(collateral_vault, false),
        AccountMeta::new(bid_stake_vault, false),
        AccountMeta::new_readonly(core_invoker, false),
        AccountMeta::new(adapter_state, false),
        AccountMeta::new(adapter_usdc_vault, false),
        AccountMeta::new_readonly(poolver_yield_vault::ID, false),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
        AccountMeta::new_readonly(RENT_SYSVAR, false),
    ]
}

#[allow(clippy::too_many_arguments)]
fn metas_join_pool(
    user: Pubkey,
    protocol_config: Pubkey,
    user_kyc: Pubkey,
    user_reputation: Pubkey,
    pool: Pubkey,
    participant: Pubkey,
    user_usdc: Pubkey,
    pool_usdc_vault: Pubkey,
    collateral_vault: Pubkey,
    protocol_fee_vault: Pubkey,
    core_invoker: Pubkey,
    reserve_fund: Pubkey,
    reserve_usdc_vault: Pubkey,
    adapter_state: Pubkey,
    adapter_usdc_vault: Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new(user, true),
        AccountMeta::new_readonly(protocol_config, false),
        AccountMeta::new_readonly(user_kyc, false),
        AccountMeta::new(user_reputation, false),
        AccountMeta::new(pool, false),
        AccountMeta::new(participant, false),
        AccountMeta::new(user_usdc, false),
        AccountMeta::new(pool_usdc_vault, false),
        AccountMeta::new(collateral_vault, false),
        AccountMeta::new(protocol_fee_vault, false),
        AccountMeta::new_readonly(core_invoker, false),
        AccountMeta::new(reserve_fund, false),
        AccountMeta::new(reserve_usdc_vault, false),
        AccountMeta::new_readonly(poolver_reserve::ID, false),
        AccountMeta::new(adapter_state, false),
        AccountMeta::new(adapter_usdc_vault, false),
        AccountMeta::new_readonly(poolver_yield_vault::ID, false),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
        AccountMeta::new_readonly(RENT_SYSVAR, false),
    ]
}

#[allow(clippy::too_many_arguments)]
fn metas_contribute(
    user: Pubkey,
    protocol_config: Pubkey,
    user_reputation: Pubkey,
    pool: Pubkey,
    participant: Pubkey,
    user_usdc: Pubkey,
    pool_usdc_vault: Pubkey,
    collateral_vault: Pubkey,
    protocol_fee_vault: Pubkey,
    core_invoker: Pubkey,
    reserve_fund: Pubkey,
    reserve_usdc_vault: Pubkey,
    adapter_state: Pubkey,
    adapter_usdc_vault: Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new(user, true),
        AccountMeta::new_readonly(protocol_config, false),
        AccountMeta::new(user_reputation, false),
        AccountMeta::new(pool, false),
        AccountMeta::new(participant, false),
        AccountMeta::new(user_usdc, false),
        AccountMeta::new(pool_usdc_vault, false),
        AccountMeta::new(collateral_vault, false),
        AccountMeta::new(protocol_fee_vault, false),
        AccountMeta::new_readonly(core_invoker, false),
        AccountMeta::new(reserve_fund, false),
        AccountMeta::new(reserve_usdc_vault, false),
        AccountMeta::new_readonly(poolver_reserve::ID, false),
        AccountMeta::new(adapter_state, false),
        AccountMeta::new(adapter_usdc_vault, false),
        AccountMeta::new_readonly(poolver_yield_vault::ID, false),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
    ]
}

fn metas_advance_month(caller: Pubkey, protocol_config: Pubkey, pool: Pubkey) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new_readonly(caller, true),
        AccountMeta::new_readonly(protocol_config, false),
        AccountMeta::new(pool, false),
    ]
}

fn build_ix(metas: Vec<AccountMeta>, data: Vec<u8>) -> Instruction {
    Instruction {
        program_id: poolver_core::ID,
        accounts: metas,
        data,
    }
}

// ───── Reserve init helper (same as test_core.rs) ────────────────────────

fn metas_reserve_initialize_reserve(
    admin: Pubkey,
    reserve_fund: Pubkey,
    usdc_mint: Pubkey,
    reserve_usdc_vault: Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new(admin, true),
        AccountMeta::new(reserve_fund, false),
        AccountMeta::new_readonly(usdc_mint, false),
        AccountMeta::new(reserve_usdc_vault, false),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
        AccountMeta::new_readonly(RENT_SYSVAR, false),
    ]
}

fn init_reserve_for(env: &mut TestEnv, tier: Tier) {
    let (reserve_fund, _) = env.reserve_fund_pda(tier);
    let (reserve_usdc_vault, _) = env.reserve_vault_pda(tier);
    let metas = metas_reserve_initialize_reserve(
        env.admin.pubkey(),
        reserve_fund,
        env.usdc_mint,
        reserve_usdc_vault,
    );
    let ix = Instruction {
        program_id: poolver_reserve::ID,
        accounts: metas,
        data: poolver_reserve::instruction::InitializeReserve {
            tier: tier.to_reserve_tier(),
        }
        .data(),
    };
    let admin = env.admin.insecure_clone();
    send_ix(&mut env.svm, &admin, ix).expect("init reserve");
}

// ───── High-level helpers (mirror those in test_core.rs) ─────────────────

fn init_protocol(env: &mut TestEnv) -> (Pubkey, Pubkey) {
    let (config_pda, _) = env.protocol_config_pda();
    let (fee_vault_pda, _) = env.protocol_fee_vault_pda();
    let metas =
        metas_initialize_protocol(env.admin.pubkey(), config_pda, env.usdc_mint, fee_vault_pda);
    let ix = build_ix(
        metas,
        poolver_core::instruction::InitializeProtocol {}.data(),
    );
    let admin = env.admin.insecure_clone();
    send_ix(&mut env.svm, &admin, ix).expect("init protocol");
    (config_pda, fee_vault_pda)
}

fn issue_mock_kyc(env: &mut TestEnv, user: &Pubkey, level: KycLevel) {
    let (config_pda, _) = env.protocol_config_pda();
    let (att_pda, _) = env.kyc_pda(user);
    let metas = metas_mock_issue_kyc(env.admin.pubkey(), config_pda, *user, att_pda);
    let ix = build_ix(
        metas,
        poolver_core::instruction::MockIssueKyc { user: *user, level }.data(),
    );
    let admin = env.admin.insecure_clone();
    send_ix(&mut env.svm, &admin, ix).expect("issue mock kyc");
}

fn init_reputation(env: &mut TestEnv, user_kp: &Keypair) {
    let (rep_pda, _) = env.reputation_pda(&user_kp.pubkey());
    let metas = metas_initialize_user_reputation(user_kp.pubkey(), rep_pda);
    let ix = build_ix(
        metas,
        poolver_core::instruction::InitializeUserReputation {}.data(),
    );
    send_ix(&mut env.svm, user_kp, ix).expect("init reputation");
}

fn bootstrap_with_creator(env: &mut TestEnv) -> Keypair {
    init_protocol(env);
    init_reserve_for(env, Tier::Vault);
    init_reserve_for(env, Tier::DeFi);

    let creator = Keypair::new();
    env.svm.airdrop(&creator.pubkey(), 100 * SOL).unwrap();
    issue_mock_kyc(env, &creator.pubkey(), KycLevel::Light);
    init_reputation(env, &creator);
    creator
}

fn create_pool_for(
    env: &mut TestEnv,
    creator: &Keypair,
    pool_id: u64,
    tier: Tier,
    contribution: u64,
    month_duration: Option<i64>,
) -> Result<Pubkey, String> {
    let (pool_pda, _) = env.pool_pda(&creator.pubkey(), pool_id);
    let (config_pda, _) = env.protocol_config_pda();
    let (creator_kyc, _) = env.kyc_pda(&creator.pubkey());
    let (creator_rep, _) = env.reputation_pda(&creator.pubkey());
    let (pool_usdc_vault, _) = env.pool_usdc_vault_pda(&pool_pda);
    let (collat_vault, _) = env.collateral_vault_pda(&pool_pda);
    let (bid_stake_vault, _) = env.bid_stake_vault_pda(&pool_pda);
    let (adapter_state, _) = env.vault_adapter_pda(&pool_pda);
    let (adapter_usdc, _) = env.vault_adapter_usdc_pda(&pool_pda);

    let metas = metas_create_pool(
        creator.pubkey(),
        config_pda,
        creator_kyc,
        creator_rep,
        pool_pda,
        env.usdc_mint,
        pool_usdc_vault,
        collat_vault,
        bid_stake_vault,
        env.core_invoker,
        adapter_state,
        adapter_usdc,
    );
    let ix = build_ix(
        metas,
        poolver_core::instruction::CreatePool {
            pool_id,
            tier,
            contribution_amount: contribution,
            month_duration_seconds: month_duration,
        }
        .data(),
    );
    send_ix(&mut env.svm, creator, ix).map(|_| pool_pda)
}

fn fully_set_up_user(env: &mut TestEnv, balance: u64) -> (Keypair, Pubkey) {
    let user = Keypair::new();
    env.svm.airdrop(&user.pubkey(), 100 * SOL).unwrap();
    issue_mock_kyc(env, &user.pubkey(), KycLevel::Light);
    init_reputation(env, &user);
    let ata = env.fund_token_account(&user.pubkey(), balance);
    (user, ata)
}

fn join_pool_for(
    env: &mut TestEnv,
    user: &Keypair,
    user_usdc: Pubkey,
    pool: Pubkey,
    tier: Tier,
) -> Result<(), String> {
    let (config_pda, _) = env.protocol_config_pda();
    let (user_kyc, _) = env.kyc_pda(&user.pubkey());
    let (user_rep, _) = env.reputation_pda(&user.pubkey());
    let (participant, _) = env.participant_pda(&pool, &user.pubkey());
    let (pool_usdc_vault, _) = env.pool_usdc_vault_pda(&pool);
    let (collat_vault, _) = env.collateral_vault_pda(&pool);
    let (fee_vault, _) = env.protocol_fee_vault_pda();
    let (reserve_fund, _) = env.reserve_fund_pda(tier);
    let (reserve_usdc, _) = env.reserve_vault_pda(tier);
    let (adapter_state, _) = env.vault_adapter_pda(&pool);
    let (adapter_usdc, _) = env.vault_adapter_usdc_pda(&pool);

    let metas = metas_join_pool(
        user.pubkey(),
        config_pda,
        user_kyc,
        user_rep,
        pool,
        participant,
        user_usdc,
        pool_usdc_vault,
        collat_vault,
        fee_vault,
        env.core_invoker,
        reserve_fund,
        reserve_usdc,
        adapter_state,
        adapter_usdc,
    );
    let ix = build_ix(metas, poolver_core::instruction::JoinPool {}.data());
    send_ix(&mut env.svm, user, ix)
}

#[allow(clippy::too_many_arguments)]
fn contribute_for(
    env: &mut TestEnv,
    user: &Keypair,
    user_usdc: Pubkey,
    pool: Pubkey,
    tier: Tier,
) -> Result<(), String> {
    contribute_for_with_reserve(env, user, user_usdc, pool, tier)
}

fn contribute_for_with_reserve(
    env: &mut TestEnv,
    user: &Keypair,
    user_usdc: Pubkey,
    pool: Pubkey,
    reserve_tier: Tier,
) -> Result<(), String> {
    let (config_pda, _) = env.protocol_config_pda();
    let (user_rep, _) = env.reputation_pda(&user.pubkey());
    let (participant, _) = env.participant_pda(&pool, &user.pubkey());
    let (pool_usdc_vault, _) = env.pool_usdc_vault_pda(&pool);
    let (collat_vault, _) = env.collateral_vault_pda(&pool);
    let (fee_vault, _) = env.protocol_fee_vault_pda();
    let (reserve_fund, _) = env.reserve_fund_pda(reserve_tier);
    let (reserve_usdc, _) = env.reserve_vault_pda(reserve_tier);
    let (adapter_state, _) = env.vault_adapter_pda(&pool);
    let (adapter_usdc, _) = env.vault_adapter_usdc_pda(&pool);

    let metas = metas_contribute(
        user.pubkey(),
        config_pda,
        user_rep,
        pool,
        participant,
        user_usdc,
        pool_usdc_vault,
        collat_vault,
        fee_vault,
        env.core_invoker,
        reserve_fund,
        reserve_usdc,
        adapter_state,
        adapter_usdc,
    );
    let ix = build_ix(metas, poolver_core::instruction::Contribute {}.data());
    send_ix(&mut env.svm, user, ix)
}

fn advance_month_for(
    env: &mut TestEnv,
    caller: &Keypair,
    pool: Pubkey,
) -> Result<(), String> {
    // LiteSVM dedupes by signature: identical (signer, blockhash,
    // ix-bytes) triplets get rejected as `AlreadyProcessed`. Tests that
    // call advance_month repeatedly with the same payer must bump the
    // blockhash between calls.
    env.svm.expire_blockhash();
    let (config_pda, _) = env.protocol_config_pda();
    let metas = metas_advance_month(caller.pubkey(), config_pda, pool);
    let ix = build_ix(metas, poolver_core::instruction::AdvanceMonth {}.data());
    send_ix(&mut env.svm, caller, ix)
}

// ───── Time helpers ──────────────────────────────────────────────────────

fn set_clock_to(env: &mut TestEnv, ts: i64) {
    let mut clock = env.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = ts;
    env.svm.set_sysvar::<Clock>(&clock);
}

fn current_clock_ts(env: &TestEnv) -> i64 {
    env.svm.get_sysvar::<Clock>().unix_timestamp
}

// ───── Setup builders ────────────────────────────────────────────────────

/// Bootstraps protocol + creates a started pool with 12 participants.
/// Returns (pool_pda, list_of_(user, ata)). All 12 paid month 1 via
/// `join_pool`. Sets the clock so we're inside the month-1 window.
fn pool_with_12_started(
    env: &mut TestEnv,
    pool_id: u64,
    contribution: u64,
    month_duration: Option<i64>,
) -> (Pubkey, Vec<(Keypair, Pubkey)>) {
    let creator = bootstrap_with_creator(env);
    // Set clock to 1_000_000 so we can move forward / backward without
    // hitting the i64::MIN saturation edge.
    set_clock_to(env, 1_000_000);

    let pool = create_pool_for(
        env,
        &creator,
        pool_id,
        Tier::Vault,
        contribution,
        month_duration,
    )
    .expect("create_pool");

    let mut users = Vec::with_capacity(12);
    for _ in 0..12 {
        let (user, ata) = fully_set_up_user(env, 50_000 * ONE_USDC);
        join_pool_for(env, &user, ata, pool, Tier::Vault).expect("join");
        users.push((user, ata));
    }

    let p = env.fetch_pool(&pool);
    assert_eq!(p.current_month, 1, "pool must auto-start");
    (pool, users)
}

// ─────────────────────────────────────────────────────────────────────────
// Helpers for the in-window timestamp
// ─────────────────────────────────────────────────────────────────────────

/// Returns a timestamp guaranteed to be inside the current month window.
fn ts_in_current_window(env: &TestEnv, pool: &Pubkey) -> i64 {
    let p = env.fetch_pool(pool);
    p.current_month_started_at + 1
}

// ─────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn t30_contribute_happy_fee_split() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_12_started(&mut env, 30, contribution, None);

    // Month 1 was paid by `join_pool`. Advance time inside month 1
    // window (clock already there) but to test month-2 we must first
    // tick the clock past the month_duration and call `advance_month`.
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    let dummy_caller = Keypair::new();
    env.svm.airdrop(&dummy_caller.pubkey(), 1 * SOL).unwrap();
    advance_month_for(&mut env, &dummy_caller, pool).expect("advance");

    let p = env.fetch_pool(&pool);
    assert_eq!(p.current_month, 2);
    set_clock_to(&mut env, p.current_month_started_at + 10);

    // Pick user[0]; pre-balance snapshot of all destinations.
    let (user, ata) = (&users[0].0, users[0].1);
    let (fee_vault, _) = env.protocol_fee_vault_pda();
    let (reserve_usdc, _) = env.reserve_vault_pda(Tier::Vault);
    let (adapter_usdc, _) = env.vault_adapter_usdc_pda(&pool);
    let fee_before = env.fetch_token_balance(&fee_vault);
    let reserve_before = env.fetch_token_balance(&reserve_usdc);
    let adapter_before = env.fetch_token_balance(&adapter_usdc);
    let user_before = env.fetch_token_balance(&ata);

    contribute_for(&mut env, user, ata, pool, Tier::Vault).expect("contribute");

    let protocol_fee = contribution * 150 / 10_000;
    let reserve_fee = contribution * 150 / 10_000;
    let net = contribution - protocol_fee - reserve_fee;

    assert_eq!(env.fetch_token_balance(&fee_vault), fee_before + protocol_fee);
    assert_eq!(env.fetch_token_balance(&reserve_usdc), reserve_before + reserve_fee);
    assert_eq!(env.fetch_token_balance(&adapter_usdc), adapter_before + net);
    assert_eq!(env.fetch_token_balance(&ata), user_before - contribution);

    let (part, _) = env.participant_pda(&pool, &user.pubkey());
    let p = env.fetch_participant(&part);
    assert_eq!(p.paid_months, 0b11, "month 1 + month 2 paid");
}

#[test]
fn t31_contribute_rejects_non_participant() {
    let mut env = TestEnv::new();
    let (pool, _) = pool_with_12_started(&mut env, 31, 1_000 * ONE_USDC, None);

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    advance_month_for(&mut env, &caller, pool).unwrap();

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + 10);

    // Outsider with no participant PDA. Anchor seed derivation (the
    // `participant` constraint) must reject — there's nothing
    // initialized at `[participant, pool, outsider]`.
    let (outsider, ata) = fully_set_up_user(&mut env, 50_000 * ONE_USDC);
    let res = contribute_for(&mut env, &outsider, ata, pool, Tier::Vault);
    assert!(res.is_err(), "non-participant must be rejected");
}

#[test]
fn t32_contribute_rejects_already_paid() {
    let mut env = TestEnv::new();
    let (pool, users) = pool_with_12_started(&mut env, 32, 1_000 * ONE_USDC, None);
    let (user, ata) = (&users[0].0, users[0].1);

    let ts = ts_in_current_window(&env, &pool);
    set_clock_to(&mut env, ts);
    // Month 1 already paid by join. Direct contribute attempt for
    // month 1 must fail.
    let res = contribute_for(&mut env, user, ata, pool, Tier::Vault);
    assert!(res.is_err(), "month-1 already paid via join_pool");
}

#[test]
fn t33_contribute_rejects_when_paused() {
    let mut env = TestEnv::new();
    let (pool, users) = pool_with_12_started(&mut env, 33, 1_000 * ONE_USDC, None);

    // Advance to month 2 first.
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    advance_month_for(&mut env, &caller, pool).unwrap();
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + 10);

    // Hack the protocol_config to set paused = true.
    let (config_pda, _) = env.protocol_config_pda();
    let mut acct = env.svm.get_account(&config_pda).unwrap().clone();
    let mut cfg = ProtocolConfig::try_deserialize(&mut acct.data.as_ref()).unwrap();
    cfg.paused = true;
    let mut buf = Vec::new();
    cfg.try_serialize(&mut buf).unwrap();
    acct.data[..buf.len()].copy_from_slice(&buf);
    env.svm.set_account(config_pda, acct).unwrap();

    let (user, ata) = (&users[0].0, users[0].1);
    let res = contribute_for(&mut env, user, ata, pool, Tier::Vault);
    assert!(res.is_err(), "paused protocol must reject");
}

#[test]
fn t34_contribute_rejects_when_pool_complete() {
    let mut env = TestEnv::new();
    let (pool, users) = pool_with_12_started(&mut env, 34, 1_000 * ONE_USDC, None);

    // Force-complete the pool by setting is_complete = true directly.
    let mut acct = env.svm.get_account(&pool).unwrap().clone();
    let mut p = Pool::try_deserialize(&mut acct.data.as_ref()).unwrap();
    p.is_complete = true;
    let mut buf = Vec::new();
    p.try_serialize(&mut buf).unwrap();
    acct.data[..buf.len()].copy_from_slice(&buf);
    env.svm.set_account(pool, acct).unwrap();

    let (user, ata) = (&users[0].0, users[0].1);
    let res = contribute_for(&mut env, user, ata, pool, Tier::Vault);
    assert!(res.is_err(), "complete pool must reject");
}

#[test]
fn t35_contribute_rejects_outside_month_window() {
    let mut env = TestEnv::new();
    let (pool, users) = pool_with_12_started(&mut env, 35, 1_000 * ONE_USDC, None);

    // Advance to month 2.
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    advance_month_for(&mut env, &caller, pool).unwrap();

    // Now jump well past the cure-path liquidation threshold (day 30+
    // after month end). Step 10 SPEC_QUESTION-6 relaxed the strict
    // in-window check to accept contributions during the day 1..=29
    // grace/suspension window, so the test needs to push past the
    // 30-day liquidation cutoff to verify the hard rejection still fires.
    let p = env.fetch_pool(&pool);
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 31 * 86_400,
    );

    let (user, ata) = (&users[0].0, users[0].1);
    let res = contribute_for(&mut env, user, ata, pool, Tier::Vault);
    assert!(res.is_err(), "out-of-window contribute must reject");
}

#[test]
fn t36_contribute_rejects_when_defaulted() {
    let mut env = TestEnv::new();
    let (pool, users) = pool_with_12_started(&mut env, 36, 1_000 * ONE_USDC, None);

    // Advance to month 2.
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    advance_month_for(&mut env, &caller, pool).unwrap();
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + 10);

    // Force user[0]'s participant.is_defaulted = true.
    let (user, ata) = (&users[0].0, users[0].1);
    let (part_pda, _) = env.participant_pda(&pool, &user.pubkey());
    let mut acct = env.svm.get_account(&part_pda).unwrap().clone();
    let mut part = Participant::try_deserialize(&mut acct.data.as_ref()).unwrap();
    part.is_defaulted = true;
    let mut buf = Vec::new();
    part.try_serialize(&mut buf).unwrap();
    acct.data[..buf.len()].copy_from_slice(&buf);
    env.svm.set_account(part_pda, acct).unwrap();

    let res = contribute_for(&mut env, user, ata, pool, Tier::Vault);
    assert!(res.is_err(), "defaulted participant must reject");
}

#[test]
fn t37_contribute_updates_user_reputation() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_12_started(&mut env, 37, contribution, None);

    // Advance to month 2.
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    advance_month_for(&mut env, &caller, pool).unwrap();
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + 10);

    let (user, ata) = (&users[0].0, users[0].1);
    let (rep_pda, _) = env.reputation_pda(&user.pubkey());
    let rep_before = env.fetch_reputation(&rep_pda);

    contribute_for(&mut env, user, ata, pool, Tier::Vault).unwrap();

    let rep_after = env.fetch_reputation(&rep_pda);
    assert_eq!(
        rep_after.total_contributed_lifetime,
        rep_before.total_contributed_lifetime + contribution,
        "reputation lifetime must include this contribution gross"
    );
}

#[test]
fn t38_contribute_releases_collateral_post_win() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_12_started(&mut env, 38, contribution, None);

    // Advance to month 2 (the simulated winner of month 1 now owes
    // months 2..=12 — a 11-month release schedule).
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    advance_month_for(&mut env, &caller, pool).unwrap();
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + 10);

    // Manually mark user[0] as having won month 1, with 11_000 USDC of
    // collateral locked. Step 8 (`claim_winning`) is the real path;
    // here we forge state to test the release branch in isolation.
    let (user, ata) = (&users[0].0, users[0].1);
    let (part_pda, _) = env.participant_pda(&pool, &user.pubkey());
    let mut acct = env.svm.get_account(&part_pda).unwrap().clone();
    let mut part = Participant::try_deserialize(&mut acct.data.as_ref()).unwrap();
    part.has_won = true;
    part.win_month = 1;
    let collateral_initial = 11_000 * ONE_USDC;
    part.collateral_initial = collateral_initial;
    part.collateral_locked = collateral_initial;
    // 11 months_remaining_at_win → 1000 USDC per on-time payment.
    part.collateral_release_per_month = collateral_initial / 11;
    let mut buf = Vec::new();
    part.try_serialize(&mut buf).unwrap();
    acct.data[..buf.len()].copy_from_slice(&buf);
    env.svm.set_account(part_pda, acct).unwrap();

    // Fund the collateral_vault so the release transfer has tokens.
    let (collat_vault, _) = env.collateral_vault_pda(&pool);
    let collat_acct = env.svm.get_account(&collat_vault).unwrap().clone();
    let mut state = spl_token_interface::state::Account::unpack(&collat_acct.data).unwrap();
    state.amount = collateral_initial;
    let mut new_data = vec![0u8; spl_token_interface::state::Account::LEN];
    spl_token_interface::state::Account::pack(state, &mut new_data).unwrap();
    let new_acct = Account {
        lamports: collat_acct.lamports,
        data: new_data,
        owner: collat_acct.owner,
        executable: collat_acct.executable,
        rent_epoch: collat_acct.rent_epoch,
    };
    env.svm.set_account(collat_vault, new_acct).unwrap();

    let user_before = env.fetch_token_balance(&ata);

    contribute_for(&mut env, user, ata, pool, Tier::Vault).expect("contribute");

    let part_after = env.fetch_participant(&part_pda);
    let release_per_month = collateral_initial / 11;
    assert_eq!(
        part_after.collateral_locked,
        collateral_initial - release_per_month,
        "collateral_locked must decrease by release_per_month"
    );

    // User USDC balance: -contribution +release_per_month
    let user_after = env.fetch_token_balance(&ata);
    assert_eq!(user_after, user_before - contribution + release_per_month);
}

#[test]
fn t39_advance_month_happy() {
    let mut env = TestEnv::new();
    let (pool, _) = pool_with_12_started(&mut env, 39, 1_000 * ONE_USDC, None);

    let p_before = env.fetch_pool(&pool);
    set_clock_to(
        &mut env,
        p_before.current_month_started_at + p_before.month_duration_seconds + 1,
    );

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    advance_month_for(&mut env, &caller, pool).expect("advance");

    let p_after = env.fetch_pool(&pool);
    assert_eq!(p_after.current_month, p_before.current_month + 1);
    let now = current_clock_ts(&env);
    assert_eq!(p_after.current_month_started_at, now);
    assert_eq!(p_after.bid_window_ends_at, now + p_after.bid_window_seconds);
    assert!(p_after.reveal_window_ends_at > p_after.bid_window_ends_at);
    assert!(!p_after.is_complete);
}

#[test]
fn t40_advance_month_rejects_too_early() {
    let mut env = TestEnv::new();
    let (pool, _) = pool_with_12_started(&mut env, 40, 1_000 * ONE_USDC, None);

    let p = env.fetch_pool(&pool);
    // Stay strictly inside the month window.
    set_clock_to(&mut env, p.current_month_started_at + 1);

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    let res = advance_month_for(&mut env, &caller, pool);
    assert!(res.is_err(), "advance during the active window must reject");
}

#[test]
fn t41_advance_month_completes_after_month12() {
    let mut env = TestEnv::new();
    // Use a tiny month_duration so we can warp through 12 months
    // quickly. 60 seconds.
    let (pool, _) = pool_with_12_started(&mut env, 41, 1_000 * ONE_USDC, Some(60));

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();

    // Advance from month 1 → 12, then 12 → completed.
    for _ in 0..12 {
        let p = env.fetch_pool(&pool);
        if p.is_complete {
            break;
        }
        set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
        advance_month_for(&mut env, &caller, pool).expect("advance");
    }

    let p = env.fetch_pool(&pool);
    assert!(p.is_complete, "pool must be complete after 12th advance");
    assert!(p.completed_at > 0, "completed_at stamped");
    assert_eq!(p.current_month, 13, "current_month rolls past 12");
}

#[test]
fn t42_advance_month_rejects_when_complete() {
    let mut env = TestEnv::new();
    let (pool, _) = pool_with_12_started(&mut env, 42, 1_000 * ONE_USDC, Some(60));

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();

    for _ in 0..12 {
        let p = env.fetch_pool(&pool);
        if p.is_complete {
            break;
        }
        set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
        advance_month_for(&mut env, &caller, pool).expect("advance");
    }

    // One more advance must error.
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.completed_at + 1_000_000);
    let res = advance_month_for(&mut env, &caller, pool);
    assert!(res.is_err(), "double-complete must reject");
}

#[test]
fn t43_e2e_12_months_solvency() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    // Short month duration for fast test.
    let (pool, users) = pool_with_12_started(&mut env, 43, contribution, Some(60));

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();

    // Months 2..=12: each of 12 participants contributes, then advance.
    for month in 2u8..=12 {
        // Tick into month `month`.
        let p = env.fetch_pool(&pool);
        set_clock_to(
            &mut env,
            p.current_month_started_at + p.month_duration_seconds + 1,
        );
        advance_month_for(&mut env, &caller, pool).expect("advance");
        let p = env.fetch_pool(&pool);
        assert_eq!(p.current_month, month);
        set_clock_to(&mut env, p.current_month_started_at + 10);

        for (user, ata) in &users {
            contribute_for(&mut env, user, *ata, pool, Tier::Vault)
                .unwrap_or_else(|e| panic!("month {} contribute failed: {}", month, e));
        }
    }

    // Confirm every participant fully paid: 12 bits set = 0xFFF.
    for (user, _) in &users {
        let (part, _) = env.participant_pda(&pool, &user.pubkey());
        let p = env.fetch_participant(&part);
        assert_eq!(p.paid_months, 0xFFF, "user paid all 12 months");
    }

    // Now finish month 12 → pool complete.
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    advance_month_for(&mut env, &caller, pool).expect("final advance");
    let p = env.fetch_pool(&pool);
    assert!(p.is_complete);

    // Solvency: 12 contributors × 12 months × contribution =
    //   protocol_fee_vault + reserve + adapter
    let protocol_fee = contribution * 150 / 10_000;
    let reserve_fee = contribution * 150 / 10_000;
    let net = contribution - protocol_fee - reserve_fee;

    let (fee_vault, _) = env.protocol_fee_vault_pda();
    let (reserve_usdc, _) = env.reserve_vault_pda(Tier::Vault);
    let (adapter_usdc, _) = env.vault_adapter_usdc_pda(&pool);

    // 12 months × 12 participants = 144 contributions
    assert_eq!(env.fetch_token_balance(&fee_vault), protocol_fee * 144);
    assert_eq!(env.fetch_token_balance(&reserve_usdc), reserve_fee * 144);
    assert_eq!(env.fetch_token_balance(&adapter_usdc), net * 144);

    let total = env.fetch_token_balance(&fee_vault)
        + env.fetch_token_balance(&reserve_usdc)
        + env.fetch_token_balance(&adapter_usdc);
    assert_eq!(total, 144 * contribution, "INV-1 solvency");
}

#[test]
fn t44_reserve_isolation_wrong_tier() {
    // INV-4 / arch §11. Build a Tier-0 pool, then attempt to contribute
    // while passing the Tier-1 reserve_fund. Anchor seeds derivation in
    // poolver_reserve::deposit must reject before any token movement.
    let mut env = TestEnv::new();
    let (pool, users) = pool_with_12_started(&mut env, 44, 1_000 * ONE_USDC, None);

    // Advance to month 2.
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    advance_month_for(&mut env, &caller, pool).unwrap();
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + 10);

    // Build a contribute ix, but swap reserve_fund + reserve_vault for
    // Tier::DeFi instead of the pool's actual Tier::Vault. The reserve
    // program's tier-encoded seed derivation must reject.
    let (user, ata) = (&users[0].0, users[0].1);
    let res = contribute_for_with_reserve(&mut env, user, ata, pool, Tier::DeFi);
    assert!(
        res.is_err(),
        "wrong-tier reserve must be rejected by seeds derivation"
    );
}
