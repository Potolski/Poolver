//! Cross-program-invocation helpers shared across `poolver-core`'s
//! adapter-touching instructions (`create_pool`, `contribute`,
//! `claim_winning`, `distribute_yield`).
//!
//! ## SPEC_QUESTION-36 — tier dispatch via `remaining_accounts`
//!
//! Step 13 wires Tier-1 (DeFi) into the four adapter-touching
//! instructions. The chosen wiring strategy (architect default for V1,
//! per QUESTIONS.md SPEC_QUESTION-36 status note) is route (b) —
//! `remaining_accounts` for tier-specific extras — to avoid ballooning
//! the fixed Anchor account contexts past the 4 KB BPF stack budget
//! (SPEC_QUESTION-15). The contexts already hold the byte-identical
//! leading prefix that BOTH adapters share (arch §13.2):
//!
//! ```text
//!   [core_invoker, adapter_state, adapter_usdc_vault, source_*..., token_program]
//! ```
//!
//! The Tier-1 surplus (kToken vault) is appended via `remaining_accounts`.
//! Order is documented per-helper in `adapter.rs`. The handlers pass the
//! correct adapter-program ID at the existing `yield_adapter_program`
//! account (Anchor `address` constraint dropped from the contexts and
//! re-validated in the dispatch helper against `pool.tier`).
//!
//! Other dispatch routes considered:
//!   - (a) Inline both Tier-0 and Tier-1 adapter contexts in every
//!     instruction. Rejected: doubles fixed-context size and pushes
//!     contribute / claim_winning over the 4 KB stack budget.
//!   - (c) Macro-generated specialized handlers per tier. Rejected: not
//!     IDL-friendly; Anchor clients see two near-identical instructions
//!     with overlapping accounts.

pub mod adapter;
