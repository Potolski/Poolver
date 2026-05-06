#![allow(dead_code)]

use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use litesvm::LiteSVM;
use solana_account::Account;
use solana_clock::Clock;
use solana_keypair::Keypair;
use solana_message::Message;
use solana_program_option::COption;
use solana_program_pack::Pack;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::Transaction;
use solana_instruction::Instruction;
use spl_associated_token_account_interface::address::get_associated_token_address;
use spl_token_interface::{
    state::{Account as SplTokenAccount, AccountState, Mint as SplMint},
    ID as TOKEN_PROGRAM_ID,
};

pub use poolver::constants::*;
pub use poolver::state::*;
pub use poolver::CreateGroupParams;

pub const USDC_DECIMALS: u8 = 6;
pub const ONE_USDC: u64 = 1_000_000;
pub const SOL: u64 = 1_000_000_000;

// Default test group parameters
pub const TEST_CONTRIBUTION: u64 = 100 * ONE_USDC; // 100 USDC
pub const TEST_MEMBERS: u8 = 3;
pub const TEST_COLLATERAL_BPS: u16 = 2000; // 20%
pub const TEST_INSURANCE_BPS: u16 = 300; // 3%

pub struct TestEnv {
    pub svm: LiteSVM,
    pub mint: Pubkey,
}

impl TestEnv {
    pub fn new() -> Self {
        let mut svm = LiteSVM::new().with_default_programs();

        // Load our program
        let elf = std::fs::read("target/deploy/poolver.so")
            .or_else(|_| std::fs::read("../../target/deploy/poolver.so"))
            .expect("Run `anchor build` first");
        svm.add_program(poolver::ID, &elf);

        // Create mock USDC mint
        let mint = Pubkey::new_unique();
        let mint_authority = Pubkey::new_unique();
        let mint_state = SplMint {
            mint_authority: COption::Some(mint_authority),
            supply: 1_000_000_000 * ONE_USDC,
            decimals: USDC_DECIMALS,
            is_initialized: true,
            freeze_authority: COption::None,
        };
        let mut mint_data = [0u8; SplMint::LEN];
        SplMint::pack(mint_state, &mut mint_data).unwrap();
        svm.set_account(
            mint,
            Account {
                lamports: SOL,
                data: mint_data.to_vec(),
                owner: TOKEN_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        )
        .unwrap();

        // Set a reasonable initial clock
        svm.set_sysvar::<Clock>(&Clock {
            unix_timestamp: 1_700_000_000, // ~Nov 2023
            slot: 100,
            epoch: 0,
            leader_schedule_epoch: 0,
            epoch_start_timestamp: 1_700_000_000,
        });

        Self { svm, mint }
    }

    /// Create a funded user with SOL and a USDC token account
    pub fn create_funded_user(&mut self, usdc_amount: u64) -> (Keypair, Pubkey) {
        let user = Keypair::new();
        self.svm.airdrop(&user.pubkey(), 10 * SOL).unwrap();

        let mint = self.mint;
        let ata = get_associated_token_address(&user.pubkey(), &mint);
        self.inject_token_account(&ata, &mint, &user.pubkey(), usdc_amount);

        (user, ata)
    }

    pub fn inject_token_account(
        &mut self,
        address: &Pubkey,
        mint: &Pubkey,
        owner: &Pubkey,
        amount: u64,
    ) {
        let token_acc = SplTokenAccount {
            mint: *mint,
            owner: *owner,
            amount,
            delegate: COption::None,
            state: AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        };
        let mut data = [0u8; SplTokenAccount::LEN];
        SplTokenAccount::pack(token_acc, &mut data).unwrap();
        self.svm
            .set_account(
                *address,
                Account {
                    lamports: SOL,
                    data: data.to_vec(),
                    owner: TOKEN_PROGRAM_ID,
                    executable: false,
                    rent_epoch: 0,
                },
            )
            .unwrap();
    }

    pub fn set_clock(&mut self, unix_timestamp: i64, slot: u64) {
        self.svm.set_sysvar::<Clock>(&Clock {
            unix_timestamp,
            slot,
            epoch: 0,
            leader_schedule_epoch: 0,
            epoch_start_timestamp: unix_timestamp,
        });
    }

    pub fn get_token_balance(&self, address: &Pubkey) -> u64 {
        match self.svm.get_account(address) {
            Some(account) => {
                SplTokenAccount::unpack(&account.data)
                    .map(|a| a.amount)
                    .unwrap_or(0)
            }
            None => 0,
        }
    }

    pub fn get_group(&self, address: &Pubkey) -> ConsorcioGroup {
        let account = self.svm.get_account(address).expect("Group not found");
        ConsorcioGroup::try_deserialize(&mut account.data.as_slice()).unwrap()
    }

    pub fn get_member(&self, address: &Pubkey) -> Member {
        let account = self.svm.get_account(address).expect("Member not found");
        Member::try_deserialize(&mut account.data.as_slice()).unwrap()
    }

    pub fn get_round(&self, address: &Pubkey) -> Round {
        let account = self.svm.get_account(address).expect("Round not found");
        Round::try_deserialize(&mut account.data.as_slice()).unwrap()
    }

    pub fn account_exists(&self, address: &Pubkey) -> bool {
        self.svm.get_account(address).is_some()
    }

    /// Send a transaction, returns Ok(()) on success or error string on failure
    pub fn send_tx(
        &mut self,
        signers: &[&Keypair],
        ixs: Vec<Instruction>,
    ) -> Result<(), String> {
        let payer = signers[0].pubkey();
        let msg = Message::new(&ixs, Some(&payer));
        let tx = Transaction::new(signers, msg, self.svm.latest_blockhash());
        self.svm
            .send_transaction(tx)
            .map(|_| ())
            .map_err(|e| format!("{:?}", e))
    }

    // ── Instruction builders ──────────────────────────────────────────

    pub fn create_group_ix(
        &self,
        creator: &Pubkey,
        params: CreateGroupParams,
    ) -> (Instruction, Pubkey) {
        let (group_pda, _) = derive_group_pda(creator, params.group_id);
        let (vault_pda, _) = derive_vault_pda(&group_pda);
        let (insurance_pda, _) = derive_insurance_pda(&group_pda);
        let (treasury_pda, _) = derive_treasury_pda(&group_pda);

        let accounts = poolver::accounts::CreateGroup {
            creator: *creator,
            group: group_pda,
            mint: self.mint,
            vault: vault_pda,
            insurance_vault: insurance_pda,
            treasury_vault: treasury_pda,
            token_program: TOKEN_PROGRAM_ID,
            system_program: anchor_lang::system_program::ID,
        };
        let ix = Instruction {
            program_id: poolver::ID,
            accounts: accounts.to_account_metas(None),
            data: poolver::instruction::CreateGroup { params }.data(),
        };
        (ix, group_pda)
    }

    pub fn join_group_ix(
        &self,
        user: &Pubkey,
        user_token_account: &Pubkey,
        group: &Pubkey,
    ) -> (Instruction, Pubkey) {
        let (member_pda, _) = derive_member_pda(group, user);
        let (vault_pda, _) = derive_vault_pda(group);

        let accounts = poolver::accounts::JoinGroup {
            user: *user,
            group: *group,
            member: member_pda,
            mint: self.mint,
            user_token_account: *user_token_account,
            vault: vault_pda,
            token_program: TOKEN_PROGRAM_ID,
            system_program: anchor_lang::system_program::ID,
        };
        let ix = Instruction {
            program_id: poolver::ID,
            accounts: accounts.to_account_metas(None),
            data: poolver::instruction::JoinGroup.data(),
        };
        (ix, member_pda)
    }

    pub fn leave_group_ix(
        &self,
        user: &Pubkey,
        user_token_account: &Pubkey,
        group: &Pubkey,
    ) -> Instruction {
        let (member_pda, _) = derive_member_pda(group, user);
        let (vault_pda, _) = derive_vault_pda(group);

        let accounts = poolver::accounts::LeaveGroup {
            user: *user,
            group: *group,
            member: member_pda,
            mint: self.mint,
            user_token_account: *user_token_account,
            vault: vault_pda,
            token_program: TOKEN_PROGRAM_ID,
            system_program: anchor_lang::system_program::ID,
        };
        Instruction {
            program_id: poolver::ID,
            accounts: accounts.to_account_metas(None),
            data: poolver::instruction::LeaveGroup.data(),
        }
    }

    pub fn activate_group_ix(
        &self,
        caller: &Pubkey,
        group: &Pubkey,
    ) -> Instruction {
        let accounts = poolver::accounts::ActivateGroup {
            caller: *caller,
            group: *group,
        };
        Instruction {
            program_id: poolver::ID,
            accounts: accounts.to_account_metas(None),
            data: poolver::instruction::ActivateGroup.data(),
        }
    }

    pub fn start_round_ix(
        &self,
        caller: &Pubkey,
        group: &Pubkey,
        round_number: u8,
    ) -> (Instruction, Pubkey) {
        let (round_pda, _) = derive_round_pda(group, round_number);

        let accounts = poolver::accounts::StartRound {
            caller: *caller,
            group: *group,
            round: round_pda,
            system_program: anchor_lang::system_program::ID,
        };
        let ix = Instruction {
            program_id: poolver::ID,
            accounts: accounts.to_account_metas(None),
            data: poolver::instruction::StartRound.data(),
        };
        (ix, round_pda)
    }

    pub fn make_payment_ix(
        &self,
        user: &Pubkey,
        user_token_account: &Pubkey,
        group: &Pubkey,
        round_number: u8,
    ) -> Instruction {
        let (member_pda, _) = derive_member_pda(group, user);
        let (round_pda, _) = derive_round_pda(group, round_number);
        let (vault_pda, _) = derive_vault_pda(group);
        let (insurance_pda, _) = derive_insurance_pda(group);

        let accounts = poolver::accounts::MakePayment {
            user: *user,
            group: *group,
            member: member_pda,
            round: round_pda,
            mint: self.mint,
            user_token_account: *user_token_account,
            vault: vault_pda,
            insurance_vault: insurance_pda,
            token_program: TOKEN_PROGRAM_ID,
        };
        Instruction {
            program_id: poolver::ID,
            accounts: accounts.to_account_metas(None),
            data: poolver::instruction::MakePayment.data(),
        }
    }

    pub fn close_collection_ix(
        &self,
        caller: &Pubkey,
        group: &Pubkey,
        round_number: u8,
    ) -> Instruction {
        let (round_pda, _) = derive_round_pda(group, round_number);

        let accounts = poolver::accounts::CloseCollection {
            caller: *caller,
            group: *group,
            round: round_pda,
        };
        Instruction {
            program_id: poolver::ID,
            accounts: accounts.to_account_metas(None),
            data: poolver::instruction::CloseCollection.data(),
        }
    }

    pub fn mark_default_ix(
        &self,
        caller: &Pubkey,
        group: &Pubkey,
        member_wallet: &Pubkey,
        round_number: u8,
    ) -> Instruction {
        let (member_pda, _) = derive_member_pda(group, member_wallet);
        let (round_pda, _) = derive_round_pda(group, round_number);
        let (vault_pda, _) = derive_vault_pda(group);
        let (insurance_pda, _) = derive_insurance_pda(group);

        let accounts = poolver::accounts::MarkDefault {
            caller: *caller,
            group: *group,
            member: member_pda,
            round: round_pda,
            mint: self.mint,
            vault: vault_pda,
            insurance_vault: insurance_pda,
            token_program: TOKEN_PROGRAM_ID,
        };
        Instruction {
            program_id: poolver::ID,
            accounts: accounts.to_account_metas(None),
            data: poolver::instruction::MarkDefault.data(),
        }
    }

    pub fn close_group_ix(
        &self,
        caller: &Pubkey,
        group: &Pubkey,
    ) -> Instruction {
        let accounts = poolver::accounts::CloseGroup {
            caller: *caller,
            group: *group,
        };
        Instruction {
            program_id: poolver::ID,
            accounts: accounts.to_account_metas(None),
            data: poolver::instruction::CloseGroup.data(),
        }
    }

    pub fn return_collateral_ix(
        &self,
        caller: &Pubkey,
        group: &Pubkey,
        member_wallet: &Pubkey,
        member_token_account: &Pubkey,
    ) -> Instruction {
        let (member_pda, _) = derive_member_pda(group, member_wallet);
        let (vault_pda, _) = derive_vault_pda(group);

        let accounts = poolver::accounts::ReturnCollateral {
            caller: *caller,
            group: *group,
            member: member_pda,
            mint: self.mint,
            member_token_account: *member_token_account,
            vault: vault_pda,
            token_program: TOKEN_PROGRAM_ID,
        };
        Instruction {
            program_id: poolver::ID,
            accounts: accounts.to_account_metas(None),
            data: poolver::instruction::ReturnCollateral.data(),
        }
    }

    pub fn distribute_insurance_ix(
        &self,
        caller: &Pubkey,
        group: &Pubkey,
        member_wallet: &Pubkey,
        member_token_account: &Pubkey,
    ) -> Instruction {
        let (member_pda, _) = derive_member_pda(group, member_wallet);
        let (insurance_pda, _) = derive_insurance_pda(group);

        let accounts = poolver::accounts::DistributeInsurance {
            caller: *caller,
            group: *group,
            member: member_pda,
            mint: self.mint,
            member_token_account: *member_token_account,
            insurance_vault: insurance_pda,
            token_program: TOKEN_PROGRAM_ID,
        };
        Instruction {
            program_id: poolver::ID,
            accounts: accounts.to_account_metas(None),
            data: poolver::instruction::DistributeInsurance.data(),
        }
    }

    pub fn skip_round_ix(
        &self,
        caller: &Pubkey,
        group: &Pubkey,
        round_number: u8,
    ) -> Instruction {
        let (round_pda, _) = derive_round_pda(group, round_number);

        let accounts = poolver::accounts::SkipRound {
            caller: *caller,
            group: *group,
            round: round_pda,
        };
        Instruction {
            program_id: poolver::ID,
            accounts: accounts.to_account_metas(None),
            data: poolver::instruction::SkipRound.data(),
        }
    }
}

// ── PDA Derivation Helpers ──────────────────────────────────────────

pub fn derive_group_pda(creator: &Pubkey, group_id: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[GROUP_SEED, creator.as_ref(), &group_id.to_le_bytes()],
        &poolver::ID,
    )
}

pub fn derive_vault_pda(group: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[VAULT_SEED, group.as_ref()], &poolver::ID)
}

pub fn derive_insurance_pda(group: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[INSURANCE_SEED, group.as_ref()], &poolver::ID)
}

pub fn derive_treasury_pda(group: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[TREASURY_SEED, group.as_ref()], &poolver::ID)
}

pub fn derive_member_pda(group: &Pubkey, wallet: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[MEMBER_SEED, group.as_ref(), wallet.as_ref()], &poolver::ID)
}

pub fn derive_round_pda(group: &Pubkey, round_number: u8) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[ROUND_SEED, group.as_ref(), &[round_number]],
        &poolver::ID,
    )
}

// ── Test scenario helpers ───────────────────────────────────────────

/// Default group params for testing
pub fn default_params(group_id: u64) -> CreateGroupParams {
    CreateGroupParams {
        monthly_contribution: TEST_CONTRIBUTION,
        total_members: TEST_MEMBERS,
        collateral_bps: TEST_COLLATERAL_BPS,
        insurance_bps: TEST_INSURANCE_BPS,
        description: "Test group".to_string(),
        group_id,
    }
}

/// Calculate collateral amount for default test params
pub fn expected_collateral() -> u64 {
    // (monthly_contribution * total_members * collateral_bps) / 10_000
    (TEST_CONTRIBUTION as u128 * TEST_MEMBERS as u128 * TEST_COLLATERAL_BPS as u128 / 10_000)
        as u64
}

/// Create a group with all members joined and activated.
/// Returns (group_pda, vec of (keypair, token_account, member_pda))
pub fn setup_active_group(
    env: &mut TestEnv,
    group_id: u64,
) -> (Pubkey, Vec<(Keypair, Pubkey, Pubkey)>) {
    let collateral = expected_collateral();
    let user_balance = collateral + 500 * ONE_USDC; // extra for payments

    // Creator creates the group
    let (creator, creator_ata) = env.create_funded_user(user_balance);
    let params = default_params(group_id);
    let (create_ix, group_pda) = env.create_group_ix(&creator.pubkey(), params);
    env.send_tx(&[&creator], vec![create_ix]).unwrap();

    // Creator joins
    let (join_ix, creator_member) = env.join_group_ix(&creator.pubkey(), &creator_ata, &group_pda);
    env.send_tx(&[&creator], vec![join_ix]).unwrap();

    let mut members = vec![(creator, creator_ata, creator_member)];

    // Additional members join
    for _ in 1..TEST_MEMBERS {
        let (user, user_ata) = env.create_funded_user(user_balance);
        let (join_ix, member_pda) = env.join_group_ix(&user.pubkey(), &user_ata, &group_pda);
        env.send_tx(&[&user], vec![join_ix]).unwrap();
        members.push((user, user_ata, member_pda));
    }

    // Activate
    let activate_ix = env.activate_group_ix(&members[0].0.pubkey(), &group_pda);
    env.send_tx(&[&members[0].0], vec![activate_ix]).unwrap();

    (group_pda, members)
}
