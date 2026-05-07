//! Unit / integration tests for `poolver-core` (step 4 surface).
//!
//! The whole file is gated under `#[cfg(feature = "mock-kyc")]` because
//! every test path needs the mock KYC issuer to seed attestations.
//! Production builds (`--no-default-features`) drop the instruction
//! AND every test that relies on it; this is the production cutover
//! gate (INV-26 + arch §10).
//!
//! Coverage map (per task prompt §F):
//!   1.  initialize_protocol happy path                              → t01
//!   2.  initialize_protocol rejects on second call                  → t02
//!   3.  mock_issue_kyc happy path (admin issues to user)            → t03
//!   4.  mock_issue_kyc rejected when caller != admin                → t04
//!   5.  initialize_user_reputation happy path                       → t05
//!   6.  initialize_user_reputation rejects on second call           → t06
//!   7.  create_pool happy path Tier 0                               → t07
//!   8.  create_pool rejected on Tier 1 (TierNotYetSupported)        → t08
//!   9.  create_pool rejected on contribution too low                → t09
//!   10. create_pool rejected on contribution too high               → t10
//!   11. create_pool rejected when creator has no Light KYC          → t11
//!   12. join_pool happy path: fee split + reserve + yield-vault     → t12
//!   13. join_pool rejected when user has no KYC                     → t13
//!   14. join_pool rejected when user has expired KYC                → t14
//!   15. join_pool rejected when user has sanctions hit              → t15
//!   16. join_pool rejected when user already a participant          → t16
//!   17. join_pool rejected when pool already at 12                  → (covered by t18 boundary)
//!   18. Pool auto-starts on 12th join (PoolStarted, current_month=1)→ t18
//!   19. End-to-end fee accounting (INV solvency check)              → t19

#![cfg(feature = "mock-kyc")]

mod common;

use anchor_lang::{InstructionData, AccountDeserialize};
use common::*;
use solana_instruction::AccountMeta;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

// ───── Account-meta builders (Anchor-order-sensitive) ────────────────────

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

fn metas_initialize_user_reputation(
    user: Pubkey,
    reputation: Pubkey,
) -> Vec<AccountMeta> {
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
        AccountMeta::new_readonly(collateral_vault, false),
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

fn build_ix(metas: Vec<AccountMeta>, data: Vec<u8>) -> solana_instruction::Instruction {
    solana_instruction::Instruction {
        program_id: poolver_core::ID,
        accounts: metas,
        data,
    }
}

// ───── Reserve init helper (reserve must be live for join_pool CPIs) ────

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
    let ix = solana_instruction::Instruction {
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

// ───── High-level test helpers ──────────────────────────────────────────

fn init_protocol(env: &mut TestEnv) -> (Pubkey, Pubkey) {
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

/// Set up: protocol, both reserve tiers, KYC + reputation for a creator.
/// Returns the creator keypair.
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

fn fully_set_up_user(env: &mut TestEnv, balance: u64) -> (Keypair, Pubkey) {
    let user = Keypair::new();
    env.svm.airdrop(&user.pubkey(), 100 * SOL).unwrap();
    issue_mock_kyc(env, &user.pubkey(), KycLevel::Light);
    init_reputation(env, &user);
    let ata = env.fund_token_account(&user.pubkey(), balance);
    (user, ata)
}

#[allow(clippy::too_many_arguments)]
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

// ─────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn t01_initialize_protocol_happy() {
    let mut env = TestEnv::new();
    let (config_pda, fee_vault_pda) = init_protocol(&mut env);
    let cfg = env.fetch_protocol_config();
    assert_eq!(cfg.admin, env.admin.pubkey());
    assert_eq!(cfg.kyc_oracle, env.admin.pubkey()); // MOCK_KYC: V1 = admin
    assert_eq!(cfg.usdc_mint, env.usdc_mint);
    assert_eq!(cfg.protocol_fee_vault, fee_vault_pda);
    assert_eq!(cfg.protocol_fee_bps, 150);
    assert_eq!(cfg.vault_reserve_fee_bps, 150);
    assert_eq!(cfg.defi_reserve_fee_bps, 250);
    assert!(!cfg.paused);
    let _ = config_pda;
}

#[test]
fn t02_initialize_protocol_rejects_double_init() {
    let mut env = TestEnv::new();
    init_protocol(&mut env);
    // Second call must fail (Anchor `init` rejects re-init).
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
    let res = send_ix(&mut env.svm, &admin, ix);
    assert!(res.is_err(), "second initialize_protocol should fail");
}

#[test]
fn t03_mock_issue_kyc_happy() {
    let mut env = TestEnv::new();
    init_protocol(&mut env);
    let user = Pubkey::new_unique();
    issue_mock_kyc(&mut env, &user, KycLevel::Light);

    let (att, _) = env.kyc_pda(&user);
    let attestation = env.fetch_kyc(&att);
    assert_eq!(attestation.user, user);
    assert_eq!(attestation.level, KycLevel::Light.as_u8());
    assert_eq!(attestation.issued_by, env.admin.pubkey());
    assert!(attestation.sanctions_clean);
    assert_eq!(attestation.cpf_hash, [0u8; 32]); // MOCK_KYC: zeroed
}

#[test]
fn t04_mock_issue_kyc_rejects_non_admin() {
    let mut env = TestEnv::new();
    init_protocol(&mut env);

    let imposter = Keypair::new();
    env.svm.airdrop(&imposter.pubkey(), 10 * SOL).unwrap();

    let target_user = Pubkey::new_unique();
    let (config_pda, _) = env.protocol_config_pda();
    let (att_pda, _) = env.kyc_pda(&target_user);

    let metas =
        metas_mock_issue_kyc(imposter.pubkey(), config_pda, target_user, att_pda);
    let ix = build_ix(
        metas,
        poolver_core::instruction::MockIssueKyc {
            user: target_user,
            level: KycLevel::Light,
        }
        .data(),
    );
    let res = send_ix(&mut env.svm, &imposter, ix);
    assert!(res.is_err(), "non-admin must not be able to issue KYC");
}

#[test]
fn t05_initialize_user_reputation_happy() {
    let mut env = TestEnv::new();
    init_protocol(&mut env);
    let user = Keypair::new();
    env.svm.airdrop(&user.pubkey(), 10 * SOL).unwrap();
    init_reputation(&mut env, &user);

    let (rep_pda, _) = env.reputation_pda(&user.pubkey());
    let rep = env.fetch_reputation(&rep_pda);
    assert_eq!(rep.user, user.pubkey());
    assert_eq!(rep.pools_joined, 0);
    assert_eq!(rep.pools_completed, 0);
    assert_eq!(rep.kyc_status, KycLevel::None.as_u8());
}

#[test]
fn t06_initialize_user_reputation_rejects_double_init() {
    let mut env = TestEnv::new();
    init_protocol(&mut env);
    let user = Keypair::new();
    env.svm.airdrop(&user.pubkey(), 10 * SOL).unwrap();
    init_reputation(&mut env, &user);

    // Second call must fail.
    let (rep_pda, _) = env.reputation_pda(&user.pubkey());
    let metas = metas_initialize_user_reputation(user.pubkey(), rep_pda);
    let ix = build_ix(
        metas,
        poolver_core::instruction::InitializeUserReputation {}.data(),
    );
    let res = send_ix(&mut env.svm, &user, ix);
    assert!(res.is_err(), "double init reputation should fail");
}

#[test]
fn t07_create_pool_happy_tier0() {
    let mut env = TestEnv::new();
    let creator = bootstrap_with_creator(&mut env);
    let contribution = 1_000 * ONE_USDC;
    let pool = create_pool_for(&mut env, &creator, 1, Tier::Vault, contribution)
        .expect("create_pool");

    let p = env.fetch_pool(&pool);
    assert_eq!(p.creator, creator.pubkey());
    assert_eq!(p.pool_id, 1);
    assert_eq!(p.tier, Tier::Vault);
    assert_eq!(p.contribution_amount, contribution);
    assert_eq!(p.current_month, 0);
    assert!(!p.is_complete);
    assert_eq!(p.participant_count, 12);
    assert_eq!(p.total_months, 12);

    // Adapter state initialized via CPI.
    let (adapter_state, _) = env.vault_adapter_pda(&pool);
    assert!(env.account_exists(&adapter_state), "adapter state must exist");
}

#[test]
fn t08_create_pool_rejects_tier1() {
    let mut env = TestEnv::new();
    let creator = bootstrap_with_creator(&mut env);
    let res = create_pool_for(&mut env, &creator, 2, Tier::DeFi, 1_000 * ONE_USDC);
    assert!(res.is_err(), "Tier 1 must be TierNotYetSupported in V1");
}

#[test]
fn t09_create_pool_rejects_contribution_too_low() {
    let mut env = TestEnv::new();
    let creator = bootstrap_with_creator(&mut env);
    // 99 USDC < 100 USDC minimum.
    let res = create_pool_for(&mut env, &creator, 3, Tier::Vault, 99 * ONE_USDC);
    assert!(res.is_err(), "below-min contribution must be rejected");
}

#[test]
fn t10_create_pool_rejects_contribution_too_high() {
    let mut env = TestEnv::new();
    let creator = bootstrap_with_creator(&mut env);
    // 10_001 USDC > 10_000 USDC max.
    let res = create_pool_for(&mut env, &creator, 4, Tier::Vault, 10_001 * ONE_USDC);
    assert!(res.is_err(), "above-max contribution must be rejected");
}

#[test]
fn t11_create_pool_rejects_no_kyc() {
    let mut env = TestEnv::new();
    init_protocol(&mut env);
    init_reserve_for(&mut env, Tier::Vault);

    let creator = Keypair::new();
    env.svm.airdrop(&creator.pubkey(), 100 * SOL).unwrap();
    init_reputation(&mut env, &creator);
    // No KYC issued.
    let res = create_pool_for(&mut env, &creator, 5, Tier::Vault, 1_000 * ONE_USDC);
    assert!(res.is_err(), "must reject creator with no KYC");
}

#[test]
fn t12_join_pool_happy_fee_split() {
    let mut env = TestEnv::new();
    let creator = bootstrap_with_creator(&mut env);
    let contribution = 1_000 * ONE_USDC;
    let pool = create_pool_for(&mut env, &creator, 7, Tier::Vault, contribution).unwrap();

    let (user, ata) = fully_set_up_user(&mut env, 5_000 * ONE_USDC);
    join_pool_for(&mut env, &user, ata, pool, Tier::Vault).expect("join_pool");

    // Fee math (Vault tier: 150 bps protocol + 150 bps reserve).
    let protocol_fee = contribution * 150 / 10_000;
    let reserve_fee = contribution * 150 / 10_000;
    let net_to_pool = contribution - protocol_fee - reserve_fee;

    // Where each piece landed:
    let (fee_vault, _) = env.protocol_fee_vault_pda();
    let (reserve_usdc, _) = env.reserve_vault_pda(Tier::Vault);
    let (adapter_usdc, _) = env.vault_adapter_usdc_pda(&pool);
    assert_eq!(env.fetch_token_balance(&fee_vault), protocol_fee);
    assert_eq!(env.fetch_token_balance(&reserve_usdc), reserve_fee);
    assert_eq!(env.fetch_token_balance(&adapter_usdc), net_to_pool);
    // pool_usdc_vault should now be drained (used as transit).
    let (pool_vault, _) = env.pool_usdc_vault_pda(&pool);
    assert_eq!(env.fetch_token_balance(&pool_vault), 0);

    // Participant record + reputation snapshot.
    let (part_pda, _) = env.participant_pda(&pool, &user.pubkey());
    let p = env.fetch_participant(&part_pda);
    assert_eq!(p.pool, pool);
    assert_eq!(p.user, user.pubkey());
    assert_eq!(p.paid_months, 0b1);
    assert_eq!(p.completed_cycles_at_join, 0); // fresh user
    assert!(!p.has_won);

    // Pool state: 1 participant, slot 0 set.
    let pool_acc = env.fetch_pool(&pool);
    assert_eq!(pool_acc.participants[0], Some(user.pubkey()));
    assert!(pool_acc.participants[1..].iter().all(|s| s.is_none()));
    assert_eq!(pool_acc.current_month, 0); // not yet started
    assert_eq!(pool_acc.total_contributed, net_to_pool);

    // Reputation incremented.
    let (rep_pda, _) = env.reputation_pda(&user.pubkey());
    let rep = env.fetch_reputation(&rep_pda);
    assert_eq!(rep.pools_joined, 1);
    assert_eq!(rep.kyc_status, KycLevel::Light.as_u8());
}

#[test]
fn t13_join_pool_rejects_no_kyc() {
    let mut env = TestEnv::new();
    let creator = bootstrap_with_creator(&mut env);
    let pool = create_pool_for(&mut env, &creator, 9, Tier::Vault, 1_000 * ONE_USDC).unwrap();

    // Set up a user with NO KYC.
    let user = Keypair::new();
    env.svm.airdrop(&user.pubkey(), 100 * SOL).unwrap();
    init_reputation(&mut env, &user);
    let ata = env.fund_token_account(&user.pubkey(), 5_000 * ONE_USDC);

    let res = join_pool_for(&mut env, &user, ata, pool, Tier::Vault);
    assert!(res.is_err(), "user without KYC must be rejected");
}

#[test]
fn t14_join_pool_rejects_expired_kyc() {
    let mut env = TestEnv::new();
    let creator = bootstrap_with_creator(&mut env);
    let pool = create_pool_for(&mut env, &creator, 10, Tier::Vault, 1_000 * ONE_USDC).unwrap();

    let (user, ata) = fully_set_up_user(&mut env, 5_000 * ONE_USDC);

    // Force KYC expiry by hacking the on-chain account.
    let (att_pda, _) = env.kyc_pda(&user.pubkey());
    let mut att_acc = env.svm.get_account(&att_pda).unwrap().clone();
    let mut att = KycAttestation::try_deserialize(&mut att_acc.data.as_ref()).unwrap();
    att.expires_at = 0; // already expired
    let mut buf: Vec<u8> = Vec::new();
    use anchor_lang::AccountSerialize;
    att.try_serialize(&mut buf).unwrap();
    att_acc.data[..buf.len()].copy_from_slice(&buf);
    env.svm.set_account(att_pda, att_acc).unwrap();

    let res = join_pool_for(&mut env, &user, ata, pool, Tier::Vault);
    assert!(res.is_err(), "expired KYC must be rejected");
}

#[test]
fn t15_join_pool_rejects_sanctions_hit() {
    let mut env = TestEnv::new();
    let creator = bootstrap_with_creator(&mut env);
    let pool = create_pool_for(&mut env, &creator, 11, Tier::Vault, 1_000 * ONE_USDC).unwrap();

    let (user, ata) = fully_set_up_user(&mut env, 5_000 * ONE_USDC);

    // Flip sanctions_clean = false on-chain.
    let (att_pda, _) = env.kyc_pda(&user.pubkey());
    let mut att_acc = env.svm.get_account(&att_pda).unwrap().clone();
    let mut att = KycAttestation::try_deserialize(&mut att_acc.data.as_ref()).unwrap();
    att.sanctions_clean = false;
    let mut buf: Vec<u8> = Vec::new();
    use anchor_lang::AccountSerialize;
    att.try_serialize(&mut buf).unwrap();
    att_acc.data[..buf.len()].copy_from_slice(&buf);
    env.svm.set_account(att_pda, att_acc).unwrap();

    let res = join_pool_for(&mut env, &user, ata, pool, Tier::Vault);
    assert!(res.is_err(), "sanctions hit must be rejected");
}

#[test]
fn t16_join_pool_rejects_double_join() {
    let mut env = TestEnv::new();
    let creator = bootstrap_with_creator(&mut env);
    let pool = create_pool_for(&mut env, &creator, 12, Tier::Vault, 1_000 * ONE_USDC).unwrap();

    let (user, ata) = fully_set_up_user(&mut env, 5_000 * ONE_USDC);
    join_pool_for(&mut env, &user, ata, pool, Tier::Vault).expect("first join");

    let res = join_pool_for(&mut env, &user, ata, pool, Tier::Vault);
    assert!(res.is_err(), "second join by same user must be rejected");
}

#[test]
fn t18_pool_auto_starts_on_12th_join() {
    let mut env = TestEnv::new();
    let creator = bootstrap_with_creator(&mut env);
    let contribution = 1_000 * ONE_USDC;
    let pool = create_pool_for(&mut env, &creator, 14, Tier::Vault, contribution).unwrap();

    // 12 fresh users join.
    for _ in 0..12 {
        let (user, ata) = fully_set_up_user(&mut env, 5_000 * ONE_USDC);
        join_pool_for(&mut env, &user, ata, pool, Tier::Vault).expect("join");
    }

    let p = env.fetch_pool(&pool);
    assert_eq!(p.current_month, 1, "pool must auto-start on 12th join");
    // start_timestamp == current_month_started_at, both set from
    // Clock::get(). LiteSVM may return 0; just assert they match.
    assert_eq!(p.start_timestamp, p.current_month_started_at);
    assert!(p.participants.iter().all(|s| s.is_some()), "pool full");

    // 13th join is rejected (PoolFull / PoolAlreadyStarted).
    let (user13, ata13) = fully_set_up_user(&mut env, 5_000 * ONE_USDC);
    let res = join_pool_for(&mut env, &user13, ata13, pool, Tier::Vault);
    assert!(res.is_err(), "13th join must be rejected");
}

#[test]
fn t19_e2e_fee_accounting_solvency() {
    // Solvency check: after 12 joins, the sum of token movements equals
    // the fee split times 12. INV-1 / INV-3 / INV-21 family.
    let mut env = TestEnv::new();
    let creator = bootstrap_with_creator(&mut env);
    let contribution = 1_000 * ONE_USDC;
    let pool = create_pool_for(&mut env, &creator, 15, Tier::Vault, contribution).unwrap();

    for _ in 0..12 {
        let (user, ata) = fully_set_up_user(&mut env, 5_000 * ONE_USDC);
        join_pool_for(&mut env, &user, ata, pool, Tier::Vault).expect("join");
    }

    let protocol_fee = contribution * 150 / 10_000;
    let reserve_fee = contribution * 150 / 10_000;
    let net = contribution - protocol_fee - reserve_fee;

    let (fee_vault, _) = env.protocol_fee_vault_pda();
    let (reserve_usdc, _) = env.reserve_vault_pda(Tier::Vault);
    let (adapter_usdc, _) = env.vault_adapter_usdc_pda(&pool);

    assert_eq!(env.fetch_token_balance(&fee_vault), protocol_fee * 12);
    assert_eq!(env.fetch_token_balance(&reserve_usdc), reserve_fee * 12);
    assert_eq!(env.fetch_token_balance(&adapter_usdc), net * 12);

    // Solvency identity: fee_vault + reserve + adapter == 12 * contribution.
    let total = env.fetch_token_balance(&fee_vault)
        + env.fetch_token_balance(&reserve_usdc)
        + env.fetch_token_balance(&adapter_usdc);
    assert_eq!(total, 12 * contribution, "solvency check failed");
}

// ─────────────────────────────────────────────────────────────────────────
// SPEC_QUESTION-26 — admin_close_protocol regression suite
// ─────────────────────────────────────────────────────────────────────────
//
// These tests cover the post-deploy USDC-mint rotation flow: the admin
// closes both `ProtocolConfig` and `protocol_fee_vault`, then re-runs
// `initialize_protocol` against a *different* USDC mint and the protocol
// comes back online with the fresh binding. Negative tests assert the
// `has_one = admin` constraint rejects rogue signers.

fn metas_admin_close_protocol(
    admin: Pubkey,
    protocol_config: Pubkey,
    protocol_fee_vault: Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new(admin, true),
        AccountMeta::new(protocol_config, false),
        AccountMeta::new(protocol_fee_vault, false),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
    ]
}

fn admin_close_protocol(env: &mut TestEnv, signer: &Keypair) -> Result<(), String> {
    let (config_pda, _) = env.protocol_config_pda();
    let (fee_vault_pda, _) = env.protocol_fee_vault_pda();
    let metas =
        metas_admin_close_protocol(signer.pubkey(), config_pda, fee_vault_pda);
    let ix = build_ix(
        metas,
        poolver_core::instruction::AdminCloseProtocol {}.data(),
    );
    send_ix(&mut env.svm, signer, ix)
}

#[test]
fn t20_admin_close_protocol_happy_then_reinit_with_new_mint() {
    let mut env = TestEnv::new();
    let (config_pda, fee_vault_pda) = init_protocol(&mut env);

    // Pre-conditions: both accounts are live.
    assert!(env.account_exists(&config_pda));
    assert!(env.account_exists(&fee_vault_pda));

    // Close. Admin signs.
    let admin = env.admin.insecure_clone();
    admin_close_protocol(&mut env, &admin).expect("admin_close_protocol");

    // Both accounts must be gone (zero lamports / no data).
    assert!(
        !env.account_exists(&config_pda),
        "ProtocolConfig must be closed"
    );
    assert!(
        !env.account_exists(&fee_vault_pda),
        "protocol_fee_vault must be closed"
    );

    // Re-init with a *different* mint succeeds.
    let new_mint = env.create_extra_usdc_mint();
    assert_ne!(
        new_mint, env.usdc_mint,
        "test bug: extra mint must differ from original"
    );

    let metas = metas_initialize_protocol(
        env.admin.pubkey(),
        config_pda,
        new_mint,
        fee_vault_pda,
    );
    let ix = build_ix(
        metas,
        poolver_core::instruction::InitializeProtocol {}.data(),
    );
    send_ix(&mut env.svm, &admin, ix).expect("re-initialize_protocol with new mint");

    // The new ProtocolConfig points at the new mint.
    let cfg = env.fetch_protocol_config();
    assert_eq!(cfg.usdc_mint, new_mint, "rotation didn't take");
    assert_eq!(cfg.admin, env.admin.pubkey());
    assert_eq!(cfg.protocol_fee_vault, fee_vault_pda);
}

#[test]
fn t21_admin_close_protocol_rejects_non_admin() {
    let mut env = TestEnv::new();
    let (config_pda, fee_vault_pda) = init_protocol(&mut env);

    // Imposter tries to close.
    let imposter = Keypair::new();
    env.svm.airdrop(&imposter.pubkey(), 10 * SOL).unwrap();

    let res = admin_close_protocol(&mut env, &imposter);
    assert!(res.is_err(), "non-admin must NOT be able to close protocol");

    // Both accounts still live after the failed call.
    assert!(env.account_exists(&config_pda));
    assert!(env.account_exists(&fee_vault_pda));
}
