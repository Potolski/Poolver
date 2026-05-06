use anchor_lang::prelude::*;

// `poolver-core` is now live; this constant is the program ID minted at
// step-4 wire-up. The seeds::program callsite is unchanged — only the
// value rotated. MUST stay byte-identical to
// `poolver-yield-vault::POOLVER_CORE_ID` and to `poolver_core::ID` —
// reserve and both adapters recognise the same `core_invoker` PDA.
pub const POOLVER_CORE_ID: Pubkey = pubkey!("2SsxJqMCYKCYesfzfXASgAPPz153j8tYMXpMKKmt2QXk");
