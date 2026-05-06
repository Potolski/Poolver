mod common;

use common::*;
use solana_signer::Signer;

// ═══════════════════════════════════════════════════════════════════
// T-001: Group Creation
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_create_group_success() {
    let mut env = TestEnv::new();
    let (creator, _creator_ata) = env.create_funded_user(0);
    let params = default_params(1);
    let (ix, group_pda) = env.create_group_ix(&creator.pubkey(), params);

    env.send_tx(&[&creator], vec![ix]).unwrap();

    let group = env.get_group(&group_pda);
    assert_eq!(group.creator, creator.pubkey());
    assert_eq!(group.monthly_contribution, TEST_CONTRIBUTION);
    assert_eq!(group.total_members, TEST_MEMBERS);
    assert_eq!(group.current_members, 0);
    assert_eq!(group.current_round, 0);
    assert_eq!(group.status, GroupStatus::Forming);
    assert_eq!(group.collateral_bps, TEST_COLLATERAL_BPS);
    assert_eq!(group.insurance_bps, TEST_INSURANCE_BPS);
    assert_eq!(group.description, "Test group");

    // Vault and insurance vault should exist
    let (vault_pda, _) = derive_vault_pda(&group_pda);
    let (insurance_pda, _) = derive_insurance_pda(&group_pda);
    assert!(env.account_exists(&vault_pda));
    assert!(env.account_exists(&insurance_pda));
}

#[test]
fn test_create_group_invalid_size_too_small() {
    let mut env = TestEnv::new();
    let (creator, _) = env.create_funded_user(0);
    let mut params = default_params(2);
    params.total_members = 2; // below MIN_GROUP_SIZE (3)

    let (ix, _) = env.create_group_ix(&creator.pubkey(), params);
    let result = env.send_tx(&[&creator], vec![ix]);
    assert!(result.is_err(), "Should fail with InvalidGroupSize");
}

#[test]
fn test_create_group_invalid_size_too_large() {
    let mut env = TestEnv::new();
    let (creator, _) = env.create_funded_user(0);
    let mut params = default_params(3);
    params.total_members = 51; // above MAX_GROUP_SIZE (50)

    let (ix, _) = env.create_group_ix(&creator.pubkey(), params);
    let result = env.send_tx(&[&creator], vec![ix]);
    assert!(result.is_err(), "Should fail with InvalidGroupSize");
}

#[test]
fn test_create_group_contribution_too_low() {
    let mut env = TestEnv::new();
    let (creator, _) = env.create_funded_user(0);
    let mut params = default_params(4);
    params.monthly_contribution = 1_000_000; // 1 USDC, below MIN_CONTRIBUTION (10 USDC)

    let (ix, _) = env.create_group_ix(&creator.pubkey(), params);
    let result = env.send_tx(&[&creator], vec![ix]);
    assert!(result.is_err(), "Should fail with ContributionTooLow");
}

#[test]
fn test_create_group_invalid_collateral_zero() {
    let mut env = TestEnv::new();
    let (creator, _) = env.create_funded_user(0);
    let mut params = default_params(5);
    params.collateral_bps = 0;

    let (ix, _) = env.create_group_ix(&creator.pubkey(), params);
    let result = env.send_tx(&[&creator], vec![ix]);
    assert!(result.is_err(), "Should fail with InvalidCollateralBps");
}

#[test]
fn test_create_group_invalid_insurance_too_high() {
    let mut env = TestEnv::new();
    let (creator, _) = env.create_funded_user(0);
    let mut params = default_params(6);
    params.insurance_bps = 2_001; // above max 2000

    let (ix, _) = env.create_group_ix(&creator.pubkey(), params);
    let result = env.send_tx(&[&creator], vec![ix]);
    assert!(result.is_err(), "Should fail with InvalidInsuranceBps");
}

// ═══════════════════════════════════════════════════════════════════
// T-002: Join, Leave, Activate
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_join_group_success() {
    let mut env = TestEnv::new();
    let collateral = expected_collateral();

    // Create group
    let (creator, _) = env.create_funded_user(0);
    let (create_ix, group_pda) = env.create_group_ix(&creator.pubkey(), default_params(10));
    env.send_tx(&[&creator], vec![create_ix]).unwrap();

    // Join
    let (user, user_ata) = env.create_funded_user(collateral + ONE_USDC);
    let (join_ix, member_pda) = env.join_group_ix(&user.pubkey(), &user_ata, &group_pda);
    env.send_tx(&[&user], vec![join_ix]).unwrap();

    // Verify member state
    let member = env.get_member(&member_pda);
    assert_eq!(member.wallet, user.pubkey());
    assert_eq!(member.group, group_pda);
    assert_eq!(member.collateral_deposited, collateral);
    assert_eq!(member.payments_made, 0);
    assert_eq!(member.status, MemberStatus::Active);

    // Verify group updated
    let group = env.get_group(&group_pda);
    assert_eq!(group.current_members, 1);
    assert_eq!(group.active_members, 1);

    // Verify collateral transferred to vault
    let (vault_pda, _) = derive_vault_pda(&group_pda);
    assert_eq!(env.get_token_balance(&vault_pda), collateral);
    assert_eq!(
        env.get_token_balance(&user_ata),
        ONE_USDC // started with collateral + 1 USDC, deposited collateral
    );
}

#[test]
fn test_join_group_full() {
    let mut env = TestEnv::new();
    let collateral = expected_collateral();

    // Create group with 3 members
    let (creator, _) = env.create_funded_user(0);
    let (create_ix, group_pda) = env.create_group_ix(&creator.pubkey(), default_params(11));
    env.send_tx(&[&creator], vec![create_ix]).unwrap();

    // Fill all 3 slots
    for _ in 0..3 {
        let (user, user_ata) = env.create_funded_user(collateral + ONE_USDC);
        let (join_ix, _) = env.join_group_ix(&user.pubkey(), &user_ata, &group_pda);
        env.send_tx(&[&user], vec![join_ix]).unwrap();
    }

    // 4th member should fail
    let (extra_user, extra_ata) = env.create_funded_user(collateral + ONE_USDC);
    let (join_ix, _) = env.join_group_ix(&extra_user.pubkey(), &extra_ata, &group_pda);
    let result = env.send_tx(&[&extra_user], vec![join_ix]);
    assert!(result.is_err(), "Should fail with GroupFull");
}

#[test]
fn test_leave_group_refund() {
    let mut env = TestEnv::new();
    let collateral = expected_collateral();
    let initial_balance = collateral + 10 * ONE_USDC;

    // Create group
    let (creator, _) = env.create_funded_user(0);
    let (create_ix, group_pda) = env.create_group_ix(&creator.pubkey(), default_params(12));
    env.send_tx(&[&creator], vec![create_ix]).unwrap();

    // Join
    let (user, user_ata) = env.create_funded_user(initial_balance);
    let (join_ix, member_pda) = env.join_group_ix(&user.pubkey(), &user_ata, &group_pda);
    env.send_tx(&[&user], vec![join_ix]).unwrap();

    let balance_after_join = env.get_token_balance(&user_ata);
    assert_eq!(balance_after_join, initial_balance - collateral);

    // Leave — should get full refund
    let leave_ix = env.leave_group_ix(&user.pubkey(), &user_ata, &group_pda);
    env.send_tx(&[&user], vec![leave_ix]).unwrap();

    // Verify refund
    let balance_after_leave = env.get_token_balance(&user_ata);
    assert_eq!(balance_after_leave, initial_balance);

    // Member account should be closed
    assert!(!env.account_exists(&member_pda));

    // Group member count should decrease
    let group = env.get_group(&group_pda);
    assert_eq!(group.current_members, 0);
    assert_eq!(group.active_members, 0);
}

#[test]
fn test_activate_group_success() {
    let mut env = TestEnv::new();
    let (group_pda, members) = setup_active_group(&mut env, 13);

    let group = env.get_group(&group_pda);
    assert_eq!(group.status, GroupStatus::Active);
    assert_eq!(group.current_members, TEST_MEMBERS);
    assert_eq!(group.active_members, TEST_MEMBERS);
    assert!(group.round_started_at > 0);

    // Verify all members are Active
    for (_, _, member_pda) in &members {
        let member = env.get_member(member_pda);
        assert_eq!(member.status, MemberStatus::Active);
    }
}

#[test]
fn test_activate_group_not_full() {
    let mut env = TestEnv::new();
    let collateral = expected_collateral();

    // Create group
    let (creator, _) = env.create_funded_user(0);
    let (create_ix, group_pda) = env.create_group_ix(&creator.pubkey(), default_params(14));
    env.send_tx(&[&creator], vec![create_ix]).unwrap();

    // Only 1 member joins (need 3)
    let (user, user_ata) = env.create_funded_user(collateral + ONE_USDC);
    let (join_ix, _) = env.join_group_ix(&user.pubkey(), &user_ata, &group_pda);
    env.send_tx(&[&user], vec![join_ix]).unwrap();

    // Activate should fail
    let activate_ix = env.activate_group_ix(&user.pubkey(), &group_pda);
    let result = env.send_tx(&[&user], vec![activate_ix]);
    assert!(result.is_err(), "Should fail — group not full");
}
