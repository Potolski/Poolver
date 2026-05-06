//! Fake poolver-core stub.
//!
//! Sole purpose: own the `core_invoker` PDA (seed `b"core_invoker"`) so that
//! `poolver-yield-vault` unit tests can drive its auth path end-to-end
//! without waiting for the real `poolver-core` program to land
//! (SPEC_QUESTION-26).
//!
//! Single entrypoint: `forward(ix_data: Vec<u8>)`.
//!
//! Account layout (passed as `accounts` to this program):
//!   0: target program (the yield-vault program, executable)
//!   1: core_invoker PDA (PDA of THIS program, seed `[b"core_invoker"]`)
//!   2..N: accounts to forward to the target instruction
//!
//! Instruction data is the (already-encoded) discriminator + args of the
//! target instruction. We `invoke_signed` it as-is, signing only the
//! `core_invoker` PDA.

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

// MUST match `POOLVER_CORE_ID` in `poolver-yield-vault/src/core_id.rs`.
// Step-4 wire-up: rotated from the placeholder to the real
// `poolver-core` program ID.
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
