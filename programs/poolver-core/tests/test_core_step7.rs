//! Step-7 integration tests for `poolver-core`: `select_winner`.
//!
//! Coverage map (per task prompt §Tests):
//!   1.  Single revealed bid → that user wins (Bid path)               → t80
//!   2.  Multiple revealed bids → highest wins                         → t81
//!   3.  Tie-break via deterministic Q-2 hash                          → t82
//!   4.  Zero revealed bids → lottery picks valid candidate            → t83
//!   5.  Rejected before reveal window closes (BidWindowOpen)          → t84
//!   6.  Rejected when winner already selected (WinnerAlreadySelected) → t85
//!   7.  Rejected when paused / pool complete                          → t86
//!   8.  Stake forfeit: unrevealed bid drained to tier reserve         → t87
//!   9.  Light-KYC bidder filtered out (revealed but ineligible)       → t88
//!   10. Previous winner filtered out                                   → t89
//!   11. Defaulted participant filtered out                            → t90
//!   12. NoEligibleParticipants when 0 lottery candidates              → t91
//!   13. End-to-end month-1 selection with 12 participants, 6 bids     → t92
//!   14. Reserve isolation: wrong-tier reserve fails                   → t93
//!
//! V1 banner (Q-21): the lottery branch uses sha256(pool || month ||
//! slot) as a mock VRF seed. Tests assert the chosen lottery candidate
//! is *one of* the eligible set (not a specific index — slot-dependent).

#![cfg(feature = "mock-kyc")]

mod common;

use anchor_lang::{AccountDeserialize, AccountSerialize, Discriminator, InstructionData};
use common::*;
use solana_clock::Clock;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_sha256_hasher::hashv;
use solana_signer::Signer;

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

#[allow(clippy::too_many_arguments)]
fn metas_select_winner(
    caller: Pubkey,
    protocol_config: Pubkey,
    pool: Pubkey,
    bid_stake_vault: Pubkey,
    core_invoker: Pubkey,
    reserve_fund: Pubkey,
    reserve_usdc_vault: Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new(caller, true),
        AccountMeta::new_readonly(protocol_config, false),
        AccountMeta::new(pool, false),
        AccountMeta::new(bid_stake_vault, false),
        AccountMeta::new_readonly(core_invoker, false),
        AccountMeta::new(reserve_fund, false),
        AccountMeta::new(reserve_usdc_vault, false),
        AccountMeta::new_readonly(poolver_reserve::ID, false),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
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

// ───── High-level helpers ───────────────────────────────────────────────

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

fn advance_slot(env: &mut TestEnv) {
    let mut clock = env.svm.get_sysvar::<Clock>();
    clock.slot = clock.slot.saturating_add(1);
    env.svm.set_sysvar::<Clock>(&clock);
}

// ───── Hash helpers ──────────────────────────────────────────────────────

fn make_commit_hash(bid_amount: u64, nonce: &[u8; 16], user: &Pubkey) -> [u8; 32] {
    let user_bytes = user.to_bytes();
    let amt = bid_amount.to_le_bytes();
    hashv(&[&amt, nonce, &user_bytes]).to_bytes()
}

// ───── Setup builder ────────────────────────────────────────────────────

/// Bootstraps a pool with N participants (all Full KYC), in month 1,
/// with the clock inside the commit window. `n` ≤ 12.
fn pool_with_n_full_kyc(
    env: &mut TestEnv,
    pool_id: u64,
    contribution: u64,
    n: usize,
) -> (Pubkey, Vec<(Keypair, Pubkey)>) {
    assert!(n <= 12, "pool size capped at 12");
    let creator = bootstrap_with_creator(env);
    set_clock_to(env, 1_000_000);

    let pool = create_pool_for(env, &creator, pool_id, Tier::Vault, contribution, None)
        .expect("create_pool");

    let mut users = Vec::with_capacity(n);
    for _ in 0..n {
        let (user, ata) = fully_set_up_user_full_kyc(env, 50_000 * ONE_USDC);
        join_pool_for(env, &user, ata, pool, Tier::Vault).expect("join");
        users.push((user, ata));
    }

    let p = env.fetch_pool(&pool);
    if n == 12 {
        assert_eq!(p.current_month, 1, "pool must auto-start with 12 joins");
    }
    (pool, users)
}

// ───── select_winner sender (with remaining_accounts builder) ────────────

/// Build a `select_winner` ix where:
///   - `bid_users`: users whose committed Bid PDA should be passed as a
///     `(bid, participant, kyc)` triple (irrespective of revealed state).
///   - `lottery_users`: users whose `(participant, kyc)` pair should be
///     passed for the lottery branch.
///
/// Returns the Instruction so the caller can choose to send (and whether
/// to expect failure).
fn select_winner_ix(
    env: &TestEnv,
    caller: &Keypair,
    pool: Pubkey,
    month: u8,
    bid_users: &[Pubkey],
    lottery_users: &[Pubkey],
) -> Instruction {
    let (config_pda, _) = env.protocol_config_pda();
    let (bid_stake_vault, _) = env.bid_stake_vault_pda(&pool);
    let (reserve_fund, _) = env.reserve_fund_pda(Tier::Vault);
    let (reserve_usdc_vault, _) = env.reserve_vault_pda(Tier::Vault);

    let mut metas = metas_select_winner(
        caller.pubkey(),
        config_pda,
        pool,
        bid_stake_vault,
        env.core_invoker,
        reserve_fund,
        reserve_usdc_vault,
    );

    // Append remaining_accounts: triples first, then pairs.
    for u in bid_users {
        let (bid_pda, _) = env.bid_pda(&pool, month, u);
        let (part_pda, _) = env.participant_pda(&pool, u);
        let (kyc_pda, _) = env.kyc_pda(u);
        metas.push(AccountMeta::new(bid_pda, false));
        metas.push(AccountMeta::new_readonly(part_pda, false));
        metas.push(AccountMeta::new_readonly(kyc_pda, false));
    }
    for u in lottery_users {
        let (part_pda, _) = env.participant_pda(&pool, u);
        let (kyc_pda, _) = env.kyc_pda(u);
        metas.push(AccountMeta::new_readonly(part_pda, false));
        metas.push(AccountMeta::new_readonly(kyc_pda, false));
    }

    build_ix(metas, poolver_core::instruction::SelectWinner {}.data())
}

fn send_select_winner(
    env: &mut TestEnv,
    caller: &Keypair,
    pool: Pubkey,
    month: u8,
    bid_users: &[Pubkey],
    lottery_users: &[Pubkey],
) -> Result<(), String> {
    env.svm.expire_blockhash();
    let ix = select_winner_ix(env, caller, pool, month, bid_users, lottery_users);
    send_ix(&mut env.svm, caller, ix)
}

// ───── Convenience: commit + reveal in one shot ─────────────────────────

fn commit_and_reveal(
    env: &mut TestEnv,
    user: &Keypair,
    user_usdc: Pubkey,
    pool: Pubkey,
    month: u8,
    bid_amount: u64,
    nonce: [u8; 16],
) {
    let p = env.fetch_pool(&pool);
    set_clock_to(env, p.current_month_started_at + 1);
    let hash = make_commit_hash(bid_amount, &nonce, &user.pubkey());
    commit_bid_for(env, user, user_usdc, pool, month, hash).expect("commit");
    set_clock_to(env, p.bid_window_ends_at + 1);
    reveal_bid_for(env, user, user_usdc, pool, month, bid_amount, nonce).expect("reveal");
}

// ─────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn t80_select_winner_single_revealed_bid() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 80, contribution, 12);

    // user[0] commits + reveals; others stay silent.
    let (u0, ata0) = (users[0].0.insecure_clone(), users[0].1);
    let bid_amount = 500 * ONE_USDC;
    commit_and_reveal(&mut env, &u0, ata0, pool, 1, bid_amount, [1u8; 16]);

    // Move past reveal window.
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.reveal_window_ends_at + 1);

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();
    send_select_winner(&mut env, &caller, pool, 1, &[u0.pubkey()], &[])
        .expect("select_winner");

    let p = env.fetch_pool(&pool);
    let mw = p.winners[0];
    assert_eq!(mw.month, 1, "month written");
    assert_eq!(mw.winner, u0.pubkey(), "user[0] wins");
    assert_eq!(mw.winning_bid, bid_amount);
    // monthly_pot = 12 * (1000 - 15 - 15) = 11_640 USDC
    let expected_pot = 12u64 * (contribution - 30_000_000);
    assert_eq!(mw.gross_payout, expected_pot);
    assert_eq!(mw.net_payout, expected_pot - bid_amount);
    assert!(matches!(
        mw.selection_method,
        poolver_core::state::SelectionMethod::Bid
    ));
    assert!(!mw.claimed);
    assert!(mw.selected_at > 0);

    // Bid.is_winner flipped.
    let (bid_pda, _) = env.bid_pda(&pool, 1, &u0.pubkey());
    let bid = env.fetch_bid(&bid_pda);
    assert!(bid.is_winner, "winning bid is_winner=true");

    // Participant.has_won NOT flipped here (claim_winning's job).
    let (part_pda, _) = env.participant_pda(&pool, &u0.pubkey());
    let part = env.fetch_participant(&part_pda);
    assert!(!part.has_won, "has_won deferred to claim_winning");
}

#[test]
fn t81_select_winner_highest_of_many_wins() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 81, contribution, 12);

    // 4 distinct revealed bids: 100, 250, 750 (winner), 500 USDC.
    let amounts = [
        100u64 * ONE_USDC,
        250 * ONE_USDC,
        750 * ONE_USDC, // winner
        500 * ONE_USDC,
    ];
    let mut bid_users = Vec::new();
    for (i, amt) in amounts.iter().enumerate() {
        let (u, ata) = (users[i].0.insecure_clone(), users[i].1);
        let nonce = {
            let mut n = [0u8; 16];
            n[0] = (i + 1) as u8;
            n
        };
        commit_and_reveal(&mut env, &u, ata, pool, 1, *amt, nonce);
        bid_users.push(u.pubkey());
    }

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.reveal_window_ends_at + 1);

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();
    send_select_winner(&mut env, &caller, pool, 1, &bid_users, &[]).expect("select");

    let p = env.fetch_pool(&pool);
    assert_eq!(
        p.winners[0].winner,
        users[2].0.pubkey(),
        "user[2] (750 USDC) wins"
    );
    assert_eq!(p.winners[0].winning_bid, 750 * ONE_USDC);
}

#[test]
fn t82_select_winner_tiebreak_deterministic_hash() {
    // Force a tie by having two users commit IDENTICAL revealed_amount.
    // Q-2: lexicographically smallest sha256(pool || month || user) wins.
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 82, contribution, 12);

    let amt = 500 * ONE_USDC;
    let (u_a, ata_a) = (users[0].0.insecure_clone(), users[0].1);
    let (u_b, ata_b) = (users[1].0.insecure_clone(), users[1].1);
    commit_and_reveal(&mut env, &u_a, ata_a, pool, 1, amt, [11u8; 16]);
    commit_and_reveal(&mut env, &u_b, ata_b, pool, 1, amt, [22u8; 16]);

    // Compute tiebreak hashes locally to figure out who SHOULD win.
    let pool_bytes = pool.to_bytes();
    let month_le = [1u8];
    let h_a = hashv(&[&pool_bytes, &month_le, &u_a.pubkey().to_bytes()]).to_bytes();
    let h_b = hashv(&[&pool_bytes, &month_le, &u_b.pubkey().to_bytes()]).to_bytes();
    let expected_winner = if h_a < h_b { u_a.pubkey() } else { u_b.pubkey() };

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.reveal_window_ends_at + 1);

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();
    send_select_winner(
        &mut env,
        &caller,
        pool,
        1,
        &[u_a.pubkey(), u_b.pubkey()],
        &[],
    )
    .expect("select");

    let p = env.fetch_pool(&pool);
    assert_eq!(
        p.winners[0].winner, expected_winner,
        "tie broken by smallest sha256(pool || month || user)"
    );
    assert_eq!(p.winners[0].winning_bid, amt);
}

#[test]
fn t83_select_winner_lottery_when_no_bids() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 83, contribution, 12);

    // No commits; jump straight past reveal window.
    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.reveal_window_ends_at + 1);
    advance_slot(&mut env); // make the slot non-zero for VRF seed mixing

    let lottery_users: Vec<Pubkey> = users.iter().map(|(u, _)| u.pubkey()).collect();
    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();
    send_select_winner(&mut env, &caller, pool, 1, &[], &lottery_users)
        .expect("lottery select");

    let p = env.fetch_pool(&pool);
    let mw = p.winners[0];
    assert_eq!(mw.month, 1);
    assert!(matches!(
        mw.selection_method,
        poolver_core::state::SelectionMethod::Lottery
    ));
    assert_eq!(mw.winning_bid, 0, "lottery → winning_bid = 0");
    let expected_pot = 12u64 * (contribution - 30_000_000);
    assert_eq!(mw.gross_payout, expected_pot);
    assert_eq!(mw.net_payout, expected_pot, "lottery → net == gross");
    // Winner must be one of the 12 candidates.
    assert!(
        users.iter().any(|(u, _)| u.pubkey() == mw.winner),
        "lottery winner must be in candidate set"
    );
}

#[test]
fn t84_select_winner_rejected_during_reveal_window() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 84, contribution, 12);
    let (u0, ata0) = (users[0].0.insecure_clone(), users[0].1);
    commit_and_reveal(&mut env, &u0, ata0, pool, 1, 500 * ONE_USDC, [3u8; 16]);

    // Stay INSIDE reveal window (clock currently at bid_window_ends_at + 1).
    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();
    let res = send_select_winner(&mut env, &caller, pool, 1, &[u0.pubkey()], &[]);
    assert!(res.is_err(), "must reject before reveal window closes");
}

#[test]
fn t85_select_winner_rejected_when_already_selected() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 85, contribution, 12);
    let (u0, ata0) = (users[0].0.insecure_clone(), users[0].1);
    commit_and_reveal(&mut env, &u0, ata0, pool, 1, 500 * ONE_USDC, [4u8; 16]);

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.reveal_window_ends_at + 1);

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();
    send_select_winner(&mut env, &caller, pool, 1, &[u0.pubkey()], &[])
        .expect("first select");

    // Second call must fail with WinnerAlreadySelected.
    let res = send_select_winner(&mut env, &caller, pool, 1, &[u0.pubkey()], &[]);
    assert!(res.is_err(), "second select_winner must reject");
}

#[test]
fn t86_select_winner_rejected_when_paused() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 86, contribution, 12);
    let (u0, ata0) = (users[0].0.insecure_clone(), users[0].1);
    commit_and_reveal(&mut env, &u0, ata0, pool, 1, 500 * ONE_USDC, [5u8; 16]);

    // Forge `protocol_config.paused = true` directly.
    let (cfg_pda, _) = env.protocol_config_pda();
    let mut acct = env.svm.get_account(&cfg_pda).unwrap().clone();
    let mut cfg = ProtocolConfig::try_deserialize(&mut acct.data.as_ref()).unwrap();
    cfg.paused = true;
    let mut buf = Vec::new();
    cfg.try_serialize(&mut buf).unwrap();
    acct.data[..buf.len()].copy_from_slice(&buf);
    env.svm.set_account(cfg_pda, acct).unwrap();

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.reveal_window_ends_at + 1);

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();
    let res = send_select_winner(&mut env, &caller, pool, 1, &[u0.pubkey()], &[]);
    assert!(res.is_err(), "must reject while paused");
}

#[test]
fn t87_stake_forfeit_for_unrevealed_bid() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 87, contribution, 12);

    // user[0] commits but DOES NOT reveal (stake gets forfeit).
    // user[1] commits + reveals (becomes the winner).
    let (u0, ata0) = (users[0].0.insecure_clone(), users[0].1);
    let (u1, ata1) = (users[1].0.insecure_clone(), users[1].1);

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.current_month_started_at + 1);

    let nonce0 = [6u8; 16];
    let h0 = make_commit_hash(100 * ONE_USDC, &nonce0, &u0.pubkey());
    commit_bid_for(&mut env, &u0, ata0, pool, 1, h0).expect("commit u0");

    let nonce1 = [7u8; 16];
    let h1 = make_commit_hash(300 * ONE_USDC, &nonce1, &u1.pubkey());
    commit_bid_for(&mut env, &u1, ata1, pool, 1, h1).expect("commit u1");

    // Move into reveal window; reveal only u1.
    set_clock_to(&mut env, p.bid_window_ends_at + 1);
    reveal_bid_for(&mut env, &u1, ata1, pool, 1, 300 * ONE_USDC, nonce1)
        .expect("reveal u1");

    // Past reveal window.
    set_clock_to(&mut env, p.reveal_window_ends_at + 1);

    let stake_amount = contribution * 100 / 10_000; // 1% = 10 USDC
    let (stake_vault, _) = env.bid_stake_vault_pda(&pool);
    let (reserve_vault, _) = env.reserve_vault_pda(Tier::Vault);
    let (reserve_fund, _) = env.reserve_fund_pda(Tier::Vault);
    let stake_before = env.fetch_token_balance(&stake_vault);
    let reserve_before = env.fetch_token_balance(&reserve_vault);

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();
    send_select_winner(
        &mut env,
        &caller,
        pool,
        1,
        &[u0.pubkey(), u1.pubkey()],
        &[],
    )
    .expect("select");

    // Stake vault drained by exactly stake_amount; reserve grew by same.
    assert_eq!(
        env.fetch_token_balance(&stake_vault),
        stake_before - stake_amount,
        "stake vault drained"
    );
    assert_eq!(
        env.fetch_token_balance(&reserve_vault),
        reserve_before + stake_amount,
        "reserve vault credited"
    );

    // u0's bid.stake_refunded = true (idempotency flag).
    let (bid0, _) = env.bid_pda(&pool, 1, &u0.pubkey());
    let b0 = env.fetch_bid(&bid0);
    assert!(b0.stake_refunded, "forfeit flips stake_refunded");
    assert!(!b0.is_winner);

    // Reserve fund total_inflows incremented.
    let acct = env.svm.get_account(&reserve_fund).unwrap();
    let fund =
        poolver_reserve::state::ReserveFund::try_deserialize(&mut acct.data.as_ref())
            .unwrap();
    assert!(
        fund.total_inflows >= stake_amount,
        "reserve total_inflows reflects forfeit"
    );

    // u1 wins (the only revealed bid).
    let p = env.fetch_pool(&pool);
    assert_eq!(p.winners[0].winner, u1.pubkey());
}

#[test]
fn t88_light_kyc_bidder_filtered_out() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 88, contribution, 12);

    // Two bidders. user[0] (high bid) gets KYC downgraded to Light AFTER
    // commit+reveal — at select_winner time they're filtered out.
    let (u0, ata0) = (users[0].0.insecure_clone(), users[0].1);
    let (u1, ata1) = (users[1].0.insecure_clone(), users[1].1);
    commit_and_reveal(&mut env, &u0, ata0, pool, 1, 600 * ONE_USDC, [8u8; 16]);
    commit_and_reveal(&mut env, &u1, ata1, pool, 1, 400 * ONE_USDC, [9u8; 16]);

    // Downgrade u0's KYC.
    let (kyc_pda, _) = env.kyc_pda(&u0.pubkey());
    let mut acct = env.svm.get_account(&kyc_pda).unwrap().clone();
    let mut kyc = KycAttestation::try_deserialize(&mut acct.data.as_ref()).unwrap();
    kyc.level = KycLevel::Light.as_u8();
    let mut buf = Vec::new();
    kyc.try_serialize(&mut buf).unwrap();
    acct.data[..buf.len()].copy_from_slice(&buf);
    env.svm.set_account(kyc_pda, acct).unwrap();

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.reveal_window_ends_at + 1);

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();
    send_select_winner(
        &mut env,
        &caller,
        pool,
        1,
        &[u0.pubkey(), u1.pubkey()],
        &[],
    )
    .expect("select");

    // Lower-but-eligible user[1] wins; high-but-ineligible user[0] skipped.
    let p = env.fetch_pool(&pool);
    assert_eq!(p.winners[0].winner, u1.pubkey());
    assert_eq!(p.winners[0].winning_bid, 400 * ONE_USDC);
}

#[test]
fn t89_previous_winner_filtered_out() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 89, contribution, 12);

    let (u0, ata0) = (users[0].0.insecure_clone(), users[0].1);
    let (u1, ata1) = (users[1].0.insecure_clone(), users[1].1);
    commit_and_reveal(&mut env, &u0, ata0, pool, 1, 600 * ONE_USDC, [10u8; 16]);
    commit_and_reveal(&mut env, &u1, ata1, pool, 1, 400 * ONE_USDC, [11u8; 16]);

    // Force-mark u0 as previous winner.
    let (part_pda, _) = env.participant_pda(&pool, &u0.pubkey());
    let mut acct = env.svm.get_account(&part_pda).unwrap().clone();
    let mut part = Participant::try_deserialize(&mut acct.data.as_ref()).unwrap();
    part.has_won = true;
    part.win_month = 0; // doesn't matter; just need has_won true
    let mut buf = Vec::new();
    part.try_serialize(&mut buf).unwrap();
    acct.data[..buf.len()].copy_from_slice(&buf);
    env.svm.set_account(part_pda, acct).unwrap();

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.reveal_window_ends_at + 1);

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();
    send_select_winner(
        &mut env,
        &caller,
        pool,
        1,
        &[u0.pubkey(), u1.pubkey()],
        &[],
    )
    .expect("select");

    let p = env.fetch_pool(&pool);
    assert_eq!(p.winners[0].winner, u1.pubkey());
}

#[test]
fn t90_defaulted_participant_filtered_out() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 90, contribution, 12);

    let (u0, ata0) = (users[0].0.insecure_clone(), users[0].1);
    let (u1, ata1) = (users[1].0.insecure_clone(), users[1].1);
    commit_and_reveal(&mut env, &u0, ata0, pool, 1, 700 * ONE_USDC, [12u8; 16]);
    commit_and_reveal(&mut env, &u1, ata1, pool, 1, 350 * ONE_USDC, [13u8; 16]);

    // Mark u0 defaulted.
    let (part_pda, _) = env.participant_pda(&pool, &u0.pubkey());
    let mut acct = env.svm.get_account(&part_pda).unwrap().clone();
    let mut part = Participant::try_deserialize(&mut acct.data.as_ref()).unwrap();
    part.is_defaulted = true;
    let mut buf = Vec::new();
    part.try_serialize(&mut buf).unwrap();
    acct.data[..buf.len()].copy_from_slice(&buf);
    env.svm.set_account(part_pda, acct).unwrap();

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.reveal_window_ends_at + 1);

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();
    send_select_winner(
        &mut env,
        &caller,
        pool,
        1,
        &[u0.pubkey(), u1.pubkey()],
        &[],
    )
    .expect("select");

    let p = env.fetch_pool(&pool);
    assert_eq!(p.winners[0].winner, u1.pubkey());
}

#[test]
fn t91_lottery_no_eligible_participants() {
    // All 12 participants forced ineligible (defaulted). No bids exist.
    // Lottery branch must surface NoEligibleParticipants.
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 91, contribution, 12);

    for (u, _) in &users {
        let (part_pda, _) = env.participant_pda(&pool, &u.pubkey());
        let mut acct = env.svm.get_account(&part_pda).unwrap().clone();
        let mut part = Participant::try_deserialize(&mut acct.data.as_ref()).unwrap();
        part.is_defaulted = true;
        let mut buf = Vec::new();
        part.try_serialize(&mut buf).unwrap();
        acct.data[..buf.len()].copy_from_slice(&buf);
        env.svm.set_account(part_pda, acct).unwrap();
    }

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.reveal_window_ends_at + 1);

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();
    let lottery_users: Vec<Pubkey> = users.iter().map(|(u, _)| u.pubkey()).collect();
    let res = send_select_winner(&mut env, &caller, pool, 1, &[], &lottery_users);
    assert!(res.is_err(), "must reject with NoEligibleParticipants");
}

#[test]
fn t92_e2e_six_bids_six_silent() {
    // 12 participants, 6 commit + reveal, 6 stay silent. select_winner
    // picks the highest revealed bid; no stake forfeit for the silent
    // 6 (they never committed).
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 92, contribution, 12);

    let amounts = [
        100u64 * ONE_USDC,
        200 * ONE_USDC,
        300 * ONE_USDC,
        400 * ONE_USDC,
        500 * ONE_USDC,
        650 * ONE_USDC, // winner
    ];
    let mut bid_users = Vec::new();
    for (i, amt) in amounts.iter().enumerate() {
        let (u, ata) = (users[i].0.insecure_clone(), users[i].1);
        let nonce = {
            let mut n = [0u8; 16];
            n[0] = (50 + i) as u8;
            n
        };
        commit_and_reveal(&mut env, &u, ata, pool, 1, *amt, nonce);
        bid_users.push(u.pubkey());
    }

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.reveal_window_ends_at + 1);

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();
    send_select_winner(&mut env, &caller, pool, 1, &bid_users, &[])
        .expect("select");

    let p = env.fetch_pool(&pool);
    let mw = p.winners[0];
    assert_eq!(mw.winner, users[5].0.pubkey(), "users[5] (650) wins");
    assert_eq!(mw.winning_bid, 650 * ONE_USDC);
    let expected_pot = 12u64 * (contribution - 30_000_000);
    assert_eq!(mw.gross_payout, expected_pot);
    assert_eq!(mw.net_payout, expected_pot - 650 * ONE_USDC);
    assert!(matches!(
        mw.selection_method,
        poolver_core::state::SelectionMethod::Bid
    ));
}

#[test]
fn t93_reserve_isolation_wrong_tier_rejected() {
    // Pool tier = Vault. Caller passes the DeFi reserve accounts.
    // Anchor's seed-derived equality check fails (Unauthorized).
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 93, contribution, 12);

    let (u0, ata0) = (users[0].0.insecure_clone(), users[0].1);
    commit_and_reveal(&mut env, &u0, ata0, pool, 1, 500 * ONE_USDC, [14u8; 16]);

    let p = env.fetch_pool(&pool);
    set_clock_to(&mut env, p.reveal_window_ends_at + 1);

    let (config_pda, _) = env.protocol_config_pda();
    let (bid_stake_vault, _) = env.bid_stake_vault_pda(&pool);
    // Wrong tier reserve:
    let (wrong_reserve_fund, _) = env.reserve_fund_pda(Tier::DeFi);
    let (wrong_reserve_vault, _) = env.reserve_vault_pda(Tier::DeFi);

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();
    let mut metas = metas_select_winner(
        caller.pubkey(),
        config_pda,
        pool,
        bid_stake_vault,
        env.core_invoker,
        wrong_reserve_fund,
        wrong_reserve_vault,
    );
    let (bid_pda, _) = env.bid_pda(&pool, 1, &u0.pubkey());
    let (part_pda, _) = env.participant_pda(&pool, &u0.pubkey());
    let (kyc_pda, _) = env.kyc_pda(&u0.pubkey());
    metas.push(AccountMeta::new(bid_pda, false));
    metas.push(AccountMeta::new_readonly(part_pda, false));
    metas.push(AccountMeta::new_readonly(kyc_pda, false));
    let ix = build_ix(metas, poolver_core::instruction::SelectWinner {}.data());
    env.svm.expire_blockhash();
    let res = send_ix(&mut env.svm, &caller, ix);
    assert!(res.is_err(), "wrong-tier reserve must be rejected");
}

// Optional smoke: verify Bid::DISCRIMINATOR != Participant::DISCRIMINATOR
// (a structural invariant the handler relies on for chunk dispatch).
#[test]
fn t94_bid_vs_participant_discriminator_distinct() {
    assert_ne!(
        Bid::DISCRIMINATOR,
        Participant::DISCRIMINATOR,
        "select_winner chunk dispatch relies on distinct discriminators"
    );
}
