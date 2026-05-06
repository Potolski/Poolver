use anchor_lang::prelude::*;

// `poolver-core`'s program ID. Pinned at step-12 wire-up to match the
// real `poolver-core` declare_id!. Identical pattern to
// `poolver-yield-vault`'s `core_id.rs`. The `seeds::program` constraint
// references this constant; the test-side `fake_core` stub must mirror
// it (the harness asserts equality at startup, see
// `tests/common/mod.rs`).
pub const POOLVER_CORE_ID: Pubkey = pubkey!("2SsxJqMCYKCYesfzfXASgAPPz153j8tYMXpMKKmt2QXk");
