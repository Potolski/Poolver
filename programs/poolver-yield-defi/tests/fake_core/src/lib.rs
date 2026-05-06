//! Fake poolver-core stub for `poolver-yield-defi` unit tests.
//!
//! Identical contract to the stub under
//! `poolver-yield-vault/tests/fake_core/`: owns the `core_invoker` PDA
//! (seed `b"core_invoker"`) so the adapter's auth path can be driven
//! end-to-end before the real `poolver-core` program lands
//! (SPEC_QUESTION-26). The stub forwards an arbitrary inner
//! instruction via `invoke_signed`, signing only the canonical
//! `core_invoker` PDA.
//!
//! Account layout:
//!   0: target program (the yield-defi program, executable)
//!   1: core_invoker PDA (PDA of THIS program, seed `[b"core_invoker"]`)
//!   2..N: accounts to forward to the target instruction

#![cfg(not(feature = "no-entrypoint"))]

use solana_program::{
    account_info::AccountInfo,
    entrypoint,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
    pubkey,
    pubkey::Pubkey,
};

// MUST match `POOLVER_CORE_ID` in `poolver-yield-defi/src/core_id.rs`.
// Step-12 wire-up: same value the real `poolver-core` declares.
solana_program::declare_id!("2SsxJqMCYKCYesfzfXASgAPPz153j8tYMXpMKKmt2QXk");

const CORE_INVOKER_SEED: &[u8] = b"core_invoker";

entrypoint!(process_instruction);

pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let target_program = &accounts[0];
    let core_invoker = &accounts[1];
    let forward_accounts = &accounts[2..];

    let (_, bump) = Pubkey::find_program_address(&[CORE_INVOKER_SEED], &id());

    let mut metas: Vec<AccountMeta> = Vec::with_capacity(forward_accounts.len());
    for ai in forward_accounts.iter() {
        if ai.key == core_invoker.key {
            metas.push(AccountMeta::new_readonly(*ai.key, true));
        } else if ai.is_writable {
            metas.push(AccountMeta::new(*ai.key, ai.is_signer));
        } else {
            metas.push(AccountMeta::new_readonly(*ai.key, ai.is_signer));
        }
    }

    let ix = Instruction {
        program_id: *target_program.key,
        accounts: metas,
        data: instruction_data.to_vec(),
    };

    let signer_seeds: &[&[&[u8]]] = &[&[CORE_INVOKER_SEED, &[bump]]];

    invoke_signed(&ix, forward_accounts, signer_seeds)
}

// Suppress unused warning when no-entrypoint feature is on.
#[allow(dead_code)]
const _: &Pubkey = &pubkey!("2SsxJqMCYKCYesfzfXASgAPPz153j8tYMXpMKKmt2QXk");
