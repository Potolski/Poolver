//! Step-6 integration tests for `poolver-core`: `commit_bid` +
//! `reveal_bid` (sealed-bid commit-reveal flow).
//!
//! Coverage map (per task prompt §Tests):
//!   1.  commit_bid happy path: Bid PDA created, stake locked, event ok → t60
//!   2.  commit_bid rejected outside bid window (before / after)        → t61
//!   3.  commit_bid rejected when participant has won                   → t62
//!   4.  commit_bid rejected when only Light KYC (Full required)        → t63
//!   5.  commit_bid rejected when KYC expired                           → t64
//!   6.  commit_bid rejected when sanctions_clean = false               → t65
//!   7.  commit_bid rejected on second commit same month (init)         → t66
//!   8.  commit_bid stake_amount = 1% of contribution_amount            → t67
//!   9.  reveal_bid happy path: revealed_amount stored, stake refunded  → t68
//!   10. reveal_bid rejected during commit window (BidWindowOpen)       → t69
//!   11. reveal_bid rejected after reveal expiry (BidWindowClosed)      → t70
//!   12. reveal_bid rejected when hash mismatches (BidRevealMismatch)   → t71
//!   13. reveal_bid rejected when bid_amount = 0                        → t72
//!   14. reveal_bid rejected when bid > 20% of pot (BidExceedsCap)      → t73
//!   15. reveal_bid rejected on second reveal (AlreadyRevealed)         → t74
//!   16. Bid cap math precise — boundary 2_328 ok, 2_329 rejects        → t75
//!   17. End-to-end: 12 commits → window close → 12 reveals             → t76
//!   18. Bid PDA tuple integrity: wrong (pool, month, user) rejects     → t77

#![cfg(feature = "mock-kyc")]

mod common;

use anchor_lang::{AccountDeserialize, AccountSerialize, InstructionData};
use common::*;
use solana_clock::Clock;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_sha256_hasher::hashv;

// ───── Account-meta builders (Anchor field order) ───────────────────────

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

fn metas_reveal_bid(
    user: Pubkey,
    protocol_config: Pubkey,
    pool: Pubkey,
    participant: Pubkey,
    bid: Pubkey,
    user_usdc: Pubkey,
    bid_stake_vault: Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new_readonly(user, true),
        AccountMeta::new_readonly(protocol_config, false),
        AccountMeta::new_readonly(pool, false),
        AccountMeta::new_readonly(participant, false),
        AccountMeta::new(bid, false),
        AccountMeta::new(user_usdc, false),
        AccountMeta::new(bid_stake_vault, false),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
    ]
}

#[allow(dead_code)]
fn metas_advance_month(
    caller: Pubkey,
    protocol_config: Pubkey,
    pool: Pubkey,
) -> Vec<AccountMeta> {
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

// ───── Reserve init helper ──────────────────────────────────────────────

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

// ───── High-level helpers ────────────────────────────────────────────────

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

fn bootstrap_with_creator(env: &mut TestEnv) -> Keypair {
    init_protocol(env);
    init_reserve_for(env, Tier::Vault);
    init_reserve_for(env, Tier::DeFi);

    let creator = Keypair::new();
    env.svm.airdrop(&creator.pubkey(), 100 * SOL).unwrap();
    // Creator joins with Light at create_pool time. We later upgrade
    // every participant (including the creator if they join the pool)
    // to Full when bidding.
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

/// Sets up a user with KYC = Full (required by `commit_bid`) + USDC ATA.
fn fully_set_up_user_full_kyc(env: &mut TestEnv, balance: u64) -> (Keypair, Pubkey) {
    let user = Keypair::new();
    env.svm.airdrop(&user.pubkey(), 100 * SOL).unwrap();
    issue_mock_kyc(env, &user.pubkey(), KycLevel::Full);
    init_reputation(env, &user);
    let ata = env.fund_token_account(&user.pubkey(), balance);
    (user, ata)
}

/// Same as above but only Light KYC. Used to test the
/// `KycInsufficientLevel` rejection on `commit_bid`.
#[allow(dead_code)]
fn fully_set_up_user_light_kyc(env: &mut TestEnv, balance: u64) -> (Keypair, Pubkey) {
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

#[allow(dead_code)]
fn advance_month_for(
    env: &mut TestEnv,
    caller: &Keypair,
    pool: Pubkey,
) -> Result<(), String> {
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

fn reveal_bid_for(
    env: &mut TestEnv,
    user: &Keypair,
    user_usdc: Pubkey,
    pool: Pubkey,
    month: u8,
    bid_amount: u64,
    nonce: [u8; 16],
) -> Result<(), String> {
    env.svm.expire_blockhash();
    let (config_pda, _) = env.protocol_config_pda();
    let (participant, _) = env.participant_pda(&pool, &user.pubkey());
    let (bid_pda, _) = env.bid_pda(&pool, month, &user.pubkey());
    let (bid_stake_vault, _) = env.bid_stake_vault_pda(&pool);

    let metas = metas_reveal_bid(
        user.pubkey(),
        config_pda,
        pool,
        participant,
        bid_pda,
        user_usdc,
        bid_stake_vault,
    );
    let ix = build_ix(
        metas,
        poolver_core::instruction::RevealBid { bid_amount, nonce }.data(),
    );
    send_ix(&mut env.svm, user, ix)
}

// ───── Time helpers ──────────────────────────────────────────────────────

fn set_clock_to(env: &mut TestEnv, ts: i64) {
    let mut clock = env.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = ts;
    env.svm.set_sysvar::<Clock>(&clock);
}

// ───── Hash helper (same shape as the reveal handler) ───────────────────

fn make_commit_hash(bid_amount: u64, nonce: &[u8; 16], user: &Pubkey) -> [u8; 32] {
    let user_bytes = user.to_bytes();
    let amt = bid_amount.to_le_bytes();
    hashv(&[&amt, nonce, &user_bytes]).to_bytes()
}

// ───── Setup builder: pool with 12 Full-KYC participants, in month 1 ────

/// Bootstraps a pool with 12 participants (all Full KYC). Returns
/// (pool_pda, list of (user, ata)). Sets clock so we are inside the
/// month-1 commit window.
fn pool_with_12_full_kyc(
    env: &mut TestEnv,
    pool_id: u64,
    contribution: u64,
    month_duration: Option<i64>,
) -> (Pubkey, Vec<(Keypair, Pubkey)>) {
    let creator = bootstrap_with_creator(env);
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
        // 50_000 USDC funding so the user can pay both the join contribution
        // and any subsequent bid stake without rebuying tokens.
        let (user, ata) = fully_set_up_user_full_kyc(env, 50_000 * ONE_USDC);
        join_pool_for(env, &user, ata, pool, Tier::Vault).expect("join");
        users.push((user, ata));
    }

    let p = env.fetch_pool(&pool);
    assert_eq!(p.current_month, 1, "pool must auto-start");
    (pool, users)
}

// ─────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn t60_commit_bid_happy_path() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_12_full_kyc(&mut env, 60, contribution, None);

    // Move clock inside the bid window (just after pool start).
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + 1);

    let (user, ata) = (&users[0].0, users[0].1);
    let nonce = [7u8; 16];
    let bid_amount = 500 * ONE_USDC;
    let hash = make_commit_hash(bid_amount, &nonce, &user.pubkey());

    let user_before = env.fetch_token_balance(&ata);
    let (stake_vault, _) = env.bid_stake_vault_pda(&pool);
    let stake_vault_before = env.fetch_token_balance(&stake_vault);

    commit_bid_for(&mut env, user, ata, pool, p.current_month, hash).expect("commit_bid");

    let expected_stake = contribution * 100 / 10_000; // 1%
    assert_eq!(
        env.fetch_token_balance(&ata),
        user_before - expected_stake,
        "user paid stake from ATA"
    );
    assert_eq!(
        env.fetch_token_balance(&stake_vault),
        stake_vault_before + expected_stake,
        "stake escrowed"
    );

    let (bid_pda, _) = env.bid_pda(&pool, p.current_month, &user.pubkey());
    let bid = env.fetch_bid(&bid_pda);
    assert_eq!(bid.pool, pool);
    assert_eq!(bid.user, user.pubkey());
    assert_eq!(bid.month, p.current_month);
    assert_eq!(bid.commit_hash, hash);
    assert_eq!(bid.stake_amount, expected_stake);
    assert!(!bid.revealed);
    assert_eq!(bid.revealed_amount, 0);
    assert!(!bid.is_winner);
    assert!(!bid.stake_refunded);
}

#[test]
fn t61_commit_bid_rejected_outside_window() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_12_full_kyc(&mut env, 61, contribution, None);
    let (user, ata) = (&users[0].0, users[0].1);

    let p = env.fetch_pool(&pool);
    let nonce = [1u8; 16];
    let hash = make_commit_hash(500 * ONE_USDC, &nonce, &user.pubkey());

    // (a) Past the bid_window_ends_at boundary → BidWindowClosed.
    set_clock_to(&mut env, p.bid_window_ends_at + 1);
    let res = commit_bid_for(&mut env, user, ata, pool, p.current_month, hash);
    assert!(res.is_err(), "after bid window must reject");

    // (b) Before pool started (current_month_started_at - 1 second).
    set_clock_to(&mut env, p.current_month_started_at - 1);
    let res = commit_bid_for(&mut env, user, ata, pool, p.current_month, hash);
    assert!(res.is_err(), "before window open must reject");
}

#[test]
fn t62_commit_bid_rejected_when_has_won() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_12_full_kyc(&mut env, 62, contribution, None);
    let (user, ata) = (&users[0].0, users[0].1);

    // Force-mark this participant as has_won (as if they won an earlier
    // month). Step 7's `select_winner` is the real path; here we forge
    // state to test the gate in isolation.
    let (part_pda, _) = env.participant_pda(&pool, &user.pubkey());
    let mut acct = env.svm.get_account(&part_pda).unwrap().clone();
    let mut part = Participant::try_deserialize(&mut acct.data.as_ref()).unwrap();
    part.has_won = true;
    part.win_month = 1;
    let mut buf = Vec::new();
    part.try_serialize(&mut buf).unwrap();
    acct.data[..buf.len()].copy_from_slice(&buf);
    env.svm.set_account(part_pda, acct).unwrap();

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + 1);
    let nonce = [2u8; 16];
    let hash = make_commit_hash(500 * ONE_USDC, &nonce, &user.pubkey());
    let res = commit_bid_for(&mut env, user, ata, pool, p.current_month, hash);
    assert!(res.is_err(), "winner must not be allowed to bid again");
}

#[test]
fn t63_commit_bid_rejected_with_only_light_kyc() {
    // Pool is bootstrapped with all Full-KYC participants. We'll hijack
    // user[0]'s KYC attestation, downgrading it to Light, and verify the
    // commit is rejected with `KycInsufficientLevel`.
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_12_full_kyc(&mut env, 63, contribution, None);
    let (user, ata) = (&users[0].0, users[0].1);

    let (kyc_pda, _) = env.kyc_pda(&user.pubkey());
    let mut acct = env.svm.get_account(&kyc_pda).unwrap().clone();
    let mut kyc = KycAttestation::try_deserialize(&mut acct.data.as_ref()).unwrap();
    kyc.level = KycLevel::Light.as_u8();
    let mut buf = Vec::new();
    kyc.try_serialize(&mut buf).unwrap();
    acct.data[..buf.len()].copy_from_slice(&buf);
    env.svm.set_account(kyc_pda, acct).unwrap();

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + 1);
    let nonce = [3u8; 16];
    let hash = make_commit_hash(500 * ONE_USDC, &nonce, &user.pubkey());
    let res = commit_bid_for(&mut env, user, ata, pool, p.current_month, hash);
    assert!(res.is_err(), "Light KYC must be rejected (Full required)");
}

#[test]
fn t64_commit_bid_rejected_when_kyc_expired() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_12_full_kyc(&mut env, 64, contribution, None);
    let (user, ata) = (&users[0].0, users[0].1);

    let (kyc_pda, _) = env.kyc_pda(&user.pubkey());
    let mut acct = env.svm.get_account(&kyc_pda).unwrap().clone();
    let mut kyc = KycAttestation::try_deserialize(&mut acct.data.as_ref()).unwrap();
    kyc.expires_at = 100; // far in the past relative to clock
    let mut buf = Vec::new();
    kyc.try_serialize(&mut buf).unwrap();
    acct.data[..buf.len()].copy_from_slice(&buf);
    env.svm.set_account(kyc_pda, acct).unwrap();

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + 1);
    let nonce = [4u8; 16];
    let hash = make_commit_hash(500 * ONE_USDC, &nonce, &user.pubkey());
    let res = commit_bid_for(&mut env, user, ata, pool, p.current_month, hash);
    assert!(res.is_err(), "expired KYC must reject");
}

#[test]
fn t65_commit_bid_rejected_when_sanctions_hit() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_12_full_kyc(&mut env, 65, contribution, None);
    let (user, ata) = (&users[0].0, users[0].1);

    let (kyc_pda, _) = env.kyc_pda(&user.pubkey());
    let mut acct = env.svm.get_account(&kyc_pda).unwrap().clone();
    let mut kyc = KycAttestation::try_deserialize(&mut acct.data.as_ref()).unwrap();
    kyc.sanctions_clean = false;
    let mut buf = Vec::new();
    kyc.try_serialize(&mut buf).unwrap();
    acct.data[..buf.len()].copy_from_slice(&buf);
    env.svm.set_account(kyc_pda, acct).unwrap();

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + 1);
    let nonce = [5u8; 16];
    let hash = make_commit_hash(500 * ONE_USDC, &nonce, &user.pubkey());
    let res = commit_bid_for(&mut env, user, ata, pool, p.current_month, hash);
    assert!(res.is_err(), "sanctions hit must reject");
}

#[test]
fn t66_commit_bid_rejected_double_commit() {
    // INV-16: PDA `init` enforces single commit per (pool, month, user).
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_12_full_kyc(&mut env, 66, contribution, None);
    let (user, ata) = (&users[0].0, users[0].1);

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + 1);
    let nonce = [6u8; 16];
    let hash = make_commit_hash(500 * ONE_USDC, &nonce, &user.pubkey());
    commit_bid_for(&mut env, user, ata, pool, p.current_month, hash).expect("first");

    let res = commit_bid_for(&mut env, user, ata, pool, p.current_month, hash);
    assert!(res.is_err(), "second commit must fail (Bid PDA already initialized)");
}

#[test]
fn t67_commit_bid_stake_amount_is_one_percent() {
    let mut env = TestEnv::new();
    let contribution = 2_500 * ONE_USDC;
    let (pool, users) = pool_with_12_full_kyc(&mut env, 67, contribution, None);
    let (user, ata) = (&users[0].0, users[0].1);

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + 1);
    let nonce = [8u8; 16];
    let hash = make_commit_hash(100 * ONE_USDC, &nonce, &user.pubkey());
    commit_bid_for(&mut env, user, ata, pool, p.current_month, hash).expect("commit");

    let (bid_pda, _) = env.bid_pda(&pool, p.current_month, &user.pubkey());
    let bid = env.fetch_bid(&bid_pda);
    // 1% of 2500 USDC = 25 USDC.
    assert_eq!(bid.stake_amount, 25 * ONE_USDC);
}

#[test]
fn t68_reveal_bid_happy_path() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_12_full_kyc(&mut env, 68, contribution, None);
    let (user, ata) = (&users[0].0, users[0].1);

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + 1);
    let nonce = [9u8; 16];
    let bid_amount = 500 * ONE_USDC;
    let hash = make_commit_hash(bid_amount, &nonce, &user.pubkey());
    commit_bid_for(&mut env, user, ata, pool, p.current_month, hash).expect("commit");

    let user_after_commit = env.fetch_token_balance(&ata);

    // Move clock into the reveal window.
    let p2 = env.fetch_pool(&pool);
    set_clock_to(&mut env, p2.bid_window_ends_at + 1);

    reveal_bid_for(&mut env, user, ata, pool, p.current_month, bid_amount, nonce)
        .expect("reveal");

    let (bid_pda, _) = env.bid_pda(&pool, p.current_month, &user.pubkey());
    let bid = env.fetch_bid(&bid_pda);
    assert!(bid.revealed);
    assert_eq!(bid.revealed_amount, bid_amount);
    assert!(bid.revealed_at > 0);
    assert!(bid.stake_refunded);

    let stake_amount = contribution * 100 / 10_000;
    assert_eq!(
        env.fetch_token_balance(&ata),
        user_after_commit + stake_amount,
        "stake refunded back to user"
    );
}

#[test]
fn t69_reveal_bid_rejected_during_commit_window() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_12_full_kyc(&mut env, 69, contribution, None);
    let (user, ata) = (&users[0].0, users[0].1);

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + 1);
    let nonce = [10u8; 16];
    let bid_amount = 500 * ONE_USDC;
    let hash = make_commit_hash(bid_amount, &nonce, &user.pubkey());
    commit_bid_for(&mut env, user, ata, pool, p.current_month, hash).expect("commit");

    // Stay inside commit window — reveal must reject with BidWindowOpen.
    let res = reveal_bid_for(
        &mut env,
        user,
        ata,
        pool,
        p.current_month,
        bid_amount,
        nonce,
    );
    assert!(res.is_err(), "reveal during commit window must reject");
}

#[test]
fn t70_reveal_bid_rejected_after_reveal_expiry() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_12_full_kyc(&mut env, 70, contribution, None);
    let (user, ata) = (&users[0].0, users[0].1);

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + 1);
    let nonce = [11u8; 16];
    let bid_amount = 500 * ONE_USDC;
    let hash = make_commit_hash(bid_amount, &nonce, &user.pubkey());
    commit_bid_for(&mut env, user, ata, pool, p.current_month, hash).expect("commit");

    // Jump past reveal_window_ends_at.
    set_clock_to(&mut env, p.reveal_window_ends_at + 1);
    let res = reveal_bid_for(
        &mut env,
        user,
        ata,
        pool,
        p.current_month,
        bid_amount,
        nonce,
    );
    assert!(res.is_err(), "reveal after reveal expiry must reject");
}

#[test]
fn t71_reveal_bid_rejected_on_hash_mismatch() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_12_full_kyc(&mut env, 71, contribution, None);
    let (user, ata) = (&users[0].0, users[0].1);

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + 1);
    let real_nonce = [12u8; 16];
    let real_amount = 500 * ONE_USDC;
    let hash = make_commit_hash(real_amount, &real_nonce, &user.pubkey());
    commit_bid_for(&mut env, user, ata, pool, p.current_month, hash).expect("commit");

    set_clock_to(&mut env, p.bid_window_ends_at + 1);
    // Try to reveal with a different (amount, nonce). Hash will not match.
    let bad_amount = 600 * ONE_USDC;
    let res = reveal_bid_for(
        &mut env,
        user,
        ata,
        pool,
        p.current_month,
        bad_amount,
        real_nonce,
    );
    assert!(res.is_err(), "bad amount must hash-mismatch");

    let res = reveal_bid_for(
        &mut env,
        user,
        ata,
        pool,
        p.current_month,
        real_amount,
        [99u8; 16],
    );
    assert!(res.is_err(), "bad nonce must hash-mismatch");
}

#[test]
fn t72_reveal_bid_rejected_on_zero_amount() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_12_full_kyc(&mut env, 72, contribution, None);
    let (user, ata) = (&users[0].0, users[0].1);

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + 1);
    // Commit with bid_amount = 0; the hash math is still well-defined.
    let nonce = [13u8; 16];
    let hash = make_commit_hash(0, &nonce, &user.pubkey());
    commit_bid_for(&mut env, user, ata, pool, p.current_month, hash).expect("commit");

    set_clock_to(&mut env, p.bid_window_ends_at + 1);
    let res = reveal_bid_for(&mut env, user, ata, pool, p.current_month, 0, nonce);
    assert!(res.is_err(), "zero amount reveal must reject");
}

#[test]
fn t73_reveal_bid_rejected_when_above_cap() {
    let mut env = TestEnv::new();
    // contribution = 1000 USDC → fees = 30 → net = 970 → pot = 11_640 →
    // cap = 2_328 microUSDC * 1e6 ... wait — values are in microUSDC
    // already (ONE_USDC = 1_000_000). Restate:
    //   contribution_amount = 1000 * 1e6 = 1_000_000_000 (microUSDC)
    //   protocol_fee = 1_000_000_000 * 150 / 10_000 = 15_000_000
    //   reserve_fee  = 1_000_000_000 * 150 / 10_000 = 15_000_000
    //   net = 970_000_000
    //   pot = 12 * net = 11_640_000_000
    //   cap = pot * 2000 / 10_000 = 2_328_000_000
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_12_full_kyc(&mut env, 73, contribution, None);
    let (user, ata) = (&users[0].0, users[0].1);

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + 1);
    let nonce = [14u8; 16];
    // Bid 1 microUSDC over the cap.
    let above = 2_328_000_001u64;
    let hash = make_commit_hash(above, &nonce, &user.pubkey());
    commit_bid_for(&mut env, user, ata, pool, p.current_month, hash).expect("commit");

    set_clock_to(&mut env, p.bid_window_ends_at + 1);
    let res = reveal_bid_for(&mut env, user, ata, pool, p.current_month, above, nonce);
    assert!(res.is_err(), "bid above cap must reject");
}

#[test]
fn t74_reveal_bid_rejected_on_double_reveal() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_12_full_kyc(&mut env, 74, contribution, None);
    let (user, ata) = (&users[0].0, users[0].1);

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + 1);
    let nonce = [15u8; 16];
    let bid_amount = 500 * ONE_USDC;
    let hash = make_commit_hash(bid_amount, &nonce, &user.pubkey());
    commit_bid_for(&mut env, user, ata, pool, p.current_month, hash).expect("commit");

    set_clock_to(&mut env, p.bid_window_ends_at + 1);
    reveal_bid_for(&mut env, user, ata, pool, p.current_month, bid_amount, nonce)
        .expect("first reveal");

    let res = reveal_bid_for(
        &mut env,
        user,
        ata,
        pool,
        p.current_month,
        bid_amount,
        nonce,
    );
    assert!(res.is_err(), "second reveal must reject (AlreadyRevealed)");
}

#[test]
fn t75_bid_cap_math_boundary() {
    // contribution = 1000 USDC, Vault tier:
    //   protocol_fee = 15 USDC
    //   reserve_fee  = 15 USDC
    //   net          = 970 USDC
    //   pot          = 12 × 970 = 11_640 USDC
    //   cap          = 11_640 × 0.20 = 2_328 USDC = 2_328_000_000 microUSDC
    // Boundary: bid = cap (passes) vs bid = cap + 1 (rejects).
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_12_full_kyc(&mut env, 75, contribution, None);

    // Use TWO different users so each can lock their own Bid PDA.
    let (user_pass, ata_pass) = (&users[0].0, users[0].1);
    let (user_fail, ata_fail) = (&users[1].0, users[1].1);

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + 1);

    let cap_microusdc = 2_328_000_000u64;
    let nonce_pass = [16u8; 16];
    let nonce_fail = [17u8; 16];
    let hash_pass = make_commit_hash(cap_microusdc, &nonce_pass, &user_pass.pubkey());
    let hash_fail = make_commit_hash(cap_microusdc + 1, &nonce_fail, &user_fail.pubkey());

    commit_bid_for(&mut env, user_pass, ata_pass, pool, p.current_month, hash_pass)
        .expect("commit pass");
    commit_bid_for(&mut env, user_fail, ata_fail, pool, p.current_month, hash_fail)
        .expect("commit fail");

    set_clock_to(&mut env, p.bid_window_ends_at + 1);

    reveal_bid_for(
        &mut env,
        user_pass,
        ata_pass,
        pool,
        p.current_month,
        cap_microusdc,
        nonce_pass,
    )
    .expect("reveal at exact cap must pass");

    let res = reveal_bid_for(
        &mut env,
        user_fail,
        ata_fail,
        pool,
        p.current_month,
        cap_microusdc + 1,
        nonce_fail,
    );
    assert!(res.is_err(), "reveal one microUSDC above cap must fail");
}

#[test]
fn t76_e2e_12_commits_then_12_reveals() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_12_full_kyc(&mut env, 76, contribution, None);

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + 1);

    let stake_amount = contribution * 100 / 10_000;
    // Pre-balances of all users.
    let pre_balances: Vec<u64> = users
        .iter()
        .map(|(_, ata)| env.fetch_token_balance(ata))
        .collect();

    // Each of 12 participants commits a unique nonce + amount.
    let mut nonces = Vec::with_capacity(12);
    let mut amounts = Vec::with_capacity(12);
    for (i, (user, ata)) in users.iter().enumerate() {
        let nonce = {
            let mut n = [0u8; 16];
            n[0] = (i as u8) + 1;
            n
        };
        let amount = (100u64 + i as u64 * 50) * ONE_USDC;
        let hash = make_commit_hash(amount, &nonce, &user.pubkey());
        commit_bid_for(&mut env, user, *ata, pool, p.current_month, hash)
            .unwrap_or_else(|e| panic!("commit {} failed: {}", i, e));
        nonces.push(nonce);
        amounts.push(amount);
    }

    // After commits: every user should have lost `stake_amount`.
    for (i, (_, ata)) in users.iter().enumerate() {
        assert_eq!(
            env.fetch_token_balance(ata),
            pre_balances[i] - stake_amount,
            "user {} stake locked",
            i
        );
    }

    // Move into the reveal window.
    set_clock_to(&mut env, p.bid_window_ends_at + 1);

    // 12 reveals.
    for (i, (user, ata)) in users.iter().enumerate() {
        reveal_bid_for(
            &mut env,
            user,
            *ata,
            pool,
            p.current_month,
            amounts[i],
            nonces[i],
        )
        .unwrap_or_else(|e| panic!("reveal {} failed: {}", i, e));
    }

    // Post-reveal: every user should be back at their pre-commit balance.
    for (i, (user, ata)) in users.iter().enumerate() {
        assert_eq!(
            env.fetch_token_balance(ata),
            pre_balances[i],
            "user {} stake refunded",
            i
        );
        let (bid_pda, _) = env.bid_pda(&pool, p.current_month, &user.pubkey());
        let bid = env.fetch_bid(&bid_pda);
        assert!(bid.revealed);
        assert_eq!(bid.revealed_amount, amounts[i]);
        assert!(bid.stake_refunded);
    }

    // Stake vault should be empty (all 12 stakes refunded).
    let (stake_vault, _) = env.bid_stake_vault_pda(&pool);
    assert_eq!(env.fetch_token_balance(&stake_vault), 0);
}

#[test]
fn t77_bid_pda_tuple_integrity() {
    // Anchor PDA derivation must reject a Bid PDA whose seeds don't
    // match the (pool, month, user) tuple of the live transaction. We
    // attempt to commit by passing user_a's `bid` PDA but signing as
    // user_b. The seed binding `[BID_SEED, pool, month, user_b]` will
    // not equal user_a's PDA, so Anchor rejects before any handler logic.
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_12_full_kyc(&mut env, 77, contribution, None);

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + 1);

    let (user_a, ata_a) = (&users[0].0, users[0].1);
    let (user_b, _ata_b) = (&users[1].0, users[1].1);

    // Bid PDA derived for user_a (intentional mismatch with signer).
    let (bid_pda_a, _) = env.bid_pda(&pool, p.current_month, &user_a.pubkey());

    let nonce = [18u8; 16];
    let hash = make_commit_hash(500 * ONE_USDC, &nonce, &user_b.pubkey());

    let (config_pda, _) = env.protocol_config_pda();
    let (participant_b, _) = env.participant_pda(&pool, &user_b.pubkey());
    let (kyc_b, _) = env.kyc_pda(&user_b.pubkey());
    let (stake_vault, _) = env.bid_stake_vault_pda(&pool);

    let metas = metas_commit_bid(
        user_b.pubkey(),
        config_pda,
        pool,
        participant_b,
        kyc_b,
        bid_pda_a,
        ata_a,
        stake_vault,
    );
    let ix = build_ix(
        metas,
        poolver_core::instruction::CommitBid { commit_hash: hash }.data(),
    );
    env.svm.expire_blockhash();
    let res = send_ix(&mut env.svm, user_b, ix);
    assert!(
        res.is_err(),
        "wrong (pool, month, user) tuple must be rejected by seed derivation"
    );
}

