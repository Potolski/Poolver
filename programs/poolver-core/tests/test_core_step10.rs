//! Step-10 integration tests for `poolver-core`: default cascade
//! (`mark_late_payment`, `suspend_participant`, `liquidate_default`) +
//! the cure-path wiring inside `contribute` + the suspended check inside
//! `commit_bid` + the reputation gate inside `join_pool`.
//!
//! Coverage map (per task prompt §Tests):
//!   1.  mark_late_payment happy path                              → t300
//!   2.  rejected before month end (GracePeriodNotElapsed)         → t301
//!   3.  rejected when already paid                                → t302
//!   4.  rejected past day 6 (GracePeriodElapsed)                  → t303
//!   5.  single mark per month — repeated calls revert             → t304
//!   6.  suspend_participant happy path day 6+                     → t305
//!   7.  suspend rejected before day 6                             → t306
//!   8.  suspend blocks commit_bid for that user                   → t307
//!   9.  cure path: late participant cures via contribute          → t308
//!   10. liquidate_default Case A happy path (no shortfall)        → t309
//!   11. Case A with shortfall covered by reserve                  → t310
//!   12. Case A reserve insufficient → LiquidationShortfall        → t311
//!   13. Case B (non-winner default) — mark only                   → t312
//!   14. liquidate rejected before day 30                          → t313
//!   15. liquidate rejected when not suspended                     → t314
//!   16. liquidate rejected on double-liquidation                  → t315
//!   17. liquidate increments user_reputation.pools_defaulted      → t316
//!   18. reserve isolation: Tier 0 default w/ Tier 1 reserve fails → t317
//!   19. end-to-end cascade: mark → suspend → liquidate            → t318
//!   20. solvency post-liquidation (INV-1)                         → t319
//!   21. continuation after default — pool advances, defaulter
//!       cannot bid                                                 → t320
//!   22. cross-pool propagation (Q-11) — defaulter blocked from
//!       joining a new pool                                         → t321

#![cfg(feature = "mock-kyc")]

mod common;

use anchor_lang::{AccountDeserialize, AccountSerialize, InstructionData};
use common::*;
use solana_clock::Clock;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_sha256_hasher::hashv;
use solana_signer::Signer;

const ONE_DAY: i64 = 86_400;

// ───── Account-meta builders ────────────────────────────────────────────

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

#[allow(clippy::too_many_arguments)]
fn metas_commit_bid(
    user: Pubkey,
    protocol_config: Pubkey,
    pool: Pubkey,
    participant: Pubkey,
    user_kyc: Pubkey,
    bid: Pubkey,
    user_usdc: Pubkey,
    bid_stake_vault: Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new(user, true),
        AccountMeta::new_readonly(protocol_config, false),
        AccountMeta::new_readonly(pool, false),
        AccountMeta::new_readonly(participant, false),
        AccountMeta::new_readonly(user_kyc, false),
        AccountMeta::new(bid, false),
        AccountMeta::new(user_usdc, false),
        AccountMeta::new(bid_stake_vault, false),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
        AccountMeta::new_readonly(RENT_SYSVAR, false),
    ]
}

fn metas_mark_late(
    caller: Pubkey,
    protocol_config: Pubkey,
    pool: Pubkey,
    participant: Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new_readonly(caller, true),
        AccountMeta::new_readonly(protocol_config, false),
        AccountMeta::new_readonly(pool, false),
        AccountMeta::new(participant, false),
    ]
}

fn metas_suspend(
    caller: Pubkey,
    protocol_config: Pubkey,
    pool: Pubkey,
    participant: Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new_readonly(caller, true),
        AccountMeta::new_readonly(protocol_config, false),
        AccountMeta::new_readonly(pool, false),
        AccountMeta::new(participant, false),
    ]
}

#[allow(clippy::too_many_arguments)]
fn metas_liquidate(
    caller: Pubkey,
    protocol_config: Pubkey,
    pool: Pubkey,
    participant: Pubkey,
    user_reputation: Pubkey,
    pool_usdc_vault: Pubkey,
    collateral_vault: Pubkey,
    core_invoker: Pubkey,
    reserve_fund: Pubkey,
    reserve_usdc_vault: Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new_readonly(caller, true),
        AccountMeta::new_readonly(protocol_config, false),
        AccountMeta::new(pool, false),
        AccountMeta::new(participant, false),
        AccountMeta::new(user_reputation, false),
        AccountMeta::new(pool_usdc_vault, false),
        AccountMeta::new(collateral_vault, false),
        AccountMeta::new_readonly(core_invoker, false),
        AccountMeta::new(reserve_fund, false),
        AccountMeta::new(reserve_usdc_vault, false),
        AccountMeta::new_readonly(poolver_reserve::ID, false),
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

fn fully_set_up_user_full_kyc(env: &mut TestEnv, balance: u64) -> (Keypair, Pubkey) {
    let user = Keypair::new();
    env.svm.airdrop(&user.pubkey(), 100 * SOL).unwrap();
    issue_mock_kyc(env, &user.pubkey(), KycLevel::Full);
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
    env.svm.expire_blockhash();
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

fn contribute_for(
    env: &mut TestEnv,
    user: &Keypair,
    user_usdc: Pubkey,
    pool: Pubkey,
) -> Result<(), String> {
    env.svm.expire_blockhash();
    let (config_pda, _) = env.protocol_config_pda();
    let (user_rep, _) = env.reputation_pda(&user.pubkey());
    let (participant, _) = env.participant_pda(&pool, &user.pubkey());
    let (pool_usdc_vault, _) = env.pool_usdc_vault_pda(&pool);
    let (collat_vault, _) = env.collateral_vault_pda(&pool);
    let (fee_vault, _) = env.protocol_fee_vault_pda();
    let (reserve_fund, _) = env.reserve_fund_pda(Tier::Vault);
    let (reserve_usdc, _) = env.reserve_vault_pda(Tier::Vault);
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

fn advance_month_for(env: &mut TestEnv, caller: &Keypair, pool: Pubkey) -> Result<(), String> {
    env.svm.expire_blockhash();
    let (config_pda, _) = env.protocol_config_pda();
    let metas = metas_advance_month(caller.pubkey(), config_pda, pool);
    let ix = build_ix(metas, poolver_core::instruction::AdvanceMonth {}.data());
    send_ix(&mut env.svm, caller, ix)
}

#[allow(clippy::too_many_arguments)]
fn commit_bid_for(
    env: &mut TestEnv,
    user: &Keypair,
    user_usdc: Pubkey,
    pool: Pubkey,
    month: u8,
    commit_hash: [u8; 32],
) -> Result<(), String> {
    env.svm.expire_blockhash();
    let (config_pda, _) = env.protocol_config_pda();
    let (participant, _) = env.participant_pda(&pool, &user.pubkey());
    let (user_kyc, _) = env.kyc_pda(&user.pubkey());
    let (bid_pda, _) = env.bid_pda(&pool, month, &user.pubkey());
    let (bid_stake_vault, _) = env.bid_stake_vault_pda(&pool);

    let metas = metas_commit_bid(
        user.pubkey(),
        config_pda,
        pool,
        participant,
        user_kyc,
        bid_pda,
        user_usdc,
        bid_stake_vault,
    );
    let ix = build_ix(
        metas,
        poolver_core::instruction::CommitBid { commit_hash }.data(),
    );
    send_ix(&mut env.svm, user, ix)
}

fn mark_late_for(
    env: &mut TestEnv,
    caller: &Keypair,
    pool: Pubkey,
    target_user: Pubkey,
) -> Result<(), String> {
    env.svm.expire_blockhash();
    let (config_pda, _) = env.protocol_config_pda();
    let (participant, _) = env.participant_pda(&pool, &target_user);
    let metas = metas_mark_late(caller.pubkey(), config_pda, pool, participant);
    let ix = build_ix(
        metas,
        poolver_core::instruction::MarkLatePayment {}.data(),
    );
    send_ix(&mut env.svm, caller, ix)
}

fn suspend_for(
    env: &mut TestEnv,
    caller: &Keypair,
    pool: Pubkey,
    target_user: Pubkey,
) -> Result<(), String> {
    env.svm.expire_blockhash();
    let (config_pda, _) = env.protocol_config_pda();
    let (participant, _) = env.participant_pda(&pool, &target_user);
    let metas = metas_suspend(caller.pubkey(), config_pda, pool, participant);
    let ix = build_ix(
        metas,
        poolver_core::instruction::SuspendParticipant {}.data(),
    );
    send_ix(&mut env.svm, caller, ix)
}

#[allow(clippy::too_many_arguments)]
fn liquidate_for(
    env: &mut TestEnv,
    caller: &Keypair,
    pool: Pubkey,
    target_user: Pubkey,
    tier: Tier,
) -> Result<(), String> {
    env.svm.expire_blockhash();
    let (config_pda, _) = env.protocol_config_pda();
    let (participant, _) = env.participant_pda(&pool, &target_user);
    let (user_rep, _) = env.reputation_pda(&target_user);
    let (pool_usdc_vault, _) = env.pool_usdc_vault_pda(&pool);
    let (collat_vault, _) = env.collateral_vault_pda(&pool);
    let (reserve_fund, _) = env.reserve_fund_pda(tier);
    let (reserve_usdc, _) = env.reserve_vault_pda(tier);

    let metas = metas_liquidate(
        caller.pubkey(),
        config_pda,
        pool,
        participant,
        user_rep,
        pool_usdc_vault,
        collat_vault,
        env.core_invoker,
        reserve_fund,
        reserve_usdc,
    );
    let ix = build_ix(
        metas,
        poolver_core::instruction::LiquidateDefault {}.data(),
    );
    send_ix(&mut env.svm, caller, ix)
}

// ───── Time + setup helpers ─────────────────────────────────────────────

fn set_clock_to(env: &mut TestEnv, ts: i64) {
    let mut clock = env.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = ts;
    env.svm.set_sysvar::<Clock>(&clock);
}

fn pool_with_n_full_kyc(
    env: &mut TestEnv,
    pool_id: u64,
    contribution: u64,
    n: usize,
) -> (Pubkey, Vec<(Keypair, Pubkey)>) {
    assert!(n <= 12, "pool size capped at 12");
    let creator = bootstrap_with_creator(env);
    set_clock_to(env, 1_000_000);

    let pool = create_pool_for(env, &creator, pool_id, Tier::Vault, contribution)
        .expect("create_pool");

    let mut users = Vec::with_capacity(n);
    for _ in 0..n {
        let (user, ata) = fully_set_up_user_full_kyc(env, 200_000 * ONE_USDC);
        join_pool_for(env, &user, ata, pool, Tier::Vault).expect("join");
        users.push((user, ata));
    }
    (pool, users)
}

/// Force a participant into the post-win state (has_won + collateral).
/// Avoids driving 7 instructions to set up step-8's full claim flow.
/// Step-8 tests already cover the legitimate path; here we just need the
/// post-claim state to exercise step-10's liquidation path.
fn force_post_win_state(
    env: &mut TestEnv,
    pool_pk: Pubkey,
    user_pk: Pubkey,
    win_month: u8,
    bid_amount: u64,
    collateral: u64,
) {
    // Patch the Participant.
    let (part_pda, _) = env.participant_pda(&pool_pk, &user_pk);
    let mut acct = env.svm.get_account(&part_pda).unwrap().clone();
    let mut p = Participant::try_deserialize(&mut acct.data.as_ref()).unwrap();
    p.has_won = true;
    p.win_month = win_month;
    p.bid_amount_when_won = bid_amount;
    p.collateral_initial = collateral;
    p.collateral_locked = collateral;
    p.collateral_release_per_month = collateral / (Pool::TOTAL_MONTHS as u64 - win_month as u64).max(1);
    let mut buf = Vec::new();
    p.try_serialize(&mut buf).unwrap();
    acct.data[..buf.len()].copy_from_slice(&buf);
    env.svm.set_account(part_pda, acct).unwrap();

    // Fund the collateral vault with `collateral` USDC so the
    // liquidate_default has something to drain.
    let (collat_vault, _) = env.collateral_vault_pda(&pool_pk);
    let acct = env.svm.get_account(&collat_vault).unwrap().clone();
    let mut data = acct.data.clone();
    use solana_program_pack::Pack;
    use spl_token_interface::state::Account as SplTokenAccount;
    let mut ta = SplTokenAccount::unpack(&data).unwrap();
    ta.amount = ta.amount.saturating_add(collateral);
    SplTokenAccount::pack(ta, &mut data).unwrap();
    let mut new_acct = acct.clone();
    new_acct.data = data;
    env.svm.set_account(collat_vault, new_acct).unwrap();

    // Track on Pool.
    let mut acct = env.svm.get_account(&pool_pk).unwrap().clone();
    let mut pool = Pool::try_deserialize(&mut acct.data.as_ref()).unwrap();
    pool.total_collateral_locked = pool.total_collateral_locked.saturating_add(collateral);
    let mut buf = Vec::new();
    pool.try_serialize(&mut buf).unwrap();
    acct.data[..buf.len()].copy_from_slice(&buf);
    env.svm.set_account(pool_pk, acct).unwrap();
}

/// Seed reserve vault USDC + ReserveFund.total_balance directly (bypasses
/// the seed_reserve admin path which doesn't ship until step 11).
fn seed_reserve_balance(env: &mut TestEnv, tier: Tier, amount: u64) {
    let (reserve_fund, _) = env.reserve_fund_pda(tier);
    let mut acct = env.svm.get_account(&reserve_fund).unwrap().clone();
    let mut fund =
        poolver_reserve::state::ReserveFund::try_deserialize(&mut acct.data.as_ref()).unwrap();
    fund.total_balance = fund.total_balance.saturating_add(amount);
    fund.total_inflows = fund.total_inflows.saturating_add(amount);
    let mut buf = Vec::new();
    fund.try_serialize(&mut buf).unwrap();
    acct.data[..buf.len()].copy_from_slice(&buf);
    env.svm.set_account(reserve_fund, acct).unwrap();

    // Bump the SPL vault balance to match.
    let (reserve_vault, _) = env.reserve_vault_pda(tier);
    let acct = env.svm.get_account(&reserve_vault).unwrap().clone();
    let mut data = acct.data.clone();
    use solana_program_pack::Pack;
    use spl_token_interface::state::Account as SplTokenAccount;
    let mut ta = SplTokenAccount::unpack(&data).unwrap();
    ta.amount = ta.amount.saturating_add(amount);
    SplTokenAccount::pack(ta, &mut data).unwrap();
    let mut new_acct = acct.clone();
    new_acct.data = data;
    env.svm.set_account(reserve_vault, new_acct).unwrap();
}

fn make_commit_hash(bid_amount: u64, nonce: &[u8; 16], user: &Pubkey) -> [u8; 32] {
    let user_bytes = user.to_bytes();
    let amt = bid_amount.to_le_bytes();
    hashv(&[&amt, nonce, &user_bytes]).to_bytes()
}

// ─────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn t300_mark_late_payment_happy() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 300, contribution, 12);

    // Advance into month 2 and skip to day 1 of grace (just past month_end).
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    advance_month_for(&mut env, &caller, pool).unwrap();

    let p = env.fetch_pool(&pool);
    // Day 1 of month 2's grace = month_end + 1.
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 1,
    );

    // user[0] has not contributed for month 2 → mark_late accepts.
    let target = users[0].0.pubkey();
    mark_late_for(&mut env, &caller, pool, target).expect("mark_late happy");

    let (part_pda, _) = env.participant_pda(&pool, &target);
    let part = env.fetch_participant(&part_pda);
    assert!(part.is_late, "is_late flipped");
    assert!(part.late_marked_at >= p.current_month_started_at + p.month_duration_seconds);
    let expected_penalty = contribution * 200 / 10_000; // 200 bps
    assert_eq!(part.late_penalty_accrued, expected_penalty, "200 bps penalty accrued");
}

#[test]
fn t301_mark_late_rejected_before_month_end() {
    let mut env = TestEnv::new();
    let (pool, users) = pool_with_n_full_kyc(&mut env, 301, 1_000 * ONE_USDC, 12);

    // Still inside month 1 — month not elapsed.
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + 100);
    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();

    let target = users[0].0.pubkey();
    let res = mark_late_for(&mut env, &caller, pool, target);
    assert!(res.is_err(), "mark_late before month_end must reject");
}

#[test]
fn t302_mark_late_rejected_when_paid() {
    let mut env = TestEnv::new();
    let (pool, users) = pool_with_n_full_kyc(&mut env, 302, 1_000 * ONE_USDC, 12);

    // Advance to month 2.
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    advance_month_for(&mut env, &caller, pool).unwrap();

    // user[0] pays month 2 normally.
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + 10);
    let (u0, ata0) = (&users[0].0, users[0].1);
    contribute_for(&mut env, u0, ata0, pool).expect("month-2 contribute");

    // Skip past month-2 end — but user[0] has already paid.
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 100);

    let res = mark_late_for(&mut env, &caller, pool, u0.pubkey());
    assert!(res.is_err(), "mark_late on paid participant must reject");
}

#[test]
fn t303_mark_late_rejected_past_grace() {
    let mut env = TestEnv::new();
    let (pool, users) = pool_with_n_full_kyc(&mut env, 303, 1_000 * ONE_USDC, 12);

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    advance_month_for(&mut env, &caller, pool).unwrap();

    // Day 6+ of grace — past the day 1..=5 mark_late window.
    let p = env.fetch_pool(&pool);
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 6 * ONE_DAY,
    );

    let res = mark_late_for(&mut env, &caller, pool, users[0].0.pubkey());
    assert!(res.is_err(), "mark_late past day 5 must reject");
}

#[test]
fn t304_mark_late_rejected_double_mark() {
    let mut env = TestEnv::new();
    let (pool, users) = pool_with_n_full_kyc(&mut env, 304, 1_000 * ONE_USDC, 12);

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    advance_month_for(&mut env, &caller, pool).unwrap();

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);

    let target = users[0].0.pubkey();
    mark_late_for(&mut env, &caller, pool, target).expect("first mark");
    let res = mark_late_for(&mut env, &caller, pool, target);
    assert!(res.is_err(), "double mark must reject");
}

#[test]
fn t305_suspend_happy_day6() {
    let mut env = TestEnv::new();
    let (pool, users) = pool_with_n_full_kyc(&mut env, 305, 1_000 * ONE_USDC, 12);

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    advance_month_for(&mut env, &caller, pool).unwrap();

    // Day 6+ → suspend accepts.
    let p = env.fetch_pool(&pool);
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 6 * ONE_DAY,
    );

    let target = users[0].0.pubkey();
    suspend_for(&mut env, &caller, pool, target).expect("suspend day 6");

    let (part_pda, _) = env.participant_pda(&pool, &target);
    let part = env.fetch_participant(&part_pda);
    assert!(part.is_suspended, "is_suspended set");
    assert!(part.is_late, "defense-in-depth: is_late also set");
    assert!(part.suspended_at > 0);
}

#[test]
fn t306_suspend_rejected_before_day6() {
    let mut env = TestEnv::new();
    let (pool, users) = pool_with_n_full_kyc(&mut env, 306, 1_000 * ONE_USDC, 12);

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    advance_month_for(&mut env, &caller, pool).unwrap();

    // Day 3 of grace — before day 6 threshold.
    let p = env.fetch_pool(&pool);
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 3 * ONE_DAY,
    );

    let res = suspend_for(&mut env, &caller, pool, users[0].0.pubkey());
    assert!(res.is_err(), "suspend before day 6 must reject");
}

#[test]
fn t307_suspended_blocks_commit_bid() {
    let mut env = TestEnv::new();
    let (pool, users) = pool_with_n_full_kyc(&mut env, 307, 1_000 * ONE_USDC, 12);

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    advance_month_for(&mut env, &caller, pool).unwrap();

    // Suspend user[0] at day 6.
    let p = env.fetch_pool(&pool);
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 6 * ONE_DAY,
    );
    let target = users[0].0.pubkey();
    suspend_for(&mut env, &caller, pool, target).expect("suspend");

    // Now advance to month 3 so a fresh commit window is open.
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    advance_month_for(&mut env, &caller, pool).unwrap();
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + 1);

    // Suspended user attempts to commit_bid → must reject.
    let (u0, ata0) = (&users[0].0, users[0].1);
    let hash = make_commit_hash(100 * ONE_USDC, &[7u8; 16], &u0.pubkey());
    let res = commit_bid_for(&mut env, u0, ata0, pool, p.current_month, hash);
    assert!(res.is_err(), "suspended commit_bid must reject");
}

#[test]
fn t308_cure_path_via_contribute() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 308, contribution, 12);

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    advance_month_for(&mut env, &caller, pool).unwrap();

    let p = env.fetch_pool(&pool);
    // Day 1 of grace — mark late.
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    let (u0, ata0) = (&users[0].0, users[0].1);
    mark_late_for(&mut env, &caller, pool, u0.pubkey()).expect("mark_late");

    let credit_before = env.fetch_pool(&pool).bid_credit_balance;

    // Now cure: user[0] calls contribute. Day 7 of grace (still well
    // before liquidation). Penalty added to bid_credit_balance.
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 7 * ONE_DAY,
    );
    contribute_for(&mut env, u0, ata0, pool).expect("cure contribute");

    let (part_pda, _) = env.participant_pda(&pool, &u0.pubkey());
    let part = env.fetch_participant(&part_pda);
    assert!(!part.is_late, "is_late cleared on cure");
    assert!(!part.is_suspended, "is_suspended cleared on cure");
    assert_eq!(part.late_penalty_accrued, 0, "penalty consumed");
    assert_eq!(part.late_marked_at, 0);
    assert_eq!(part.suspended_at, 0);
    assert!(part.has_paid_month(p.current_month), "month marked paid");

    let credit_after = env.fetch_pool(&pool).bid_credit_balance;
    let penalty = contribution * 200 / 10_000;
    assert_eq!(
        credit_after,
        credit_before + penalty,
        "penalty routed to bid_credit_balance per Q-6"
    );
}

#[test]
fn t309_liquidate_case_a_no_shortfall() {
    // Post-win defaulter with collateral fully covering total_owed.
    // win_month = 2; current_month = 3. months_remaining = 12 - 3 + 1 = 10.
    // total_owed = 10 × 1000 = 10_000 USDC. Collateral_locked = 12_000
    // → drains 10_000 from collateral, no shortfall.
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 309, contribution, 12);

    // Advance to month 3.
    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    for _ in 0..2 {
        let p = env.fetch_pool(&pool);
        set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
        advance_month_for(&mut env, &caller, pool).unwrap();
    }

    let target = users[0].0.pubkey();
    let collateral = 12_000 * ONE_USDC;
    force_post_win_state(&mut env, pool, target, 2, 100 * ONE_USDC, collateral);

    // Suspend then liquidate at day 30+.
    let p = env.fetch_pool(&pool);
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 6 * ONE_DAY,
    );
    suspend_for(&mut env, &caller, pool, target).expect("suspend");

    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 31 * ONE_DAY,
    );

    let (pool_vault, _) = env.pool_usdc_vault_pda(&pool);
    let (collat_vault, _) = env.collateral_vault_pda(&pool);
    let (reserve_vault, _) = env.reserve_vault_pda(Tier::Vault);
    let pool_vault_before = env.fetch_token_balance(&pool_vault);
    let collat_before = env.fetch_token_balance(&collat_vault);
    let reserve_before = env.fetch_token_balance(&reserve_vault);

    liquidate_for(&mut env, &caller, pool, target, Tier::Vault).expect("liquidate Case A");

    // months_remaining = 12 - 3 + 1 = 10; total_owed = 10 × 1000 = 10_000.
    let total_owed = 10 * contribution;
    assert_eq!(
        env.fetch_token_balance(&collat_vault),
        collat_before - total_owed,
        "collateral vault drained by total_owed"
    );
    assert_eq!(
        env.fetch_token_balance(&pool_vault),
        pool_vault_before + total_owed,
        "pool vault funded by total_owed"
    );
    assert_eq!(
        env.fetch_token_balance(&reserve_vault),
        reserve_before,
        "reserve untouched (no shortfall)"
    );

    // Participant + reputation state.
    let (part_pda, _) = env.participant_pda(&pool, &target);
    let part = env.fetch_participant(&part_pda);
    assert!(part.is_defaulted);
    assert!(part.defaulted_at > 0);
    assert_eq!(part.collateral_locked, collateral - total_owed);
    assert_eq!(part.liquidation_amount, total_owed);

    let (rep_pda, _) = env.reputation_pda(&target);
    let rep = env.fetch_reputation(&rep_pda);
    assert_eq!(rep.pools_defaulted, 1);
}

#[test]
fn t310_liquidate_case_a_shortfall_covered_by_reserve() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 310, contribution, 12);

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    for _ in 0..2 {
        let p = env.fetch_pool(&pool);
        set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
        advance_month_for(&mut env, &caller, pool).unwrap();
    }

    let target = users[0].0.pubkey();
    // Under-collateralize: total_owed = 10_000, collateral = 6_000 →
    // shortfall = 4_000. Reserve has 5_000 USDC available → covers fully.
    let collateral = 6_000 * ONE_USDC;
    force_post_win_state(&mut env, pool, target, 2, 100 * ONE_USDC, collateral);
    seed_reserve_balance(&mut env, Tier::Vault, 5_000 * ONE_USDC);

    let p = env.fetch_pool(&pool);
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 6 * ONE_DAY,
    );
    suspend_for(&mut env, &caller, pool, target).expect("suspend");
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 31 * ONE_DAY,
    );

    let (pool_vault, _) = env.pool_usdc_vault_pda(&pool);
    let (reserve_vault, _) = env.reserve_vault_pda(Tier::Vault);
    let pool_before = env.fetch_token_balance(&pool_vault);
    let reserve_before = env.fetch_token_balance(&reserve_vault);

    liquidate_for(&mut env, &caller, pool, target, Tier::Vault)
        .expect("liquidate Case A with reserve coverage");

    let total_owed = 10 * contribution;
    let from_collateral = collateral;
    let from_reserve = total_owed - from_collateral;

    assert_eq!(
        env.fetch_token_balance(&pool_vault),
        pool_before + total_owed,
        "pool vault gets total_owed"
    );
    assert_eq!(
        env.fetch_token_balance(&reserve_vault),
        reserve_before - from_reserve,
        "reserve drawn by shortfall"
    );

    let (part_pda, _) = env.participant_pda(&pool, &target);
    let part = env.fetch_participant(&part_pda);
    assert_eq!(part.liquidation_amount, total_owed);
    assert_eq!(part.collateral_locked, 0);
}

#[test]
fn t311_liquidate_case_a_reserve_insufficient() {
    // Reserve has LESS than the shortfall → partial coverage, residual
    // shortfall recorded but liquidation still succeeds (arch §5.4).
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 311, contribution, 12);

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    for _ in 0..2 {
        let p = env.fetch_pool(&pool);
        set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
        advance_month_for(&mut env, &caller, pool).unwrap();
    }

    let target = users[0].0.pubkey();
    // Stage a HIGH total_owed by under-collateralizing aggressively:
    // collateral = 1_000, total_owed = 10_000 → shortfall = 9_000.
    // Reserve already has ~180 USDC from join-pool inflows; we don't
    // top it up. Drawable will be exactly the reserve's pre-existing
    // balance; residual ~8_820 USDC stays as recorded shortfall.
    let collateral = 1_000 * ONE_USDC;
    force_post_win_state(&mut env, pool, target, 2, 100 * ONE_USDC, collateral);

    let (reserve_vault, _) = env.reserve_vault_pda(Tier::Vault);
    let reserve_balance_before = env.fetch_token_balance(&reserve_vault);

    let p = env.fetch_pool(&pool);
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 6 * ONE_DAY,
    );
    suspend_for(&mut env, &caller, pool, target).expect("suspend");
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 31 * ONE_DAY,
    );

    let (pool_vault, _) = env.pool_usdc_vault_pda(&pool);
    let pool_before = env.fetch_token_balance(&pool_vault);

    liquidate_for(&mut env, &caller, pool, target, Tier::Vault)
        .expect("liquidate succeeds even with partial reserve");

    let total_owed = 10 * contribution; // 10_000
    let shortfall_required = total_owed - collateral; // 9_000
    // Drawable = min(shortfall, reserve_balance_before).
    let drawn_from_reserve = core::cmp::min(shortfall_required, reserve_balance_before);
    let actual_total = collateral + drawn_from_reserve;
    let residual_shortfall = shortfall_required - drawn_from_reserve;

    assert!(residual_shortfall > 0, "test scaffolding: reserve must be insufficient");
    assert_eq!(
        env.fetch_token_balance(&pool_vault),
        pool_before + actual_total
    );
    assert_eq!(
        env.fetch_token_balance(&reserve_vault),
        reserve_balance_before - drawn_from_reserve
    );

    let (part_pda, _) = env.participant_pda(&pool, &target);
    let part = env.fetch_participant(&part_pda);
    assert!(part.is_defaulted);
    assert_eq!(part.liquidation_amount, actual_total);
    // Off-chain reconstruction from `LiquidationShortfall` event:
    // residual_shortfall = total_owed - actual_total. Contract didn't
    // abort and participant is marked defaulted.
    let _ = residual_shortfall;
}

#[test]
fn t312_liquidate_case_b_non_winner() {
    // Non-winner default. NO collateral, NO token movement. Just mark.
    let mut env = TestEnv::new();
    let (pool, users) = pool_with_n_full_kyc(&mut env, 312, 1_000 * ONE_USDC, 12);

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    advance_month_for(&mut env, &caller, pool).unwrap();

    let target = users[0].0.pubkey();
    // user[0] never won. Just suspend then liquidate.
    let p = env.fetch_pool(&pool);
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 6 * ONE_DAY,
    );
    suspend_for(&mut env, &caller, pool, target).expect("suspend");

    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 31 * ONE_DAY,
    );

    let (pool_vault, _) = env.pool_usdc_vault_pda(&pool);
    let (collat_vault, _) = env.collateral_vault_pda(&pool);
    let (reserve_vault, _) = env.reserve_vault_pda(Tier::Vault);
    let pool_before = env.fetch_token_balance(&pool_vault);
    let collat_before = env.fetch_token_balance(&collat_vault);
    let reserve_before = env.fetch_token_balance(&reserve_vault);

    liquidate_for(&mut env, &caller, pool, target, Tier::Vault).expect("liquidate Case B");

    // Zero token movements.
    assert_eq!(env.fetch_token_balance(&pool_vault), pool_before);
    assert_eq!(env.fetch_token_balance(&collat_vault), collat_before);
    assert_eq!(env.fetch_token_balance(&reserve_vault), reserve_before);

    let (part_pda, _) = env.participant_pda(&pool, &target);
    let part = env.fetch_participant(&part_pda);
    assert!(part.is_defaulted);
    assert_eq!(part.liquidation_amount, 0);

    let (rep_pda, _) = env.reputation_pda(&target);
    let rep = env.fetch_reputation(&rep_pda);
    assert_eq!(rep.pools_defaulted, 1);
}

#[test]
fn t313_liquidate_rejected_before_day30() {
    let mut env = TestEnv::new();
    let (pool, users) = pool_with_n_full_kyc(&mut env, 313, 1_000 * ONE_USDC, 12);

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    advance_month_for(&mut env, &caller, pool).unwrap();

    let p = env.fetch_pool(&pool);
    let target = users[0].0.pubkey();
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 6 * ONE_DAY,
    );
    suspend_for(&mut env, &caller, pool, target).expect("suspend");

    // Day 15 — well before liquidation threshold (day 30).
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 15 * ONE_DAY,
    );
    let res = liquidate_for(&mut env, &caller, pool, target, Tier::Vault);
    assert!(res.is_err(), "liquidate before day 30 must reject");
}

#[test]
fn t314_liquidate_rejected_when_not_suspended() {
    let mut env = TestEnv::new();
    let (pool, users) = pool_with_n_full_kyc(&mut env, 314, 1_000 * ONE_USDC, 12);

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    advance_month_for(&mut env, &caller, pool).unwrap();

    let p = env.fetch_pool(&pool);
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 31 * ONE_DAY,
    );
    let target = users[0].0.pubkey();
    // Skip suspend step → liquidate must reject.
    let res = liquidate_for(&mut env, &caller, pool, target, Tier::Vault);
    assert!(res.is_err(), "liquidate w/o suspension must reject");
}

#[test]
fn t315_liquidate_rejected_double_liquidation() {
    let mut env = TestEnv::new();
    let (pool, users) = pool_with_n_full_kyc(&mut env, 315, 1_000 * ONE_USDC, 12);

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    advance_month_for(&mut env, &caller, pool).unwrap();

    let p = env.fetch_pool(&pool);
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 6 * ONE_DAY,
    );
    let target = users[0].0.pubkey();
    suspend_for(&mut env, &caller, pool, target).expect("suspend");

    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 31 * ONE_DAY,
    );
    liquidate_for(&mut env, &caller, pool, target, Tier::Vault).expect("first liquidate");
    let res = liquidate_for(&mut env, &caller, pool, target, Tier::Vault);
    assert!(res.is_err(), "double liquidation must reject");
}

#[test]
fn t316_liquidate_increments_pools_defaulted() {
    let mut env = TestEnv::new();
    let (pool, users) = pool_with_n_full_kyc(&mut env, 316, 1_000 * ONE_USDC, 12);

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    advance_month_for(&mut env, &caller, pool).unwrap();

    let p = env.fetch_pool(&pool);
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 6 * ONE_DAY,
    );
    let target = users[0].0.pubkey();
    suspend_for(&mut env, &caller, pool, target).expect("suspend");

    let (rep_pda, _) = env.reputation_pda(&target);
    let rep_before = env.fetch_reputation(&rep_pda);
    assert_eq!(rep_before.pools_defaulted, 0);

    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 31 * ONE_DAY,
    );
    liquidate_for(&mut env, &caller, pool, target, Tier::Vault).expect("liquidate");

    let rep_after = env.fetch_reputation(&rep_pda);
    assert_eq!(rep_after.pools_defaulted, 1);
}

#[test]
fn t317_reserve_isolation_wrong_tier() {
    // Pool is Tier 0; pass Tier 1 reserve → must reject before CPI.
    let mut env = TestEnv::new();
    let (pool, users) = pool_with_n_full_kyc(&mut env, 317, 1_000 * ONE_USDC, 12);

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    advance_month_for(&mut env, &caller, pool).unwrap();

    let p = env.fetch_pool(&pool);
    let target = users[0].0.pubkey();
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 6 * ONE_DAY,
    );
    suspend_for(&mut env, &caller, pool, target).expect("suspend");

    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 31 * ONE_DAY,
    );
    let res = liquidate_for(&mut env, &caller, pool, target, Tier::DeFi);
    assert!(res.is_err(), "wrong-tier reserve must reject");
}

#[test]
fn t318_e2e_cascade_mark_suspend_liquidate() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 318, contribution, 12);

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    advance_month_for(&mut env, &caller, pool).unwrap();

    let p = env.fetch_pool(&pool);
    let target = users[0].0.pubkey();

    // Day 1 → mark_late.
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    mark_late_for(&mut env, &caller, pool, target).expect("mark");

    // Day 6 → suspend.
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 6 * ONE_DAY,
    );
    suspend_for(&mut env, &caller, pool, target).expect("suspend");

    // Day 30 → liquidate (Case B — no collateral).
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 31 * ONE_DAY,
    );
    liquidate_for(&mut env, &caller, pool, target, Tier::Vault).expect("liquidate");

    let (part_pda, _) = env.participant_pda(&pool, &target);
    let part = env.fetch_participant(&part_pda);
    assert!(part.is_late);
    assert!(part.is_suspended);
    assert!(part.is_defaulted);
}

#[test]
fn t319_solvency_post_liquidation() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 319, contribution, 12);

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    for _ in 0..2 {
        let p = env.fetch_pool(&pool);
        set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
        advance_month_for(&mut env, &caller, pool).unwrap();
    }

    let target = users[0].0.pubkey();
    let collateral = 6_000 * ONE_USDC;
    force_post_win_state(&mut env, pool, target, 2, 100 * ONE_USDC, collateral);
    seed_reserve_balance(&mut env, Tier::Vault, 5_000 * ONE_USDC);

    let p = env.fetch_pool(&pool);
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 6 * ONE_DAY,
    );
    suspend_for(&mut env, &caller, pool, target).expect("suspend");
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 31 * ONE_DAY,
    );

    // Snapshot all four custody endpoints.
    let (pool_vault, _) = env.pool_usdc_vault_pda(&pool);
    let (collat_vault, _) = env.collateral_vault_pda(&pool);
    let (reserve_vault, _) = env.reserve_vault_pda(Tier::Vault);
    let (adapter_usdc, _) = env.vault_adapter_usdc_pda(&pool);

    let total_before = env.fetch_token_balance(&pool_vault)
        + env.fetch_token_balance(&collat_vault)
        + env.fetch_token_balance(&reserve_vault)
        + env.fetch_token_balance(&adapter_usdc);

    liquidate_for(&mut env, &caller, pool, target, Tier::Vault).expect("liquidate");

    let total_after = env.fetch_token_balance(&pool_vault)
        + env.fetch_token_balance(&collat_vault)
        + env.fetch_token_balance(&reserve_vault)
        + env.fetch_token_balance(&adapter_usdc);

    assert_eq!(
        total_before, total_after,
        "INV-1 solvency: liquidation is balance-preserving across custody endpoints"
    );
}

#[test]
fn t320_continuation_after_default() {
    // Post-liquidation: pool advances; defaulter cannot bid; other
    // participants contribute normally.
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 320, contribution, 12);

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    advance_month_for(&mut env, &caller, pool).unwrap();

    let p = env.fetch_pool(&pool);
    let target = users[0].0.pubkey();
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 6 * ONE_DAY,
    );
    suspend_for(&mut env, &caller, pool, target).expect("suspend");
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 31 * ONE_DAY,
    );
    liquidate_for(&mut env, &caller, pool, target, Tier::Vault).expect("liquidate");

    // advance to month 3; defaulter's commit_bid must fail; other users
    // can contribute normally.
    advance_month_for(&mut env, &caller, pool).unwrap();
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + 1);

    let (u0, ata0) = (&users[0].0, users[0].1);
    let hash = make_commit_hash(50 * ONE_USDC, &[1u8; 16], &u0.pubkey());
    let res = commit_bid_for(&mut env, u0, ata0, pool, p.current_month, hash);
    assert!(res.is_err(), "defaulter cannot commit_bid");

    // user[1] contributes normally.
    let (u1, ata1) = (&users[1].0, users[1].1);
    contribute_for(&mut env, u1, ata1, pool).expect("non-defaulter contribute");
}

#[test]
fn t321_cross_pool_propagation_q11() {
    // Default in pool A → user_reputation.pools_defaulted == 1 → blocked
    // from joining pool B (Q-11 reputation gate).
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool_a, users_a) = pool_with_n_full_kyc(&mut env, 401, contribution, 12);

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 1 * SOL).unwrap();
    let p = env.fetch_pool(&pool_a);
    set_clock_to(&mut env, p.current_month_started_at + p.month_duration_seconds + 1);
    advance_month_for(&mut env, &caller, pool_a).unwrap();

    let p = env.fetch_pool(&pool_a);
    let target_kp = users_a[0].0.insecure_clone();
    let target = target_kp.pubkey();
    let target_ata = users_a[0].1;

    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 6 * ONE_DAY,
    );
    suspend_for(&mut env, &caller, pool_a, target).expect("suspend");
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 31 * ONE_DAY,
    );
    liquidate_for(&mut env, &caller, pool_a, target, Tier::Vault).expect("liquidate");

    // user[0] now has pools_defaulted == 1. Try joining a fresh pool.
    let other_creator = bootstrap_with_creator_for_existing_protocol(&mut env);
    let pool_b = create_pool_for(&mut env, &other_creator, 999, Tier::Vault, contribution)
        .expect("create_pool B");
    // Top up the defaulter's USDC ATA so insufficient-balance isn't the
    // first failure mode.
    env.fund_token_account(&target, 200_000 * ONE_USDC);

    let res = join_pool_for(&mut env, &target_kp, target_ata, pool_b, Tier::Vault);
    assert!(
        res.is_err(),
        "Q-11 reputation gate must block defaulter from joining a new pool"
    );
}

/// Helper for t321: spin up a *second* creator (and pool) reusing the
/// already-initialized protocol_config + reserves. The full
/// `bootstrap_with_creator` unconditionally re-runs initialize_protocol
/// which double-init's; this helper just creates a fresh creator.
fn bootstrap_with_creator_for_existing_protocol(env: &mut TestEnv) -> Keypair {
    let creator = Keypair::new();
    env.svm.airdrop(&creator.pubkey(), 100 * SOL).unwrap();
    issue_mock_kyc(env, &creator.pubkey(), KycLevel::Light);
    init_reputation(env, &creator);
    creator
}
