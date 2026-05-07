//! Test harness for `poolver-core`.
//!
//! Builds a LiteSVM environment containing all three V1 step-4 programs:
//! `poolver-core` (under test), `poolver-reserve`, and
//! `poolver-yield-vault`. Tests drive `poolver-core` end-to-end; CPIs
//! into reserve and yield-vault hit the REAL adapter / reserve binaries
//! and assert on their post-call state. No fake-core stub here — core
//! IS the program, so the `core_invoker` PDA is naturally derived from
//! `poolver_core::ID`.

#![allow(dead_code)]

use anchor_lang::AccountDeserialize;
use litesvm::LiteSVM;
use solana_account::Account;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_message::Message;
use solana_program_option::COption;
use solana_program_pack::Pack;
use solana_pubkey::{pubkey, Pubkey};
use solana_signer::Signer;
use solana_transaction::Transaction;
use spl_associated_token_account_interface::address::get_associated_token_address;
use spl_token_interface::{
    state::{Account as SplTokenAccount, AccountState, Mint as SplMint},
    ID as TOKEN_PROGRAM_ID,
};

pub use poolver_core::constants::*;
pub use poolver_core::state::{
    Bid, KycAttestation, KycLevel, Participant, Pool, ProtocolConfig, Tier, UserReputation,
};

pub const USDC_DECIMALS: u8 = 6;
pub const ONE_USDC: u64 = 1_000_000;
pub const SOL: u64 = 1_000_000_000;

pub const SYSTEM_PROGRAM: Pubkey = pubkey!("11111111111111111111111111111111");
pub const RENT_SYSVAR: Pubkey = pubkey!("SysvarRent111111111111111111111111111111111");

pub struct TestEnv {
    pub svm: LiteSVM,
    pub usdc_mint: Pubkey,
    pub admin: Keypair,
    pub core_invoker: Pubkey,
    pub core_invoker_bump: u8,
}

impl TestEnv {
    pub fn new() -> Self {
        let mut svm = LiteSVM::new().with_default_programs();

        // Load all three programs from the workspace deploy artifacts.
        let core_elf = std::fs::read(core_so_path()).expect("Run `anchor build`");
        svm.add_program(poolver_core::ID, &core_elf)
            .expect("add core program");

        let reserve_elf = std::fs::read(reserve_so_path()).expect("Run `anchor build`");
        svm.add_program(poolver_reserve::ID, &reserve_elf)
            .expect("add reserve program");

        let yield_vault_elf =
            std::fs::read(yield_vault_so_path()).expect("Run `anchor build`");
        svm.add_program(poolver_yield_vault::ID, &yield_vault_elf)
            .expect("add yield-vault program");

        // SPEC_QUESTION-36 (step 13): yield-defi adapter for Tier 1
        // integration tests. Loaded unconditionally so the same TestEnv
        // can drive both tiers.
        let yield_defi_elf =
            std::fs::read(yield_defi_so_path()).expect("Run `anchor build`");
        svm.add_program(poolver_yield_defi::ID, &yield_defi_elf)
            .expect("add yield-defi program");

        let usdc_mint = create_usdc_mint(&mut svm);

        let admin = Keypair::new();
        svm.airdrop(&admin.pubkey(), 100 * SOL).unwrap();

        let (core_invoker, core_invoker_bump) =
            Pubkey::find_program_address(&[CORE_INVOKER_SEED], &poolver_core::ID);

        Self {
            svm,
            usdc_mint,
            admin,
            core_invoker,
            core_invoker_bump,
        }
    }

    // ───── PDA derivers ────────────────────────────────────────────────

    pub fn protocol_config_pda(&self) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[PROTOCOL_CONFIG_SEED], &poolver_core::ID)
    }

    pub fn protocol_fee_vault_pda(&self) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[PROTOCOL_FEE_VAULT_SEED], &poolver_core::ID)
    }

    pub fn pool_pda(&self, creator: &Pubkey, pool_id: u64) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[POOL_SEED, creator.as_ref(), &pool_id.to_le_bytes()],
            &poolver_core::ID,
        )
    }

    pub fn pool_usdc_vault_pda(&self, pool: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[POOL_USDC_VAULT_SEED, pool.as_ref()],
            &poolver_core::ID,
        )
    }

    pub fn collateral_vault_pda(&self, pool: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[COLLATERAL_VAULT_SEED, pool.as_ref()],
            &poolver_core::ID,
        )
    }

    pub fn bid_stake_vault_pda(&self, pool: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[BID_STAKE_VAULT_SEED, pool.as_ref()],
            &poolver_core::ID,
        )
    }

    pub fn bid_pda(&self, pool: &Pubkey, month: u8, user: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[BID_SEED, pool.as_ref(), &[month], user.as_ref()],
            &poolver_core::ID,
        )
    }

    pub fn participant_pda(&self, pool: &Pubkey, user: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[PARTICIPANT_SEED, pool.as_ref(), user.as_ref()],
            &poolver_core::ID,
        )
    }

    pub fn reputation_pda(&self, user: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[REPUTATION_SEED, user.as_ref()], &poolver_core::ID)
    }

    pub fn kyc_pda(&self, user: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[KYC_SEED, user.as_ref()], &poolver_core::ID)
    }

    pub fn reserve_fund_pda(&self, tier: Tier) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[RESERVE_FUND_SEED, &tier.seed_bytes()],
            &poolver_reserve::ID,
        )
    }

    pub fn reserve_vault_pda(&self, tier: Tier) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[RESERVE_VAULT_SEED, &tier.seed_bytes()],
            &poolver_reserve::ID,
        )
    }

    pub fn vault_adapter_pda(&self, pool: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[VAULT_ADAPTER_SEED, pool.as_ref()],
            &poolver_yield_vault::ID,
        )
    }

    pub fn vault_adapter_usdc_pda(&self, pool: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[VAULT_ADAPTER_USDC_SEED, pool.as_ref()],
            &poolver_yield_vault::ID,
        )
    }

    // SPEC_QUESTION-36 (step 13): Tier 1 adapter PDAs.

    pub fn defi_adapter_pda(&self, pool: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[DEFI_ADAPTER_SEED, pool.as_ref()],
            &poolver_yield_defi::ID,
        )
    }

    pub fn defi_adapter_usdc_pda(&self, pool: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[DEFI_ADAPTER_USDC_SEED, pool.as_ref()],
            &poolver_yield_defi::ID,
        )
    }

    pub fn defi_adapter_ktoken_pda(&self, pool: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[DEFI_ADAPTER_KTOKEN_SEED, pool.as_ref()],
            &poolver_yield_defi::ID,
        )
    }

    // ───── Token helpers ────────────────────────────────────────────────

    pub fn fund_token_account(&mut self, owner: &Pubkey, amount: u64) -> Pubkey {
        let ata = get_associated_token_address(owner, &self.usdc_mint);
        write_token_account(&mut self.svm, ata, owner, &self.usdc_mint, amount);
        ata
    }

    /// Mint a *new* USDC-shaped mint distinct from `self.usdc_mint`. Used by
    /// the SPEC_QUESTION-26 close+rotate regression tests so we can prove
    /// that `initialize_protocol` re-runs successfully against a fresh mint
    /// after `admin_close_protocol`.
    pub fn create_extra_usdc_mint(&mut self) -> Pubkey {
        create_usdc_mint(&mut self.svm)
    }

    pub fn fetch_token_balance(&self, account: &Pubkey) -> u64 {
        let acct = self.svm.get_account(account).expect("token account missing");
        SplTokenAccount::unpack(&acct.data).unwrap().amount
    }

    pub fn fetch_token_balance_opt(&self, account: &Pubkey) -> Option<u64> {
        let acct = self.svm.get_account(account)?;
        if acct.data.is_empty() {
            return None;
        }
        Some(SplTokenAccount::unpack(&acct.data).ok()?.amount)
    }

    // ───── Account fetchers ────────────────────────────────────────────

    pub fn fetch_protocol_config(&self) -> ProtocolConfig {
        let (pda, _) = self.protocol_config_pda();
        let acct = self.svm.get_account(&pda).expect("config missing");
        ProtocolConfig::try_deserialize(&mut acct.data.as_ref()).unwrap()
    }

    pub fn fetch_pool(&self, pda: &Pubkey) -> Pool {
        let acct = self.svm.get_account(pda).expect("pool missing");
        Pool::try_deserialize(&mut acct.data.as_ref()).unwrap()
    }

    pub fn fetch_participant(&self, pda: &Pubkey) -> Participant {
        let acct = self.svm.get_account(pda).expect("participant missing");
        Participant::try_deserialize(&mut acct.data.as_ref()).unwrap()
    }

    pub fn fetch_reputation(&self, pda: &Pubkey) -> UserReputation {
        let acct = self.svm.get_account(pda).expect("reputation missing");
        UserReputation::try_deserialize(&mut acct.data.as_ref()).unwrap()
    }

    pub fn fetch_kyc(&self, pda: &Pubkey) -> KycAttestation {
        let acct = self.svm.get_account(pda).expect("kyc missing");
        KycAttestation::try_deserialize(&mut acct.data.as_ref()).unwrap()
    }

    pub fn fetch_bid(&self, pda: &Pubkey) -> Bid {
        let acct = self.svm.get_account(pda).expect("bid missing");
        Bid::try_deserialize(&mut acct.data.as_ref()).unwrap()
    }

    pub fn account_exists(&self, pda: &Pubkey) -> bool {
        match self.svm.get_account(pda) {
            Some(a) => !a.data.is_empty() && a.lamports > 0,
            None => false,
        }
    }
}

// ───── Path helpers ─────────────────────────────────────────────────────

fn target_root() -> std::path::PathBuf {
    if std::path::Path::new("target/deploy").exists() {
        "target/deploy".into()
    } else {
        "../../target/deploy".into()
    }
}

fn core_so_path() -> String {
    target_root().join("poolver_core.so").to_string_lossy().to_string()
}

fn reserve_so_path() -> String {
    target_root().join("poolver_reserve.so").to_string_lossy().to_string()
}

fn yield_vault_so_path() -> String {
    target_root().join("poolver_yield_vault.so").to_string_lossy().to_string()
}

fn yield_defi_so_path() -> String {
    target_root().join("poolver_yield_defi.so").to_string_lossy().to_string()
}

// ───── Mint / token-account fixtures (mirrored from peer suites) ────────

fn create_usdc_mint(svm: &mut LiteSVM) -> Pubkey {
    let mint = Pubkey::new_unique();
    let mint_authority = Pubkey::new_unique();
    let mint_state = SplMint {
        mint_authority: COption::Some(mint_authority),
        supply: 1_000_000_000 * ONE_USDC,
        decimals: USDC_DECIMALS,
        is_initialized: true,
        freeze_authority: COption::None,
    };
    let mut mint_data = vec![0u8; SplMint::LEN];
    SplMint::pack(mint_state, &mut mint_data).unwrap();
    let mint_account = Account {
        lamports: 1_461_600,
        data: mint_data,
        owner: TOKEN_PROGRAM_ID,
        executable: false,
        rent_epoch: 0,
    };
    svm.set_account(mint, mint_account).unwrap();
    mint
}

fn write_token_account(
    svm: &mut LiteSVM,
    address: Pubkey,
    owner: &Pubkey,
    mint: &Pubkey,
    amount: u64,
) {
    let token_state = SplTokenAccount {
        mint: *mint,
        owner: *owner,
        amount,
        delegate: COption::None,
        state: AccountState::Initialized,
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::None,
    };
    let mut data = vec![0u8; SplTokenAccount::LEN];
    SplTokenAccount::pack(token_state, &mut data).unwrap();
    let acct = Account {
        lamports: 2_039_280,
        data,
        owner: TOKEN_PROGRAM_ID,
        executable: false,
        rent_epoch: 0,
    };
    svm.set_account(address, acct).unwrap();
}

// ───── Tx helpers ───────────────────────────────────────────────────────

pub fn send_ix(
    svm: &mut LiteSVM,
    payer: &Keypair,
    ix: Instruction,
) -> Result<(), String> {
    let blockhash = svm.latest_blockhash();
    let message = Message::new(&[ix], Some(&payer.pubkey()));
    let tx = Transaction::new(&[payer], message, blockhash);
    svm.send_transaction(tx).map(|_| ()).map_err(|e| format!("{e:?}"))
}

pub fn send_ix_signed(
    svm: &mut LiteSVM,
    payer: &Keypair,
    extra: &[&Keypair],
    ix: Instruction,
) -> Result<(), String> {
    let blockhash = svm.latest_blockhash();
    let message = Message::new(&[ix], Some(&payer.pubkey()));
    let mut signers: Vec<&Keypair> = vec![payer];
    signers.extend_from_slice(extra);
    let tx = Transaction::new(&signers, message, blockhash);
    svm.send_transaction(tx).map(|_| ()).map_err(|e| format!("{e:?}"))
}
