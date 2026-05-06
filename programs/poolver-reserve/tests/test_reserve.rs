//! Unit tests for `poolver-reserve`.
//!
//! Coverage map (per task prompt §F):
//!   1.  initialize_reserve happy path Tier 0                    → t01
//!   2.  initialize_reserve happy path Tier 1                    → t02
//!   3.  initialize_reserve cannot be called twice               → t03
//!   4.  deposit happy path                                      → t04
//!   5.  deposit rejects non-canonical core_invoker              → t05
//!   6.  deposit rejects amount=0                                → t06
//!   7.  draw happy path                                         → t07
//!   8.  draw rejects amount > balance (CRITICAL — INV-2)        → t08
//!   9.  draw rejects non-canonical core_invoker                 → t09
//!   10. seed happy path                                         → t10
//!   11. reserve isolation (Tier 0 vs Tier 1) — INV-4            → t11
//!
//! Every test that ends with a successful state mutation also asserts the
//! INV-3 identity `total_balance == total_inflows − total_outflows`.

mod common;

use anchor_lang::InstructionData;
use common::*;
use solana_instruction::AccountMeta;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

// ───── INV-3 helper ──────────────────────────────────────────────────────
// Every state-mutating call site invokes this so a logic bug that breaks
// the inflow/outflow identity is caught at the test layer in addition to
// the CPI-layer constraints.
fn assert_inv3(fund: &ReserveFund) {
    assert_eq!(
        fund.total_balance,
        fund.total_inflows - fund.total_outflows,
        "INV-3 violated: balance != inflows - outflows  (b={} in={} out={})",
        fund.total_balance,
        fund.total_inflows,
        fund.total_outflows,
    );
}

// ───── Account-meta builders (Anchor-order-sensitive) ────────────────────

fn metas_initialize_reserve(
    env: &TestEnv,
    admin: Pubkey,
    reserve_fund: Pubkey,
    reserve_usdc_vault: Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new(admin, true),
        AccountMeta::new(reserve_fund, false),
        AccountMeta::new_readonly(env.usdc_mint, false),
        AccountMeta::new(reserve_usdc_vault, false),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
        AccountMeta::new_readonly(solana_pubkey::pubkey!("11111111111111111111111111111111"), false),
        AccountMeta::new_readonly(
            solana_pubkey::pubkey!("SysvarRent111111111111111111111111111111111"),
            false,
        ),
    ]
}

fn metas_deposit(
    core_invoker: Pubkey,
    reserve_fund: Pubkey,
    reserve_usdc_vault: Pubkey,
    source_usdc: Pubkey,
    source_authority: Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new_readonly(core_invoker, false),
        AccountMeta::new(reserve_fund, false),
        AccountMeta::new(reserve_usdc_vault, false),
        AccountMeta::new(source_usdc, false),
        AccountMeta::new_readonly(source_authority, true),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
    ]
}

fn metas_draw(
    core_invoker: Pubkey,
    reserve_fund: Pubkey,
    reserve_usdc_vault: Pubkey,
    destination_usdc: Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new_readonly(core_invoker, false),
        AccountMeta::new(reserve_fund, false),
        AccountMeta::new(reserve_usdc_vault, false),
        AccountMeta::new(destination_usdc, false),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
    ]
}

fn metas_seed(
    funder: Pubkey,
    reserve_fund: Pubkey,
    reserve_usdc_vault: Pubkey,
    source_usdc: Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new_readonly(funder, true),
        AccountMeta::new(reserve_fund, false),
        AccountMeta::new(reserve_usdc_vault, false),
        AccountMeta::new(source_usdc, false),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
    ]
}

// ───── Helpers ───────────────────────────────────────────────────────────

/// Initialise a tier reserve. Returns `(reserve_fund_pda, reserve_vault_pda)`.
fn init_tier(env: &mut TestEnv, tier: Tier) -> (Pubkey, Pubkey) {
    let (reserve_fund, _) = env.reserve_fund_pda(tier);
    let (reserve_usdc_vault, _) = env.reserve_vault_pda(tier);

    let metas =
        metas_initialize_reserve(env, env.payer.pubkey(), reserve_fund, reserve_usdc_vault);
    let data = poolver_reserve::instruction::InitializeReserve { tier }.data();
    let ix = solana_instruction::Instruction {
        program_id: poolver_reserve::ID,
        accounts: metas,
        data,
    };
    let payer_kp = env.payer.insecure_clone();
    send_ix(&mut env.svm, &payer_kp, ix).expect("initialize_reserve failed");
    (reserve_fund, reserve_usdc_vault)
}

/// CPI deposit through fake-core. Caller must pre-fund `source_authority`'s
/// ATA with at least `amount` USDC.
fn cpi_deposit(
    env: &mut TestEnv,
    fund: Pubkey,
    vault: Pubkey,
    source_authority: &Keypair,
    source_ata: Pubkey,
    amount: u64,
) -> Result<(), String> {
    let metas = metas_deposit(
        env.core_invoker,
        fund,
        vault,
        source_ata,
        source_authority.pubkey(),
    );
    let data = poolver_reserve::instruction::Deposit { amount }.data();
    let ix = forward_through_fake_core(metas, data, env.core_invoker);
    let payer_kp = env.payer.insecure_clone();
    send_ix_signed(&mut env.svm, &payer_kp, &[source_authority], ix)
}

// ─────────────────────────────────────────────────────────────────────────
// Test 1: initialize_reserve happy path Tier 0
// ─────────────────────────────────────────────────────────────────────────
#[test]
fn t01_initialize_reserve_tier0_happy() {
    let mut env = TestEnv::new();
    let (fund_pda, vault_pda) = init_tier(&mut env, Tier::Vault);

    let fund = env.fetch_fund(&fund_pda);
    assert_eq!(fund.tier, Tier::Vault);
    assert_eq!(fund.total_balance, 0);
    assert_eq!(fund.total_inflows, 0);
    assert_eq!(fund.total_outflows, 0);
    assert_eq!(fund.usdc_vault, vault_pda);
    assert!(fund.bump != 0);
    assert_inv3(&fund);

    // Token account exists and is empty.
    assert_eq!(env.fetch_token_balance(&vault_pda), 0);

    // Fund PDA seed is genuinely tier-encoded.
    let (expected_pda, _) = solana_pubkey::Pubkey::find_program_address(
        &[RESERVE_FUND_SEED, &[Tier::Vault.as_u8()]],
        &poolver_reserve::ID,
    );
    assert_eq!(fund_pda, expected_pda);
}

// ─────────────────────────────────────────────────────────────────────────
// Test 2: initialize_reserve happy path Tier 1 — distinct PDA from Tier 0
// ─────────────────────────────────────────────────────────────────────────
#[test]
fn t02_initialize_reserve_tier1_happy() {
    let mut env = TestEnv::new();
    let (tier0_fund, tier0_vault) = init_tier(&mut env, Tier::Vault);
    let (tier1_fund, tier1_vault) = init_tier(&mut env, Tier::DeFi);

    assert_ne!(tier0_fund, tier1_fund, "tier-encoded seeds must differ");
    assert_ne!(tier0_vault, tier1_vault, "tier vault PDAs must differ");

    let fund = env.fetch_fund(&tier1_fund);
    assert_eq!(fund.tier, Tier::DeFi);
    assert_eq!(fund.total_balance, 0);
    assert_eq!(fund.usdc_vault, tier1_vault);
    assert_inv3(&fund);
}

// ─────────────────────────────────────────────────────────────────────────
// Test 3: initialize_reserve cannot be called twice for the same tier
// ─────────────────────────────────────────────────────────────────────────
#[test]
fn t03_initialize_reserve_double_init_rejected() {
    let mut env = TestEnv::new();
    let _ = init_tier(&mut env, Tier::Vault);

    // Second call with the same tier — Anchor's `init` constraint must
    // refuse because the account already exists. Re-init for the same tier
    // is structurally impossible because the seed is the tier byte.
    let (reserve_fund, _) = env.reserve_fund_pda(Tier::Vault);
    let (reserve_usdc_vault, _) = env.reserve_vault_pda(Tier::Vault);
    let metas =
        metas_initialize_reserve(&env, env.payer.pubkey(), reserve_fund, reserve_usdc_vault);
    let data = poolver_reserve::instruction::InitializeReserve { tier: Tier::Vault }.data();
    let ix = solana_instruction::Instruction {
        program_id: poolver_reserve::ID,
        accounts: metas,
        data,
    };
    let payer_kp = env.payer.insecure_clone();
    let result = send_ix(&mut env.svm, &payer_kp, ix);
    assert!(result.is_err(), "double-init must be rejected");
}

// ─────────────────────────────────────────────────────────────────────────
// Test 4: deposit happy path
// ─────────────────────────────────────────────────────────────────────────
#[test]
fn t04_deposit_happy() {
    let mut env = TestEnv::new();
    let (fund_pda, vault_pda) = init_tier(&mut env, Tier::Vault);

    // Source: an external "source authority" keypair with a token account.
    // In production this is the pool USDC vault PDA (signed by core).
    let source_authority = Keypair::new();
    env.svm.airdrop(&source_authority.pubkey(), SOL).unwrap();
    let source_ata = env.fund_token_account(&source_authority.pubkey(), 1_000 * ONE_USDC);

    let amount = 250 * ONE_USDC;
    cpi_deposit(
        &mut env,
        fund_pda,
        vault_pda,
        &source_authority,
        source_ata,
        amount,
    )
    .expect("deposit failed");

    let fund = env.fetch_fund(&fund_pda);
    assert_eq!(fund.total_balance, amount);
    assert_eq!(fund.total_inflows, amount);
    assert_eq!(fund.total_outflows, 0, "outflows untouched on deposit");
    assert_inv3(&fund);

    assert_eq!(env.fetch_token_balance(&vault_pda), amount);
    assert_eq!(env.fetch_token_balance(&source_ata), 1_000 * ONE_USDC - amount);
}

// ─────────────────────────────────────────────────────────────────────────
// Test 5: deposit rejects non-canonical core_invoker
// ─────────────────────────────────────────────────────────────────────────
#[test]
fn t05_deposit_rejects_wrong_core() {
    let mut env = TestEnv::new();
    let (fund_pda, vault_pda) = init_tier(&mut env, Tier::Vault);

    let source_authority = Keypair::new();
    env.svm.airdrop(&source_authority.pubkey(), SOL).unwrap();
    let source_ata = env.fund_token_account(&source_authority.pubkey(), 100 * ONE_USDC);

    let bogus_core = Pubkey::new_unique(); // any-old key, not the canonical PDA.
    // Bypass fake_core entirely — the
    // `seeds::program = POOLVER_CORE_ID, seeds = [b"core_invoker"]`
    // constraint must reject regardless of is_signer flag because the key
    // won't match the canonical PDA derivation.
    let metas = metas_deposit(
        bogus_core,
        fund_pda,
        vault_pda,
        source_ata,
        source_authority.pubkey(),
    );
    let data = poolver_reserve::instruction::Deposit { amount: 50 * ONE_USDC }.data();
    let ix = solana_instruction::Instruction {
        program_id: poolver_reserve::ID,
        accounts: metas,
        data,
    };
    let payer_kp = env.payer.insecure_clone();
    let result = send_ix_signed(&mut env.svm, &payer_kp, &[&source_authority], ix);
    assert!(result.is_err(), "wrong core_invoker must be rejected");

    // State unchanged.
    let fund = env.fetch_fund(&fund_pda);
    assert_eq!(fund.total_balance, 0);
    assert_inv3(&fund);
}

// ─────────────────────────────────────────────────────────────────────────
// Test 6: deposit rejects amount=0
// ─────────────────────────────────────────────────────────────────────────
#[test]
fn t06_deposit_rejects_zero_amount() {
    let mut env = TestEnv::new();
    let (fund_pda, vault_pda) = init_tier(&mut env, Tier::Vault);

    let source_authority = Keypair::new();
    env.svm.airdrop(&source_authority.pubkey(), SOL).unwrap();
    let source_ata = env.fund_token_account(&source_authority.pubkey(), 100 * ONE_USDC);

    let result = cpi_deposit(&mut env, fund_pda, vault_pda, &source_authority, source_ata, 0);
    assert!(result.is_err(), "amount=0 must be rejected");

    let fund = env.fetch_fund(&fund_pda);
    assert_eq!(fund.total_balance, 0);
    assert_inv3(&fund);
}

// ─────────────────────────────────────────────────────────────────────────
// Test 7: draw happy path
// ─────────────────────────────────────────────────────────────────────────
#[test]
fn t07_draw_happy() {
    let mut env = TestEnv::new();
    let (fund_pda, vault_pda) = init_tier(&mut env, Tier::Vault);

    // Seed via deposit: 500 USDC.
    let source_authority = Keypair::new();
    env.svm.airdrop(&source_authority.pubkey(), SOL).unwrap();
    let source_ata = env.fund_token_account(&source_authority.pubkey(), 500 * ONE_USDC);
    cpi_deposit(
        &mut env,
        fund_pda,
        vault_pda,
        &source_authority,
        source_ata,
        500 * ONE_USDC,
    )
    .unwrap();

    // Destination — any ATA we control.
    let recipient = Keypair::new();
    let dest_ata = env.fund_token_account(&recipient.pubkey(), 0);

    let draw_amount = 200 * ONE_USDC;
    let metas = metas_draw(env.core_invoker, fund_pda, vault_pda, dest_ata);
    let data = poolver_reserve::instruction::Draw { amount: draw_amount }.data();
    let ix = forward_through_fake_core(metas, data, env.core_invoker);
    let payer_kp = env.payer.insecure_clone();
    send_ix(&mut env.svm, &payer_kp, ix).expect("draw failed");

    let fund = env.fetch_fund(&fund_pda);
    assert_eq!(fund.total_balance, 500 * ONE_USDC - draw_amount);
    // Inflows untouched on draw.
    assert_eq!(fund.total_inflows, 500 * ONE_USDC);
    assert_eq!(fund.total_outflows, draw_amount);
    assert_inv3(&fund);

    assert_eq!(
        env.fetch_token_balance(&vault_pda),
        500 * ONE_USDC - draw_amount
    );
    assert_eq!(env.fetch_token_balance(&dest_ata), draw_amount);
}

// ─────────────────────────────────────────────────────────────────────────
// Test 8: draw rejects with ReserveInsufficient when amount > balance.
// THIS IS THE CRITICAL INV-2 TEST.
// ─────────────────────────────────────────────────────────────────────────
#[test]
fn t08_draw_rejects_insufficient_inv2() {
    let mut env = TestEnv::new();
    let (fund_pda, vault_pda) = init_tier(&mut env, Tier::Vault);

    // Seed the reserve with 100 USDC.
    let source_authority = Keypair::new();
    env.svm.airdrop(&source_authority.pubkey(), SOL).unwrap();
    let source_ata = env.fund_token_account(&source_authority.pubkey(), 100 * ONE_USDC);
    cpi_deposit(
        &mut env,
        fund_pda,
        vault_pda,
        &source_authority,
        source_ata,
        100 * ONE_USDC,
    )
    .unwrap();

    let recipient = Keypair::new();
    let dest_ata = env.fund_token_account(&recipient.pubkey(), 0);

    // Try to draw twice the balance — must error with ReserveInsufficient.
    let metas = metas_draw(env.core_invoker, fund_pda, vault_pda, dest_ata);
    let data = poolver_reserve::instruction::Draw {
        amount: 200 * ONE_USDC,
    }
    .data();
    let ix = forward_through_fake_core(metas, data, env.core_invoker);
    let payer_kp = env.payer.insecure_clone();
    let result = send_ix(&mut env.svm, &payer_kp, ix);
    assert!(result.is_err(), "draw exceeding balance must be rejected (INV-2)");

    // INV-2 holds: balance unchanged, no underflow occurred.
    let fund = env.fetch_fund(&fund_pda);
    assert_eq!(fund.total_balance, 100 * ONE_USDC);
    assert_eq!(fund.total_outflows, 0);
    assert_inv3(&fund);
}

// ─────────────────────────────────────────────────────────────────────────
// Test 9: draw rejects non-canonical core_invoker
// ─────────────────────────────────────────────────────────────────────────
#[test]
fn t09_draw_rejects_wrong_core() {
    let mut env = TestEnv::new();
    let (fund_pda, vault_pda) = init_tier(&mut env, Tier::Vault);

    let recipient = Keypair::new();
    let dest_ata = env.fund_token_account(&recipient.pubkey(), 0);

    let bogus_core = Pubkey::new_unique();
    let metas = metas_draw(bogus_core, fund_pda, vault_pda, dest_ata);
    let data = poolver_reserve::instruction::Draw { amount: ONE_USDC }.data();
    let ix = solana_instruction::Instruction {
        program_id: poolver_reserve::ID,
        accounts: metas,
        data,
    };
    let payer_kp = env.payer.insecure_clone();
    let result = send_ix(&mut env.svm, &payer_kp, ix);
    assert!(result.is_err(), "wrong core_invoker must be rejected");
}

// ─────────────────────────────────────────────────────────────────────────
// Test 10: seed happy path
// ─────────────────────────────────────────────────────────────────────────
#[test]
fn t10_seed_happy() {
    let mut env = TestEnv::new();
    let (fund_pda, vault_pda) = init_tier(&mut env, Tier::DeFi);

    // Funder: any signer in V1 (SPEC_QUESTION-26).
    let funder = Keypair::new();
    env.svm.airdrop(&funder.pubkey(), SOL).unwrap();
    let source_ata = env.fund_token_account(&funder.pubkey(), 750 * ONE_USDC);

    let amount = 300 * ONE_USDC;
    let metas = metas_seed(funder.pubkey(), fund_pda, vault_pda, source_ata);
    let data = poolver_reserve::instruction::Seed { amount }.data();
    let ix = solana_instruction::Instruction {
        program_id: poolver_reserve::ID,
        accounts: metas,
        data,
    };
    let payer_kp = env.payer.insecure_clone();
    send_ix_signed(&mut env.svm, &payer_kp, &[&funder], ix).expect("seed failed");

    let fund = env.fetch_fund(&fund_pda);
    assert_eq!(fund.total_balance, amount);
    assert_eq!(fund.total_inflows, amount);
    assert_eq!(fund.total_outflows, 0);
    assert_inv3(&fund);

    assert_eq!(env.fetch_token_balance(&vault_pda), amount);
    assert_eq!(env.fetch_token_balance(&source_ata), 750 * ONE_USDC - amount);
}

// ─────────────────────────────────────────────────────────────────────────
// Test 11: reserve isolation — INV-4. The Tier 0 and Tier 1 reserves are
// independent accounts; a deposit into Tier 0 leaves Tier 1 untouched. The
// structural enforcement (different PDAs from different seed bytes) is
// what makes wrong-tier passing impossible at the constraint layer.
// ─────────────────────────────────────────────────────────────────────────
#[test]
fn t11_reserve_isolation_inv4() {
    let mut env = TestEnv::new();
    let (tier0_fund, tier0_vault) = init_tier(&mut env, Tier::Vault);
    let (tier1_fund, tier1_vault) = init_tier(&mut env, Tier::DeFi);

    // Distinct PDAs.
    assert_ne!(tier0_fund, tier1_fund);
    assert_ne!(tier0_vault, tier1_vault);

    // Deposit 400 USDC into Tier 0 only.
    let source_authority = Keypair::new();
    env.svm.airdrop(&source_authority.pubkey(), SOL).unwrap();
    let source_ata = env.fund_token_account(&source_authority.pubkey(), 1_000 * ONE_USDC);
    cpi_deposit(
        &mut env,
        tier0_fund,
        tier0_vault,
        &source_authority,
        source_ata,
        400 * ONE_USDC,
    )
    .expect("tier0 deposit failed");

    // Tier 0 moved.
    let t0 = env.fetch_fund(&tier0_fund);
    assert_eq!(t0.total_balance, 400 * ONE_USDC);
    assert_eq!(t0.total_inflows, 400 * ONE_USDC);
    assert_inv3(&t0);
    assert_eq!(env.fetch_token_balance(&tier0_vault), 400 * ONE_USDC);

    // Tier 1 is structurally untouched — both the on-chain account state
    // and the underlying token vault.
    let t1 = env.fetch_fund(&tier1_fund);
    assert_eq!(t1.total_balance, 0);
    assert_eq!(t1.total_inflows, 0);
    assert_eq!(t1.total_outflows, 0);
    assert_inv3(&t1);
    assert_eq!(env.fetch_token_balance(&tier1_vault), 0);

    // Cross-tier seed mismatch: try to call `deposit` against Tier 1 fund
    // but pass Tier 0's vault. Anchor's seeds constraint on
    // `reserve_usdc_vault` (seeds = [RESERVE_VAULT_SEED, &(reserve_fund.tier
    // as u8).to_le_bytes()]) re-derives from `reserve_fund.tier`, so passing
    // the wrong-tier vault yields ConstraintSeeds. This is the structural
    // enforcement promised by arch §11.
    let metas = metas_deposit(
        env.core_invoker,
        tier1_fund,
        tier0_vault, // WRONG: Tier 0 vault for a Tier 1 fund.
        source_ata,
        source_authority.pubkey(),
    );
    let data = poolver_reserve::instruction::Deposit {
        amount: 10 * ONE_USDC,
    }
    .data();
    let ix = forward_through_fake_core(metas, data, env.core_invoker);
    let payer_kp = env.payer.insecure_clone();
    let result = send_ix_signed(&mut env.svm, &payer_kp, &[&source_authority], ix);
    assert!(
        result.is_err(),
        "Cross-tier vault must be rejected by ConstraintSeeds (INV-4)"
    );

    // Final post-state: still nothing on Tier 1, still 400 USDC on Tier 0.
    let t1_final = env.fetch_fund(&tier1_fund);
    assert_eq!(t1_final.total_balance, 0);
    assert_inv3(&t1_final);
    let t0_final = env.fetch_fund(&tier0_fund);
    assert_eq!(t0_final.total_balance, 400 * ONE_USDC);
    assert_inv3(&t0_final);
}
