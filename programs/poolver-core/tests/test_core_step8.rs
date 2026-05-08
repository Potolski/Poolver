//! Step-8 integration tests for `poolver-core`: `claim_winning` +
//! the bid-credit pro-rata wiring inside `contribute`.
//!
//! Coverage map (per task prompt §Tests):
//!   1.  claim_winning happy path Tier 0 (rep multiplier 100%)         → t100
//!   2.  rejected when caller is not the selected winner               → t101
//!   3.  rejected when winner has not been selected                    → t102
//!   4.  rejected on double-claim                                      → t103
//!   5.  rejected when winner USDC < total_collateral_required         → t104
//!   6.  rejected when KYC missing/expired/sanctions                   → t105
//!   7.  reputation multiplier 0 cycles → 100% baseline                → t106
//!   8.  reputation multiplier 1 cycle → 70% baseline                  → t107
//!   9.  reputation multiplier 2+ cycles → 50% baseline                → t108
//!   10. bid distribution math (5/20/75 split)                         → t109
//!   11. solvency assertion (INV-1)                                    → t110
//!   12. MonthWinner.claimed flips                                     → t111
//!   13. Participant fields populated correctly                        → t112
//!   14. collateral_release_per_month computed correctly               → t113
//!   15. Month-12 winner edge case (immediate refund — Q-34)           → t114
//!   16. Post-claim contribute draws bid_credit_balance pro-rata       → t115
//!   17. advance_month resets paid_count_for_current_month             → t116
//!   18. End-to-end: 12 participants, win in month 5, all subsequent
//!       contributions reduced by credit                               → t117

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

#[allow(clippy::too_many_arguments)]
fn metas_claim_winning(
    winner: Pubkey,
    protocol_config: Pubkey,
    pool: Pubkey,
    participant: Pubkey,
    user_reputation: Pubkey,
    user_kyc: Pubkey,
    winner_usdc: Pubkey,
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
        AccountMeta::new(winner, true),
        AccountMeta::new_readonly(protocol_config, false),
        AccountMeta::new(pool, false),
        AccountMeta::new(participant, false),
        AccountMeta::new(user_reputation, false),
        AccountMeta::new_readonly(user_kyc, false),
        AccountMeta::new(winner_usdc, false),
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

fn claim_winning_for(
    env: &mut TestEnv,
    winner: &Keypair,
    winner_usdc: Pubkey,
    pool: Pubkey,
) -> Result<(), String> {
    env.svm.expire_blockhash();
    let (config_pda, _) = env.protocol_config_pda();
    let (participant, _) = env.participant_pda(&pool, &winner.pubkey());
    let (user_rep, _) = env.reputation_pda(&winner.pubkey());
    let (user_kyc, _) = env.kyc_pda(&winner.pubkey());
    let (pool_usdc_vault, _) = env.pool_usdc_vault_pda(&pool);
    let (collat_vault, _) = env.collateral_vault_pda(&pool);
    let (fee_vault, _) = env.protocol_fee_vault_pda();
    let (reserve_fund, _) = env.reserve_fund_pda(Tier::Vault);
    let (reserve_usdc, _) = env.reserve_vault_pda(Tier::Vault);
    let (adapter_state, _) = env.vault_adapter_pda(&pool);
    let (adapter_usdc, _) = env.vault_adapter_usdc_pda(&pool);

    let metas = metas_claim_winning(
        winner.pubkey(),
        config_pda,
        pool,
        participant,
        user_rep,
        user_kyc,
        winner_usdc,
        pool_usdc_vault,
        collat_vault,
        fee_vault,
        env.core_invoker,
        reserve_fund,
        reserve_usdc,
        adapter_state,
        adapter_usdc,
    );
    // Test helper claims the current month — matches pre-retroactive-claim
    // behavior so existing tests don't need rewriting.
    let pool_acct = env.fetch_pool(&pool);
    let ix = build_ix(
        metas,
        poolver_core::instruction::ClaimWinning {
            claim_month: pool_acct.current_month,
        }
        .data(),
    );
    send_ix(&mut env.svm, winner, ix)
}

// ───── Time helpers ──────────────────────────────────────────────────────

fn set_clock_to(env: &mut TestEnv, ts: i64) {
    let mut clock = env.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = ts;
    env.svm.set_sysvar::<Clock>(&clock);
}

// ───── Hash helper ─────────────────────────────────────────────────────

fn make_commit_hash(bid_amount: u64, nonce: &[u8; 16], user: &Pubkey) -> [u8; 32] {
    let user_bytes = user.to_bytes();
    let amt = bid_amount.to_le_bytes();
    hashv(&[&amt, nonce, &user_bytes]).to_bytes()
}

// ───── Setup builder ────────────────────────────────────────────────────

fn pool_with_n_full_kyc(
    env: &mut TestEnv,
    pool_id: u64,
    contribution: u64,
    n: usize,
    month_duration: Option<i64>,
) -> (Pubkey, Vec<(Keypair, Pubkey)>) {
    assert!(n <= 12, "pool size capped at 12");
    let creator = bootstrap_with_creator(env);
    set_clock_to(env, 1_000_000);

    let pool = create_pool_for(env, &creator, pool_id, Tier::Vault, contribution, month_duration)
        .expect("create_pool");

    let mut users = Vec::with_capacity(n);
    for _ in 0..n {
        let (user, ata) = fully_set_up_user_full_kyc(env, 200_000 * ONE_USDC);
        join_pool_for(env, &user, ata, pool, Tier::Vault).expect("join");
        users.push((user, ata));
    }
    (pool, users)
}

/// Convenience: commit + reveal in a single helper.
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

/// Run a full month-N selection: skip into month N, optionally have a
/// designated user commit+reveal `bid_amount`, then call `select_winner`.
/// Returns the winner pubkey from the on-chain MonthWinner record.
fn drive_month_with_winning_bid(
    env: &mut TestEnv,
    pool: Pubkey,
    target_month: u8,
    winner_kp: &Keypair,
    winner_ata: Pubkey,
    bid_amount: u64,
    other_users: &[Pubkey],
    nonce: [u8; 16],
    caller: &Keypair,
) -> Pubkey {
    // Make the (winner) commit + reveal.
    commit_and_reveal(env, winner_kp, winner_ata, pool, target_month, bid_amount, nonce);

    // Past reveal window: select_winner.
    let p = env.fetch_pool(&pool);
    set_clock_to(env, p.reveal_window_ends_at + 1);

    let mut bid_users = vec![winner_kp.pubkey()];
    bid_users.extend_from_slice(other_users);
    send_select_winner(env, caller, pool, target_month, &bid_users, &[])
        .expect("select_winner");

    let p = env.fetch_pool(&pool);
    let m_idx = (target_month as usize) - 1;
    p.winners[m_idx].winner
}

// ─────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn t100_claim_winning_happy_path() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 100, contribution, 12, None);

    // Winner = user[0]. completed_cycles_at_join = 0 (default new
    // reputation), so reputation multiplier = 100% (10_000 bps).
    // Win in month 1 (the auto-started month) with bid_amount = 500 USDC.
    let (u0, ata0) = (users[0].0.insecure_clone(), users[0].1);
    let bid_amount = 500 * ONE_USDC;
    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();

    let winner_pub = drive_month_with_winning_bid(
        &mut env, pool, 1, &u0, ata0, bid_amount, &[], [1u8; 16], &caller,
    );
    assert_eq!(winner_pub, u0.pubkey());

    // Snapshots BEFORE claim.
    let (fee_vault, _) = env.protocol_fee_vault_pda();
    let (reserve_usdc, _) = env.reserve_vault_pda(Tier::Vault);
    let (collat_vault, _) = env.collateral_vault_pda(&pool);
    let (pool_vault, _) = env.pool_usdc_vault_pda(&pool);
    let (adapter_usdc, _) = env.vault_adapter_usdc_pda(&pool);

    let fee_before = env.fetch_token_balance(&fee_vault);
    let reserve_before = env.fetch_token_balance(&reserve_usdc);
    let collat_before = env.fetch_token_balance(&collat_vault);
    let adapter_before = env.fetch_token_balance(&adapter_usdc);
    let pool_before = env.fetch_token_balance(&pool_vault);
    let winner_before = env.fetch_token_balance(&ata0);

    claim_winning_for(&mut env, &u0, ata0, pool).expect("claim_winning");

    // Compute expected values.
    let monthly_pot = 12u64 * (contribution - 30_000_000); // 11_640 USDC
    let net_payout = monthly_pot - bid_amount;             // 11_140 USDC
    let baseline = 11u64 * contribution;                    // (12-1) × 1000
    let total_collateral = baseline + 2 * bid_amount;       // 100% rep × baseline + premium
    let protocol_share = bid_amount * 500 / 10_000;
    let reserve_share = bid_amount * 2_000 / 10_000;
    let participant_share = bid_amount - protocol_share - reserve_share;

    // Fee + reserve received the bid carve-out.
    assert_eq!(env.fetch_token_balance(&fee_vault), fee_before + protocol_share);
    assert_eq!(env.fetch_token_balance(&reserve_usdc), reserve_before + reserve_share);
    // Collateral vault holds total_collateral.
    assert_eq!(env.fetch_token_balance(&collat_vault), collat_before + total_collateral);
    // Adapter drained by gross_payout (= net_payout + winning_bid).
    assert_eq!(env.fetch_token_balance(&adapter_usdc), adapter_before - monthly_pot);
    // Pool vault: +gross_payout (withdraw) - net_payout (to winner)
    //   - protocol_share - reserve_share = +participant_share retained.
    assert_eq!(
        env.fetch_token_balance(&pool_vault),
        pool_before + participant_share
    );
    // Winner balance: -collateral + net_payout.
    assert_eq!(
        env.fetch_token_balance(&ata0),
        winner_before + net_payout - total_collateral
    );

    // Pool state.
    let p = env.fetch_pool(&pool);
    assert!(p.winners[0].claimed, "MonthWinner.claimed flipped");
    assert_eq!(p.bid_credit_balance, participant_share, "75% credited");
    assert_eq!(p.total_distributed, net_payout);
    // Total collateral = 12 join collaterals (1× contribution each) +
    // the winner's post-win collateral.
    assert_eq!(
        p.total_collateral_locked,
        total_collateral + 12 * contribution
    );

    // Participant state.
    let (part_pda, _) = env.participant_pda(&pool, &u0.pubkey());
    let part = env.fetch_participant(&part_pda);
    assert!(part.has_won);
    assert_eq!(part.win_month, 1);
    assert_eq!(part.bid_amount_when_won, bid_amount);
    assert_eq!(part.collateral_initial, total_collateral);
    assert_eq!(part.collateral_locked, total_collateral);
    assert_eq!(part.collateral_release_per_month, total_collateral / 11);

    // Reputation state.
    let (rep_pda, _) = env.reputation_pda(&u0.pubkey());
    let rep = env.fetch_reputation(&rep_pda);
    assert_eq!(rep.total_received_lifetime, net_payout);
}

#[test]
fn t101_claim_rejected_when_not_winner() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 101, contribution, 12, None);

    let (u0, ata0) = (users[0].0.insecure_clone(), users[0].1);
    let (u1, ata1) = (users[1].0.insecure_clone(), users[1].1);
    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();
    drive_month_with_winning_bid(
        &mut env, pool, 1, &u0, ata0, 500 * ONE_USDC, &[], [2u8; 16], &caller,
    );

    // user[1] is NOT the winner; claim must reject.
    let res = claim_winning_for(&mut env, &u1, ata1, pool);
    assert!(res.is_err(), "non-winner claim must be rejected");
}

#[test]
fn t102_claim_rejected_when_not_yet_selected() {
    // No winner has been selected for the current month.
    // claim_winning must reject with NotWinner.
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 102, contribution, 12, None);

    let (u0, ata0) = (users[0].0.insecure_clone(), users[0].1);
    let res = claim_winning_for(&mut env, &u0, ata0, pool);
    assert!(res.is_err(), "claim before selection must reject");
}

#[test]
fn t103_claim_rejected_on_double_claim() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 103, contribution, 12, None);

    let (u0, ata0) = (users[0].0.insecure_clone(), users[0].1);
    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();
    drive_month_with_winning_bid(
        &mut env, pool, 1, &u0, ata0, 500 * ONE_USDC, &[], [3u8; 16], &caller,
    );

    claim_winning_for(&mut env, &u0, ata0, pool).expect("first claim");
    // Second call must fail with AlreadyClaimed (defense-in-depth:
    // also AlreadyWon catches this since participant.has_won=true now).
    let res = claim_winning_for(&mut env, &u0, ata0, pool);
    assert!(res.is_err(), "double claim must be rejected");
}

#[test]
fn t104_claim_rejected_when_collateral_insufficient() {
    // Underfund the winner so they can't post the required collateral.
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 104, contribution, 12, None);

    // Winner's ATA balance after `join_pool` = 200_000 - 1_000 = 199_000.
    // For win_month=1, total_collateral = 11×1000 + 2×bid_amount.
    // Drain the winner's balance below `total_collateral`.
    let (u0, ata0) = (users[0].0.insecure_clone(), users[0].1);
    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();
    let bid_amount = 500 * ONE_USDC;
    drive_month_with_winning_bid(
        &mut env, pool, 1, &u0, ata0, bid_amount, &[], [4u8; 16], &caller,
    );

    // Drain u0's ATA to 100 USDC.
    let acct = env.svm.get_account(&ata0).unwrap();
    let mut data = acct.data.clone();
    use solana_program_pack::Pack;
    use spl_token_interface::state::Account as SplTokenAccount;
    let mut ta = SplTokenAccount::unpack(&data).unwrap();
    ta.amount = 100 * ONE_USDC;
    SplTokenAccount::pack(ta, &mut data).unwrap();
    let mut new_acct = acct.clone();
    new_acct.data = data;
    env.svm.set_account(ata0, new_acct).unwrap();

    let res = claim_winning_for(&mut env, &u0, ata0, pool);
    assert!(res.is_err(), "underfunded winner must be rejected");
}

#[test]
fn t105_claim_rejected_when_kyc_expired() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 105, contribution, 12, None);

    let (u0, ata0) = (users[0].0.insecure_clone(), users[0].1);
    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();
    drive_month_with_winning_bid(
        &mut env, pool, 1, &u0, ata0, 500 * ONE_USDC, &[], [5u8; 16], &caller,
    );

    // Force-expire u0's KYC.
    let (kyc_pda, _) = env.kyc_pda(&u0.pubkey());
    let mut acct = env.svm.get_account(&kyc_pda).unwrap().clone();
    let mut kyc = KycAttestation::try_deserialize(&mut acct.data.as_ref()).unwrap();
    kyc.expires_at = 1; // epoch 1; clock is at 1_000_000+
    let mut buf = Vec::new();
    kyc.try_serialize(&mut buf).unwrap();
    acct.data[..buf.len()].copy_from_slice(&buf);
    env.svm.set_account(kyc_pda, acct).unwrap();

    let res = claim_winning_for(&mut env, &u0, ata0, pool);
    assert!(res.is_err(), "expired KYC must be rejected at claim");
}

/// Helper to set `completed_cycles_at_join` on a participant after
/// `join_pool`. Used by t106/t107/t108 to test the reputation
/// multiplier table without driving multiple full pool cycles.
fn force_completed_cycles_at_join(env: &mut TestEnv, pool: &Pubkey, user: &Pubkey, cycles: u8) {
    let (part_pda, _) = env.participant_pda(pool, user);
    let mut acct = env.svm.get_account(&part_pda).unwrap().clone();
    let mut part = Participant::try_deserialize(&mut acct.data.as_ref()).unwrap();
    part.completed_cycles_at_join = cycles;
    let mut buf = Vec::new();
    part.try_serialize(&mut buf).unwrap();
    acct.data[..buf.len()].copy_from_slice(&buf);
    env.svm.set_account(part_pda, acct).unwrap();
}

#[test]
fn t106_reputation_multiplier_zero_cycles() {
    // 0 cycles → 100% baseline. Already covered by t100; assert the
    // exact multiplier formula explicitly.
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 106, contribution, 12, None);
    let (u0, ata0) = (users[0].0.insecure_clone(), users[0].1);
    force_completed_cycles_at_join(&mut env, &pool, &u0.pubkey(), 0);

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();
    let bid_amount = 500 * ONE_USDC;
    drive_month_with_winning_bid(
        &mut env, pool, 1, &u0, ata0, bid_amount, &[], [6u8; 16], &caller,
    );
    claim_winning_for(&mut env, &u0, ata0, pool).expect("claim");

    let (part_pda, _) = env.participant_pda(&pool, &u0.pubkey());
    let part = env.fetch_participant(&part_pda);
    let baseline = 11u64 * contribution;
    let expected = baseline + 2 * bid_amount;
    assert_eq!(part.collateral_initial, expected, "0 cycles → 100% baseline");
}

#[test]
fn t107_reputation_multiplier_one_cycle() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 107, contribution, 12, None);
    let (u0, ata0) = (users[0].0.insecure_clone(), users[0].1);
    force_completed_cycles_at_join(&mut env, &pool, &u0.pubkey(), 1);

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();
    let bid_amount = 500 * ONE_USDC;
    drive_month_with_winning_bid(
        &mut env, pool, 1, &u0, ata0, bid_amount, &[], [7u8; 16], &caller,
    );
    claim_winning_for(&mut env, &u0, ata0, pool).expect("claim");

    let (part_pda, _) = env.participant_pda(&pool, &u0.pubkey());
    let part = env.fetch_participant(&part_pda);
    let baseline = 11u64 * contribution;
    // 70% rep: 11_000 × 7000 / 10000 = 7700
    let adjusted = baseline * 7_000 / 10_000;
    let expected = adjusted + 2 * bid_amount;
    assert_eq!(part.collateral_initial, expected, "1 cycle → 70% baseline");
}

#[test]
fn t108_reputation_multiplier_two_or_more_cycles() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 108, contribution, 12, None);
    let (u0, ata0) = (users[0].0.insecure_clone(), users[0].1);
    force_completed_cycles_at_join(&mut env, &pool, &u0.pubkey(), 5);

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();
    let bid_amount = 500 * ONE_USDC;
    drive_month_with_winning_bid(
        &mut env, pool, 1, &u0, ata0, bid_amount, &[], [8u8; 16], &caller,
    );
    claim_winning_for(&mut env, &u0, ata0, pool).expect("claim");

    let (part_pda, _) = env.participant_pda(&pool, &u0.pubkey());
    let part = env.fetch_participant(&part_pda);
    let baseline = 11u64 * contribution;
    // 50% rep
    let adjusted = baseline * 5_000 / 10_000;
    let expected = adjusted + 2 * bid_amount;
    assert_eq!(part.collateral_initial, expected, "5 cycles → 50% baseline");
}

#[test]
fn t109_bid_distribution_math_5_20_75() {
    // winning_bid = 1000 USDC → protocol = 50, reserve = 200, credit = 750.
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 109, contribution, 12, None);

    let (u0, ata0) = (users[0].0.insecure_clone(), users[0].1);
    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();
    let bid_amount = 1_000 * ONE_USDC;

    // Snapshots BEFORE driving the month (so we capture starting balances
    // including any inflows from join_pool / select_winner phases).
    let (fee_vault, _) = env.protocol_fee_vault_pda();
    let (reserve_usdc, _) = env.reserve_vault_pda(Tier::Vault);
    let fee_before = env.fetch_token_balance(&fee_vault);
    let reserve_before = env.fetch_token_balance(&reserve_usdc);

    drive_month_with_winning_bid(
        &mut env, pool, 1, &u0, ata0, bid_amount, &[], [9u8; 16], &caller,
    );
    claim_winning_for(&mut env, &u0, ata0, pool).expect("claim");

    let expected_protocol = 50 * ONE_USDC;
    let expected_reserve = 200 * ONE_USDC;
    let expected_credit = 750 * ONE_USDC;

    assert_eq!(env.fetch_token_balance(&fee_vault), fee_before + expected_protocol);
    assert_eq!(env.fetch_token_balance(&reserve_usdc), reserve_before + expected_reserve);
    let p = env.fetch_pool(&pool);
    assert_eq!(p.bid_credit_balance, expected_credit);
    assert_eq!(
        expected_protocol + expected_reserve + expected_credit,
        bid_amount,
        "5%+20%+75% = 100%"
    );
}

#[test]
fn t110_inv1_solvency_after_claim() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 110, contribution, 12, None);

    let (u0, ata0) = (users[0].0.insecure_clone(), users[0].1);
    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();
    let bid_amount = 500 * ONE_USDC;
    drive_month_with_winning_bid(
        &mut env, pool, 1, &u0, ata0, bid_amount, &[], [10u8; 16], &caller,
    );

    // Snapshot ALL USDC custody endpoints + winner balance.
    let (fee_vault, _) = env.protocol_fee_vault_pda();
    let (reserve_usdc, _) = env.reserve_vault_pda(Tier::Vault);
    let (collat_vault, _) = env.collateral_vault_pda(&pool);
    let (pool_vault, _) = env.pool_usdc_vault_pda(&pool);
    let (adapter_usdc, _) = env.vault_adapter_usdc_pda(&pool);

    let total_before = env.fetch_token_balance(&fee_vault)
        + env.fetch_token_balance(&reserve_usdc)
        + env.fetch_token_balance(&collat_vault)
        + env.fetch_token_balance(&pool_vault)
        + env.fetch_token_balance(&adapter_usdc)
        + env.fetch_token_balance(&ata0);

    claim_winning_for(&mut env, &u0, ata0, pool).expect("claim");

    let total_after = env.fetch_token_balance(&fee_vault)
        + env.fetch_token_balance(&reserve_usdc)
        + env.fetch_token_balance(&collat_vault)
        + env.fetch_token_balance(&pool_vault)
        + env.fetch_token_balance(&adapter_usdc)
        + env.fetch_token_balance(&ata0);

    // INV-1 solvency: no USDC was created or destroyed; only moved
    // between custody endpoints. The 75% bid credit is virtual (no
    // token movement) so it doesn't appear in the sum.
    assert_eq!(
        total_before, total_after,
        "INV-1 solvency: total USDC across custody endpoints unchanged"
    );
}

#[test]
fn t111_month_winner_claimed_flips() {
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 111, contribution, 12, None);
    let (u0, ata0) = (users[0].0.insecure_clone(), users[0].1);
    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();
    drive_month_with_winning_bid(
        &mut env, pool, 1, &u0, ata0, 500 * ONE_USDC, &[], [11u8; 16], &caller,
    );

    let p = env.fetch_pool(&pool);
    assert!(!p.winners[0].claimed, "claimed=false before claim_winning");

    claim_winning_for(&mut env, &u0, ata0, pool).expect("claim");
    let p = env.fetch_pool(&pool);
    assert!(p.winners[0].claimed, "claimed=true after claim_winning");
}

#[test]
fn t112_participant_fields_populated() {
    // Aggregate happy-path field check.
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 112, contribution, 12, None);
    let (u0, ata0) = (users[0].0.insecure_clone(), users[0].1);
    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();
    let bid_amount = 750 * ONE_USDC;
    drive_month_with_winning_bid(
        &mut env, pool, 1, &u0, ata0, bid_amount, &[], [12u8; 16], &caller,
    );

    let (part_pda, _) = env.participant_pda(&pool, &u0.pubkey());
    let part_before = env.fetch_participant(&part_pda);
    assert!(!part_before.has_won, "has_won=false before claim");
    assert_eq!(part_before.win_month, 0);

    claim_winning_for(&mut env, &u0, ata0, pool).expect("claim");

    let part = env.fetch_participant(&part_pda);
    assert!(part.has_won);
    assert_eq!(part.win_month, 1);
    assert_eq!(part.bid_amount_when_won, bid_amount);
    assert_eq!(part.collateral_initial, 11 * contribution + 2 * bid_amount);
    assert_eq!(part.collateral_locked, part.collateral_initial);
}

#[test]
fn t113_collateral_release_per_month_cached() {
    // Win month 3 with bid 200; baseline = 9 × 1000; 0 cycles → 100%
    // multiplier. release_per_month = (9000 + 400) / 9 = 1044.444…
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    // Short month so we can advance fast.
    let (pool, users) = pool_with_n_full_kyc(&mut env, 113, contribution, 12, Some(60));

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();

    // Advance to month 3 by ticking through advance_month twice.
    for _ in 0..2 {
        let p = env.fetch_pool(&pool);
        set_clock_to(
            &mut env,
            p.current_month_started_at + p.month_duration_seconds + 1,
        );
        advance_month_for(&mut env, &caller, pool).expect("advance");
    }

    // u0 bids in month 3.
    let (u0, ata0) = (users[0].0.insecure_clone(), users[0].1);
    let bid_amount = 200 * ONE_USDC;
    drive_month_with_winning_bid(
        &mut env, pool, 3, &u0, ata0, bid_amount, &[], [13u8; 16], &caller,
    );
    claim_winning_for(&mut env, &u0, ata0, pool).expect("claim");

    let (part_pda, _) = env.participant_pda(&pool, &u0.pubkey());
    let part = env.fetch_participant(&part_pda);
    let total_collateral = 9 * contribution + 2 * bid_amount;
    let expected_release = total_collateral / 9;
    assert_eq!(part.collateral_release_per_month, expected_release);
}

#[test]
fn t114_month12_winner_immediate_refund() {
    // SPEC_QUESTION-34: month-12 winner has baseline=0 → only bid_premium
    // collateral, immediately refunded since there are no future
    // contributions.
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 114, contribution, 12, Some(60));

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();

    // Advance to month 12 (11 ticks).
    for _ in 0..11 {
        let p = env.fetch_pool(&pool);
        set_clock_to(
            &mut env,
            p.current_month_started_at + p.month_duration_seconds + 1,
        );
        advance_month_for(&mut env, &caller, pool).expect("advance");
    }

    let p = env.fetch_pool(&pool);
    assert_eq!(p.current_month, 12);

    let (u0, ata0) = (users[0].0.insecure_clone(), users[0].1);
    let bid_amount = 100 * ONE_USDC;
    drive_month_with_winning_bid(
        &mut env, pool, 12, &u0, ata0, bid_amount, &[], [14u8; 16], &caller,
    );

    let winner_before = env.fetch_token_balance(&ata0);
    let (collat_vault, _) = env.collateral_vault_pda(&pool);
    let collat_before = env.fetch_token_balance(&collat_vault);

    claim_winning_for(&mut env, &u0, ata0, pool).expect("claim month 12");

    let monthly_pot = 12u64 * (contribution - 30_000_000);
    let net_payout = monthly_pot - bid_amount;
    let bid_premium = 2 * bid_amount;

    // Winner received net_payout AND immediately refunded the
    // bid_premium they posted. Net change to winner ATA = +net_payout.
    let winner_after = env.fetch_token_balance(&ata0);
    assert_eq!(winner_after, winner_before + net_payout);

    // Collateral vault: +bid_premium (post) − bid_premium (refund) = 0 net.
    let collat_after = env.fetch_token_balance(&collat_vault);
    assert_eq!(collat_after, collat_before);

    // Participant.collateral_locked = 0 (refunded), but
    // collateral_initial = bid_premium for indexer history.
    let (part_pda, _) = env.participant_pda(&pool, &u0.pubkey());
    let part = env.fetch_participant(&part_pda);
    assert_eq!(part.collateral_initial, bid_premium);
    assert_eq!(part.collateral_locked, 0, "month-12: refunded immediately");
}

#[test]
fn t115_post_claim_contribute_uses_bid_credit() {
    // After u0 wins month 1 with bid 1200, in month 2 the 12 participants
    // each contribute. Each draws an equal share from bid_credit_balance.
    // Share = balance / unpaid_count.
    //
    //   month 1 win: bid_credit_balance starts at 1200 × 75% = 900
    //   month 2 contribute call 1: divisor = 12 (no one paid yet)
    //     → share = 900/12 = 75
    //   month 2 contribute call 2: divisor = 11; balance = 900 - 75 = 825
    //     → share = 825/11 = 75
    //   ... each subsequent call also draws 75 (mathematically clean).
    //
    // Total drawn over the month: 12 × 75 = 900. Balance ends at 0.
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 115, contribution, 12, Some(60));

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();

    let (u0, ata0) = (users[0].0.insecure_clone(), users[0].1);
    let bid_amount = 1_200 * ONE_USDC;
    drive_month_with_winning_bid(
        &mut env, pool, 1, &u0, ata0, bid_amount, &[], [15u8; 16], &caller,
    );
    claim_winning_for(&mut env, &u0, ata0, pool).expect("claim");

    let p = env.fetch_pool(&pool);
    let credit_total = bid_amount * 7_500 / 10_000; // 900 USDC
    assert_eq!(p.bid_credit_balance, credit_total);

    // Advance to month 2.
    let p = env.fetch_pool(&pool);
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 1,
    );
    advance_month_for(&mut env, &caller, pool).expect("advance");
    let p = env.fetch_pool(&pool);
    assert_eq!(p.current_month, 2);
    assert_eq!(
        p.paid_count_for_current_month, 0,
        "advance_month resets paid counter"
    );
    set_clock_to(&mut env, p.current_month_started_at + 10);

    // First contribute: pick a NON-winner (users[1]) so we can assert
    // the actual_paid math without the winner's collateral-release
    // release path muddying the balance change. (Winners DO contribute
    // — winning_bid_when_won unlocks them — but their balance change
    // also includes the per-month `collateral_release_per_month`
    // refund from the collateral_vault, which is tested separately.)
    let credit_share = credit_total / 12; // 75 USDC
    let actual_paid = contribution - credit_share;
    let (u1, ata1) = (users[1].0.insecure_clone(), users[1].1);
    let user1_balance_before = env.fetch_token_balance(&ata1);
    contribute_for(&mut env, &u1, ata1, pool).expect("contribute u1");
    let user1_balance_after = env.fetch_token_balance(&ata1);
    assert_eq!(
        user1_balance_before - user1_balance_after,
        actual_paid,
        "non-winner u1 paid contribution - credit_share in month 2"
    );

    // Now have the remaining 11 (winner u0 + users[2..]) all contribute.
    contribute_for(&mut env, &u0, ata0, pool).expect("contribute u0 (winner)");
    for i in 2..12 {
        let (u, ata) = (users[i].0.insecure_clone(), users[i].1);
        contribute_for(&mut env, &u, ata, pool).expect("contribute");
    }

    let p = env.fetch_pool(&pool);
    assert_eq!(p.paid_count_for_current_month, 12);
    assert_eq!(
        p.bid_credit_balance, 0,
        "fully drained after 12 contributions"
    );
}

#[test]
fn t116_advance_month_resets_paid_counter() {
    // Functional check that advance_month zeroes paid_count_for_current_month
    // even when bid_credit_balance is unchanged.
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 116, contribution, 12, Some(60));

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();

    let (u0, ata0) = (users[0].0.insecure_clone(), users[0].1);
    drive_month_with_winning_bid(
        &mut env, pool, 1, &u0, ata0, 500 * ONE_USDC, &[], [16u8; 16], &caller,
    );
    claim_winning_for(&mut env, &u0, ata0, pool).expect("claim");

    let credit_before = env.fetch_pool(&pool).bid_credit_balance;
    assert!(credit_before > 0);

    // Advance to month 2.
    let p = env.fetch_pool(&pool);
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 1,
    );
    advance_month_for(&mut env, &caller, pool).expect("advance");

    let p = env.fetch_pool(&pool);
    assert_eq!(p.current_month, 2);
    assert_eq!(p.paid_count_for_current_month, 0, "counter resets");
    assert_eq!(
        p.bid_credit_balance, credit_before,
        "bid_credit_balance carries forward across months"
    );
}

#[test]
fn t117_e2e_win_month5_subsequent_contributions_reduced() {
    // 12 participants. u0 wins month 5 with bid 240.
    // bid_credit_balance after claim = 180 USDC.
    // In months 6..=12 (7 months), each of 12 participants contributes.
    // Total credit available: 180. After 12 contributions in month 6,
    // credit drains to 0. Subsequent months see no credit.
    let mut env = TestEnv::new();
    let contribution = 1_000 * ONE_USDC;
    let (pool, users) = pool_with_n_full_kyc(&mut env, 117, contribution, 12, Some(60));

    let caller = Keypair::new();
    env.svm.airdrop(&caller.pubkey(), 10 * SOL).unwrap();

    // Months 2..=4: simple advance (no contributes — keeps test fast).
    for _ in 0..4 {
        let p = env.fetch_pool(&pool);
        set_clock_to(
            &mut env,
            p.current_month_started_at + p.month_duration_seconds + 1,
        );
        advance_month_for(&mut env, &caller, pool).expect("advance");
    }
    let p = env.fetch_pool(&pool);
    assert_eq!(p.current_month, 5);

    // u0 wins month 5.
    let (u0, ata0) = (users[0].0.insecure_clone(), users[0].1);
    let bid_amount = 240 * ONE_USDC;
    drive_month_with_winning_bid(
        &mut env, pool, 5, &u0, ata0, bid_amount, &[], [17u8; 16], &caller,
    );
    claim_winning_for(&mut env, &u0, ata0, pool).expect("claim");

    let p = env.fetch_pool(&pool);
    let credit_total = bid_amount * 7_500 / 10_000; // 180
    assert_eq!(p.bid_credit_balance, credit_total);

    // Advance to month 6 and have all 12 contribute.
    let p = env.fetch_pool(&pool);
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 1,
    );
    advance_month_for(&mut env, &caller, pool).expect("advance");
    let p = env.fetch_pool(&pool);
    assert_eq!(p.current_month, 6);
    set_clock_to(&mut env, p.current_month_started_at + 10);

    for (u, ata) in &users {
        contribute_for(&mut env, u, *ata, pool).expect("month 6 contribute");
    }
    let p = env.fetch_pool(&pool);
    // 180 / 12 = 15 per share. 12 calls × 15 = 180 fully drained.
    assert_eq!(p.bid_credit_balance, 0, "credit fully drained in month 6");

    // Month 7: no credit; full contribution required.
    let p = env.fetch_pool(&pool);
    set_clock_to(
        &mut env,
        p.current_month_started_at + p.month_duration_seconds + 1,
    );
    advance_month_for(&mut env, &caller, pool).expect("advance");
    let p = env.fetch_pool(&pool);
    assert_eq!(p.current_month, 7);
    set_clock_to(&mut env, p.current_month_started_at + 10);

    let (u_any, ata_any) = (users[1].0.insecure_clone(), users[1].1);
    let before = env.fetch_token_balance(&ata_any);
    contribute_for(&mut env, &u_any, ata_any, pool).expect("month 7 contribute");
    let after = env.fetch_token_balance(&ata_any);
    assert_eq!(
        before - after,
        contribution,
        "month 7 has no credit; full contribution charged"
    );
}
