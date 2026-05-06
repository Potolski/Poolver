use anchor_lang::prelude::*;

// `poolver-core` is now live; this constant is the program ID minted at
// step-4 wire-up. Prior history (SPEC_QUESTION-26) used a deterministic
// placeholder while `poolver-core` did not exist. The seeds::program
// callsite (`seeds::program = crate::POOLVER_CORE_ID`) is unchanged — only
// the value rotated. The fake-core stub under `tests/fake_core/` and the
// test harness's `FAKE_CORE_ID` MUST match this constant; the test-side
// assertion (see `tests/common/mod.rs`) fails loudly if drift occurs.
pub const POOLVER_CORE_ID: Pubkey = pubkey!("2SsxJqMCYKCYesfzfXASgAPPz153j8tYMXpMKKmt2QXk");
