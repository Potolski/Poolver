//! Test harness for `poolver-reserve`.
//!
//! Builds a LiteSVM environment containing the reserve program and a tiny
//! fake-core stub (see `tests/fake_core/`) that owns the `core_invoker`
//! PDA. The fake-core IS NOT a substitute for `poolver-core` — it just
//! exists so the auth path (`seeds::program = POOLVER_CORE_ID`) can be
//! exercised end-to-end before the real core program lands
//! (SPEC_QUESTION-26).
//!
//! Mirrors `programs/poolver-yield-vault/tests/common/mod.rs` deliberately
//! — keeping the two harnesses in lock-step makes integration with the
//! eventual `poolver-core` test fixture straightforward.

#![allow(dead_code)]

use anchor_lang::AccountDeserialize;
use litesvm::LiteSVM;
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
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

pub use poolver_reserve::constants::*;
pub use poolver_reserve::state::*;
pub use poolver_reserve::POOLVER_CORE_ID;

pub const USDC_DECIMALS: u8 = 6;
pub const ONE_USDC: u64 = 1_000_000;
pub const SOL: u64 = 1_000_000_000;

// MUST match `programs/poolver-reserve/tests/fake_core/src/lib.rs`.
// Step-4 wire-up: rotated to the real `poolver-core` program ID.
pub const FAKE_CORE_ID: Pubkey = pubkey!("2SsxJqMCYKCYesfzfXASgAPPz153j8tYMXpMKKmt2QXk");

pub struct TestEnv {
    pub svm: LiteSVM,
    pub usdc_mint: Pubkey,
    pub payer: Keypair,
    /// Canonical `core_invoker` PDA — derived from the placeholder core ID
    /// just like the real reserve constraint (`seeds::program = POOLVER_CORE_ID`).
    pub core_invoker: Pubkey,
    pub core_invoker_bump: u8,
}

impl TestEnv {
    pub fn new() -> Self {
        let mut svm = LiteSVM::new().with_default_programs();

        // Load the reserve program.
        let reserve_elf = std::fs::read(reserve_so_path()).expect("Run `anchor build` first");
        svm.add_program(poolver_reserve::ID, &reserve_elf)
            .expect("add reserve program");

        // Load the fake-core stub. It owns the core_invoker PDA so we can
        // sign CPIs into the reserve end-to-end.
        let fake_core_elf = std::fs::read(fake_core_so_path())
            .expect("Run `cargo build-sbf` in tests/fake_core/");
        svm.add_program(FAKE_CORE_ID, &fake_core_elf).expect("add fake_core");

        // Sanity: the reserve's compile-time POOLVER_CORE_ID must equal the
        // fake_core's ID. If a future engineer rotates one, tests must fail
        // loudly here instead of mysteriously at runtime.
        assert_eq!(
            POOLVER_CORE_ID, FAKE_CORE_ID,
            "POOLVER_CORE_ID drifted from FAKE_CORE_ID — keep tests/fake_core/src/lib.rs in sync"
        );

        let usdc_mint = create_usdc_mint(&mut svm);

        let payer = Keypair::new();
        svm.airdrop(&payer.pubkey(), 100 * SOL).unwrap();

        let (core_invoker, core_invoker_bump) =
            Pubkey::find_program_address(&[CORE_INVOKER_SEED], &POOLVER_CORE_ID);

        Self {
            svm,
            usdc_mint,
            payer,
            core_invoker,
            core_invoker_bump,
        }
    }

    pub fn reserve_fund_pda(&self, tier: Tier) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[RESERVE_FUND_SEED, &[tier.as_u8()]],
            &poolver_reserve::ID,
        )
    }

    pub fn reserve_vault_pda(&self, tier: Tier) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[RESERVE_VAULT_SEED, &[tier.as_u8()]],
            &poolver_reserve::ID,
        )
    }

    pub fn fund_token_account(&mut self, owner: &Pubkey, amount: u64) -> Pubkey {
        let ata = get_associated_token_address(owner, &self.usdc_mint);
        write_token_account(&mut self.svm, ata, owner, &self.usdc_mint, amount);
        ata
    }

    pub fn fetch_fund(&self, pda: &Pubkey) -> ReserveFund {
        let acct = self.svm.get_account(pda).expect("reserve_fund missing");
        ReserveFund::try_deserialize(&mut acct.data.as_ref()).unwrap()
    }

    pub fn fetch_token_balance(&self, account: &Pubkey) -> u64 {
        let acct = self.svm.get_account(account).expect("token account missing");
        SplTokenAccount::unpack(&acct.data).unwrap().amount
    }

    pub fn fetch_fund_opt(&self, pda: &Pubkey) -> Option<ReserveFund> {
        let acct = self.svm.get_account(pda)?;
        if acct.data.is_empty() {
            return None;
        }
        ReserveFund::try_deserialize(&mut acct.data.as_ref()).ok()
    }
}

fn reserve_so_path() -> String {
    if std::path::Path::new("target/deploy/poolver_reserve.so").exists() {
        "target/deploy/poolver_reserve.so".into()
    } else {
        "../../target/deploy/poolver_reserve.so".into()
    }
}

fn fake_core_so_path() -> String {
    if std::path::Path::new(
        "programs/poolver-reserve/tests/fake_core/target/deploy/fake_core_reserve.so",
    )
    .exists()
    {
        "programs/poolver-reserve/tests/fake_core/target/deploy/fake_core_reserve.so".into()
    } else {
        "tests/fake_core/target/deploy/fake_core_reserve.so".into()
    }
}

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

/// Wrap a reserve call into a fake-core `forward` instruction so the
/// `core_invoker` PDA is a valid signer. `forwarded_metas` must contain
/// the reserve instruction's accounts in the exact order Anchor expects,
/// with the canonical `core_invoker` PDA as one of those entries (the
/// fake-core stub identifies it by key match and re-marks it as a signer
/// for the inner CPI).
pub fn forward_through_fake_core(
    forwarded_metas: Vec<AccountMeta>,
    target_data: Vec<u8>,
    core_invoker: Pubkey,
) -> Instruction {
    let mut metas = vec![
        AccountMeta::new_readonly(poolver_reserve::ID, false),
        AccountMeta::new_readonly(core_invoker, false),
    ];
    metas.extend(forwarded_metas);

    Instruction {
        program_id: FAKE_CORE_ID,
        accounts: metas,
        data: target_data,
    }
}

/// Send a single instruction signed by `payer`.
pub fn send_ix(svm: &mut LiteSVM, payer: &Keypair, ix: Instruction) -> Result<(), String> {
    let blockhash = svm.latest_blockhash();
    let message = Message::new(&[ix], Some(&payer.pubkey()));
    let tx = Transaction::new(&[payer], message, blockhash);
    svm.send_transaction(tx).map(|_| ()).map_err(|e| format!("{e:?}"))
}

/// Same, but with multiple signers (e.g., when `source_authority` is also
/// a keypair).
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
