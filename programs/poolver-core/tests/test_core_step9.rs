//! Step-9 integration tests for `poolver-core`: `distribute_yield`.
//!
//! Coverage map (per task prompt §Tests):
//!   1.  Tier 0 happy path: harvest returns 0, no-op events emitted   → t200
//!   2.  Permissionless: any wallet can call (not just creator)        → t201
//!   3.  Rejected when protocol is paused                              → t202
//!   4.  Rejected when pool is complete                                → t203
//!   5.  Tier 1 → TierNotYetSupported (until step 12)                  → t204
//!   6.  Yield math unit test: 1000 → 700/200/100                      → t205
//!   7.  Yield math unit test: rounding goes INTO participant share    → t206
//!   8.  Yield math unit test: 0 → 0/0/0                               → t207
//!   9.  total_yield_distributed unchanged on Tier 0 no-op             → t208
//!   10. bid_credit_balance unchanged on Tier 0 no-op                  → t209
//!   11. Sequential calls: both succeed as no-ops                      → t210
//!   12. Reserve isolation: pass Tier 1's reserve to Tier 0 pool       → t211
//!   13. Solvency: no token movements on Tier 0 no-op                  → t212
//!
//! Tests #6/7/8 are pure unit tests of the public `compute_yield_splits`
//! helper — they don't need LiteSVM. Tier 0's `harvest()` always returns
//! 0 in V1 (spec §5.3) so the integration tests can only assert the
//! no-op happy path. The split math is exercised via the helper directly,
//! which is the same code that runs on-chain.

#![cfg(feature = "mock-kyc")]

mod common;

use anchor_lang::{AccountSerialize, InstructionData};
use common::*;
use solana_clock::Clock;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

// ───── Account-meta builders (mirror Anchor field order) ────────────────

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
fn metas_distribute_yield(
    caller: Pubkey,
    protocol_config: Pubkey,
    pool: Pubkey,
    pool_usdc_vault: Pubkey,
    protocol_fee_vault: Pubkey,
    core_invoker: Pubkey,
    reserve_fund: Pubkey,
    reserve_usdc_vault: Pubkey,
    adapter_state: Pubkey,
    adapter_usdc_vault: Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new(caller, true),
        AccountMeta::new_readonly(protocol_config, false),
        AccountMeta::new(pool, false),
        AccountMeta::new(pool_usdc_vault, false),
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

fn build_ix(metas: Vec<AccountMeta>, data: Vec<u8>) -> Instruction {
    Instruction {
        program_id: poolver_core::ID,
        accounts: metas,
        data,
    }
}

// ───── High-level helpers ───────────────────────────────────────────────

fn init_protocol(env: &mut TestEnv) {
    let (config_pda, _) = env.protocol_config_pda();
    let (fee_vault_pda, _) = env.protocol_fee_vault_pda();
    let metas = metas_initialize_protocol(
        env.admin.pubkey(),
        config_pda,
        env.usdc_mint,
        fee_vault_pda,
    );
    let ix = build_ix(
        metas,
        poolver_core::instruction::InitializeProtocol {}.data(),
    );
    let admin = env.admin.insecure_clone();
    send_ix(&mut env.svm, &admin, ix).expect("init protocol");
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
            month_duration_seconds: None,
        }
        .data(),
    );
    send_ix(&mut env.svm, creator, ix).map(|_| pool_pda)
}

fn distribute_yield_for(
    env: &mut TestEnv,
    caller: &Keypair,
    pool: Pubkey,
    tier: Tier,
) -> Result<(), String> {
    env.svm.expire_blockhash();
    let (config_pda, _) = env.protocol_config_pda();
    let (pool_usdc_vault, _) = env.pool_usdc_vault_pda(&pool);
    let (fee_vault, _) = env.protocol_fee_vault_pda();
    let (reserve_fund, _) = env.reserve_fund_pda(tier);
    let (reserve_usdc, _) = env.reserve_vault_pda(tier);
    let (adapter_state, _) = env.vault_adapter_pda(&pool);
    let (adapter_usdc, _) = env.vault_adapter_usdc_pda(&pool);

    let metas = metas_distribute_yield(
        caller.pubkey(),
        config_pda,
        pool,
        pool_usdc_vault,
        fee_vault,
        env.core_invoker,
        reserve_fund,
        reserve_usdc,
        adapter_state,
        adapter_usdc,
    );
    let ix = build_ix(
        metas,
        poolver_core::instruction::DistributeYield {}.data(),
    );
    send_ix(&mut env.svm, caller, ix)
}

fn set_clock_to(env: &mut TestEnv, ts: i64) {
    let mut clock = env.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = ts;
    env.svm.set_sysvar::<Clock>(&clock);
}

// ─────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn t200_distribute_yield_tier0_happy_path_noop() {
    // Tier 0 harvest returns 0 → both events emitted with zeros, no
    // token movements, all state unchanged.
    let mut env = TestEnv::new();
    let creator = bootstrap_with_creator(&mut env);
    set_clock_to(&mut env, 1_000_000);
    let pool = create_pool_for(&mut env, &creator, 200, Tier::Vault, 1_000 * ONE_USDC)
        .expect("create_pool");

    // Snapshot ALL token balances + pool state BEFORE distribute_yield.
    let (fee_vault, _) = env.protocol_fee_vault_pda();
    let (reserve_usdc, _) = env.reserve_vault_pda(Tier::Vault);
    let (pool_vault, _) = env.pool_usdc_vault_pda(&pool);
    let (adapter_usdc, _) = env.vault_adapter_usdc_pda(&pool);

    let fee_before = env.fetch_token_balance(&fee_vault);
    let reserve_before = env.fetch_token_balance(&reserve_usdc);
    let pool_before = env.fetch_token_balance(&pool_vault);
    let adapter_before = env.fetch_token_balance(&adapter_usdc);
    let p_before = env.fetch_pool(&pool);

    distribute_yield_for(&mut env, &creator, pool, Tier::Vault)
        .expect("distribute_yield Tier 0 happy path");

    // No token movements.
    assert_eq!(env.fetch_token_balance(&fee_vault), fee_before);
    assert_eq!(env.fetch_token_balance(&reserve_usdc), reserve_before);
    assert_eq!(env.fetch_token_balance(&pool_vault), pool_before);
    assert_eq!(env.fetch_token_balance(&adapter_usdc), adapter_before);

    // Pool state unchanged.
    let p_after = env.fetch_pool(&pool);
    assert_eq!(p_after.total_yield_distributed, 0);
    assert_eq!(p_after.total_yield_distributed, p_before.total_yield_distributed);
    assert_eq!(p_after.bid_credit_balance, p_before.bid_credit_balance);
}

#[test]
fn t201_distribute_yield_permissionless() {
    // Any wallet can call distribute_yield — no signer authorization
    // beyond paying tx fees. This is critical for keeper-bot operation.
    let mut env = TestEnv::new();
    let creator = bootstrap_with_creator(&mut env);
    set_clock_to(&mut env, 1_000_000);
    let pool = create_pool_for(&mut env, &creator, 201, Tier::Vault, 1_000 * ONE_USDC)
        .expect("create_pool");

    // A random wallet (NOT the creator, NOT the admin) drives the call.
    let random_caller = Keypair::new();
    env.svm.airdrop(&random_caller.pubkey(), 10 * SOL).unwrap();

    distribute_yield_for(&mut env, &random_caller, pool, Tier::Vault)
        .expect("permissionless: random caller can distribute_yield");
}

#[test]
fn t202_distribute_yield_rejected_when_paused() {
    // Pausing the protocol disables distribute_yield (defence-in-depth).
    let mut env = TestEnv::new();
    let creator = bootstrap_with_creator(&mut env);
    set_clock_to(&mut env, 1_000_000);
    let pool = create_pool_for(&mut env, &creator, 202, Tier::Vault, 1_000 * ONE_USDC)
        .expect("create_pool");

    // Force-pause the protocol_config in-place. There is no
    // emergency_pause ix in V1 (step 11+), so we patch the account
    // directly — same trick as step 8's KYC-expired test (t105).
    use anchor_lang::AccountDeserialize;
    let (config_pda, _) = env.protocol_config_pda();
    let mut acct = env.svm.get_account(&config_pda).unwrap().clone();
    let mut cfg = ProtocolConfig::try_deserialize(&mut acct.data.as_ref()).unwrap();
    cfg.paused = true;
    let mut buf = Vec::new();
    cfg.try_serialize(&mut buf).unwrap();
    acct.data[..buf.len()].copy_from_slice(&buf);
    env.svm.set_account(config_pda, acct).unwrap();

    let res = distribute_yield_for(&mut env, &creator, pool, Tier::Vault);
    assert!(res.is_err(), "distribute_yield must reject when paused");
}

#[test]
fn t203_distribute_yield_rejected_when_pool_complete() {
    // After advance_month finalizes the pool (current_month > 12), the
    // pool is_complete=true and distribute_yield must reject.
    let mut env = TestEnv::new();
    let creator = bootstrap_with_creator(&mut env);
    set_clock_to(&mut env, 1_000_000);
    let pool = create_pool_for(&mut env, &creator, 203, Tier::Vault, 1_000 * ONE_USDC)
        .expect("create_pool");

    // Force-mark the pool complete in-place. Simpler than driving 12
    // full months; same field semantics either way.
    use anchor_lang::AccountDeserialize;
    let mut acct = env.svm.get_account(&pool).unwrap().clone();
    let mut p = Pool::try_deserialize(&mut acct.data.as_ref()).unwrap();
    p.is_complete = true;
    p.completed_at = 2_000_000;
    let mut buf = Vec::new();
    p.try_serialize(&mut buf).unwrap();
    acct.data[..buf.len()].copy_from_slice(&buf);
    env.svm.set_account(pool, acct).unwrap();

    let res = distribute_yield_for(&mut env, &creator, pool, Tier::Vault);
    assert!(res.is_err(), "distribute_yield must reject completed pool");
}

#[test]
fn t204_distribute_yield_tier1_rejected() {
    // V1 only Tier 0 is enabled. A pool with tier == DeFi should reject
    // with TierNotYetSupported. Since create_pool already rejects Tier 1
    // creation, we simulate by patching pool.tier in-place after create.
    let mut env = TestEnv::new();
    let creator = bootstrap_with_creator(&mut env);
    set_clock_to(&mut env, 1_000_000);
    let pool = create_pool_for(&mut env, &creator, 204, Tier::Vault, 1_000 * ONE_USDC)
        .expect("create_pool");

    // Patch pool.tier to DeFi (1).
    use anchor_lang::AccountDeserialize;
    let mut acct = env.svm.get_account(&pool).unwrap().clone();
    let mut p = Pool::try_deserialize(&mut acct.data.as_ref()).unwrap();
    p.tier = Tier::DeFi;
    let mut buf = Vec::new();
    p.try_serialize(&mut buf).unwrap();
    acct.data[..buf.len()].copy_from_slice(&buf);
    env.svm.set_account(pool, acct).unwrap();

    // Driver still passes Tier::DeFi reserve (so the reserve isolation
    // check itself doesn't fire first).
    let res = distribute_yield_for(&mut env, &creator, pool, Tier::DeFi);
    assert!(
        res.is_err(),
        "distribute_yield must reject Tier 1 pool until step 12"
    );
}

#[test]
fn t205_yield_splits_1000_usdc() {
    // Pure unit test of the public split helper. 1000 USDC → 700/200/100.
    use poolver_core::instructions::distribute_yield::compute_yield_splits;
    let yield_amount = 1_000 * ONE_USDC;
    let (participant, reserve, protocol) = compute_yield_splits(yield_amount).unwrap();
    assert_eq!(participant, 700 * ONE_USDC);
    assert_eq!(reserve, 200 * ONE_USDC);
    assert_eq!(protocol, 100 * ONE_USDC);
    assert_eq!(participant + reserve + protocol, yield_amount);
}

#[test]
fn t206_yield_splits_rounding_into_participant() {
    // Spec §4 + §13 style: any BPS rounding error must stay INSIDE the
    // participant share — never inflate the protocol or reserve.
    //
    // 12_345 lamports yield: 1000 bps → 1234.5 (truncated to 1234),
    //                        2000 bps → 2469.0 (exactly 2469).
    // Subtraction-based participant share absorbs the truncation:
    //   participant = 12_345 - 1234 - 2469 = 8_642
    use poolver_core::instructions::distribute_yield::compute_yield_splits;
    let yield_amount = 12_345u64;
    let (participant, reserve, protocol) = compute_yield_splits(yield_amount).unwrap();
    assert_eq!(protocol, 1234, "protocol = floor(12_345 * 1000 / 10000)");
    assert_eq!(reserve, 2469, "reserve = floor(12_345 * 2000 / 10000)");
    assert_eq!(participant, 8642, "participant absorbs rounding");
    assert_eq!(participant + reserve + protocol, yield_amount);
}

#[test]
fn t207_yield_splits_zero() {
    // 0 → 0/0/0 — the V1 Tier 0 happy path.
    use poolver_core::instructions::distribute_yield::compute_yield_splits;
    let (participant, reserve, protocol) = compute_yield_splits(0).unwrap();
    assert_eq!(participant, 0);
    assert_eq!(reserve, 0);
    assert_eq!(protocol, 0);
}

#[test]
fn t208_total_yield_distributed_unchanged_on_noop() {
    // distribute_yield called repeatedly on Tier 0 — total_yield_distributed
    // stays at 0 forever (Tier 0 generates no yield by definition).
    let mut env = TestEnv::new();
    let creator = bootstrap_with_creator(&mut env);
    set_clock_to(&mut env, 1_000_000);
    let pool = create_pool_for(&mut env, &creator, 208, Tier::Vault, 1_000 * ONE_USDC)
        .expect("create_pool");

    let p_before = env.fetch_pool(&pool);
    assert_eq!(p_before.total_yield_distributed, 0, "starts at 0");

    for _ in 0..3 {
        distribute_yield_for(&mut env, &creator, pool, Tier::Vault).expect("distribute_yield");
    }

    let p_after = env.fetch_pool(&pool);
    assert_eq!(
        p_after.total_yield_distributed, 0,
        "Tier 0 total_yield_distributed stays at 0 across multiple harvests"
    );
}

#[test]
fn t209_bid_credit_balance_unchanged_on_noop() {
    // Same as t208 but for bid_credit_balance — the participant share is
    // 0 on a no-op so the credit ledger doesn't move.
    let mut env = TestEnv::new();
    let creator = bootstrap_with_creator(&mut env);
    set_clock_to(&mut env, 1_000_000);
    let pool = create_pool_for(&mut env, &creator, 209, Tier::Vault, 1_000 * ONE_USDC)
        .expect("create_pool");

    let p_before = env.fetch_pool(&pool);
    distribute_yield_for(&mut env, &creator, pool, Tier::Vault).expect("distribute_yield");
    let p_after = env.fetch_pool(&pool);

    assert_eq!(
        p_after.bid_credit_balance, p_before.bid_credit_balance,
        "bid_credit_balance unchanged on no-op"
    );
}

#[test]
fn t210_sequential_calls_both_succeed() {
    // distribute_yield is idempotent for Tier 0: every call is a no-op
    // that succeeds and emits zeroes. (Tier 1 step 12 will make the
    // second call return 0 because last_recorded_balance is now equal
    // to the vault balance.)
    let mut env = TestEnv::new();
    let creator = bootstrap_with_creator(&mut env);
    set_clock_to(&mut env, 1_000_000);
    let pool = create_pool_for(&mut env, &creator, 210, Tier::Vault, 1_000 * ONE_USDC)
        .expect("create_pool");

    distribute_yield_for(&mut env, &creator, pool, Tier::Vault).expect("first call");
    distribute_yield_for(&mut env, &creator, pool, Tier::Vault).expect("second call");
    distribute_yield_for(&mut env, &creator, pool, Tier::Vault).expect("third call");
}

#[test]
fn t211_reserve_isolation_wrong_tier_rejected() {
    // INV-4 (tier isolation): a Tier 0 pool MUST NOT distribute yield
    // into the Tier 1 reserve. The handler re-derives the expected
    // reserve_fund PDA against `pool.tier` and rejects mismatches with
    // CoreError::Unauthorized BEFORE the reserve CPI fires.
    let mut env = TestEnv::new();
    let creator = bootstrap_with_creator(&mut env);
    set_clock_to(&mut env, 1_000_000);
    let pool = create_pool_for(&mut env, &creator, 211, Tier::Vault, 1_000 * ONE_USDC)
        .expect("create_pool");

    // Pool is Tier 0 (Vault); driver passes Tier::DeFi reserve accounts.
    let res = distribute_yield_for(&mut env, &creator, pool, Tier::DeFi);
    assert!(
        res.is_err(),
        "Tier 0 pool with Tier 1 reserve must be rejected"
    );
}

#[test]
fn t212_solvency_no_token_movements_on_noop() {
    // INV-1 solvency: zero-yield distribute_yield doesn't move any USDC.
    // Sum of all custody endpoints is invariant.
    let mut env = TestEnv::new();
    let creator = bootstrap_with_creator(&mut env);
    set_clock_to(&mut env, 1_000_000);
    let pool = create_pool_for(&mut env, &creator, 212, Tier::Vault, 1_000 * ONE_USDC)
        .expect("create_pool");

    let (fee_vault, _) = env.protocol_fee_vault_pda();
    let (reserve_usdc, _) = env.reserve_vault_pda(Tier::Vault);
    let (collat_vault, _) = env.collateral_vault_pda(&pool);
    let (pool_vault, _) = env.pool_usdc_vault_pda(&pool);
    let (adapter_usdc, _) = env.vault_adapter_usdc_pda(&pool);

    let total_before = env.fetch_token_balance(&fee_vault)
        + env.fetch_token_balance(&reserve_usdc)
        + env.fetch_token_balance(&collat_vault)
        + env.fetch_token_balance(&pool_vault)
        + env.fetch_token_balance(&adapter_usdc);

    distribute_yield_for(&mut env, &creator, pool, Tier::Vault).expect("distribute_yield");

    let total_after = env.fetch_token_balance(&fee_vault)
        + env.fetch_token_balance(&reserve_usdc)
        + env.fetch_token_balance(&collat_vault)
        + env.fetch_token_balance(&pool_vault)
        + env.fetch_token_balance(&adapter_usdc);

    assert_eq!(
        total_before, total_after,
        "INV-1 solvency: zero-yield distribute_yield is balance-preserving"
    );
}
