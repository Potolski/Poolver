pub mod constants;
pub mod core_id;
pub mod error;
pub mod events;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use core_id::*;
pub use error::*;
pub use events::*;
pub use instructions::*;
pub use state::*;

declare_id!("DAitPF7KHzRDVWcV4XM3J7dYGrKJkH332dQHPYUiP7UP");

/// Tier 1 yield adapter — the Kamino-mock adapter. Same
/// instruction surface as `poolver-yield-vault` (`initialize_adapter`,
/// `deposit`, `withdraw`, `harvest`, `emergency_unwind`) so
/// `poolver-core` can dispatch on `pool.tier` against a single CPI
/// shape (arch §13 common interface; INV-21).
///
/// SPEC_QUESTION-19 / SPEC_QUESTION-20: this V1 build does NOT CPI
/// into real Kamino. The deployed (75%) leg is simulated via an
/// internal token transfer between two PDA-owned USDC token accounts;
/// "yield" is injected directly via the dev-only `mock_inject_yield`
/// helper (gated by the `mock-yield` Cargo feature). Every site that
/// real Kamino would replace is annotated with
/// `// SPEC_QUESTION-19:` so a future engineer can grep them in one
/// pass when the integration lands.
///
/// SPEC_QUESTION-23: the oracle-deviation breaker input is also
/// mocked (`mock_set_oracle_deviation`); production reads from a Pyth
/// USDC/USD price feed.
#[program]
pub mod poolver_yield_defi {
    use super::*;

    pub fn initialize_adapter(
        ctx: Context<InitializeAdapter>,
        pool: Pubkey,
    ) -> Result<()> {
        handle_initialize_adapter(ctx, pool)
    }

    pub fn deposit(ctx: Context<AdapterDeposit>, amount: u64) -> Result<()> {
        handle_deposit(ctx, amount)
    }

    pub fn withdraw(ctx: Context<AdapterWithdraw>, amount: u64) -> Result<()> {
        handle_withdraw(ctx, amount)
    }

    pub fn harvest(ctx: Context<AdapterHarvest>) -> Result<u64> {
        handle_harvest(ctx)
    }

    pub fn emergency_unwind(ctx: Context<AdapterUnwind>) -> Result<()> {
        handle_emergency_unwind(ctx)
    }

    /// Operator-driven breaker reset. Always present (not feature-gated).
    pub fn reset_circuit_breaker(ctx: Context<ResetCircuitBreaker>) -> Result<()> {
        handle_reset_circuit_breaker(ctx)
    }

    // ───── Mock-only helpers ─────
    //
    // SPEC_QUESTION-19/20/23: gated by `mock-yield`. Mainnet builds
    // (`--no-default-features`) drop these from the dispatch table,
    // IDL, and binary entirely. Same playbook as `poolver-core`'s
    // `mock_issue_kyc` (arch §10, INV-26).

    #[cfg(feature = "mock-yield")]
    pub fn mock_inject_yield(
        ctx: Context<MockInjectYield>,
        amount: u64,
    ) -> Result<()> {
        handle_mock_inject_yield(ctx, amount)
    }

    #[cfg(feature = "mock-yield")]
    pub fn mock_set_utilization(
        ctx: Context<MockSetBreakerInput>,
        bps: u16,
    ) -> Result<()> {
        handle_mock_set_utilization(ctx, bps)
    }

    #[cfg(feature = "mock-yield")]
    pub fn mock_set_oracle_deviation(
        ctx: Context<MockSetBreakerInput>,
        bps: u16,
    ) -> Result<()> {
        handle_mock_set_oracle_deviation(ctx, bps)
    }

    #[cfg(feature = "mock-yield")]
    pub fn mock_set_kamino_paused(
        ctx: Context<MockSetBreakerInput>,
        paused: bool,
    ) -> Result<()> {
        handle_mock_set_kamino_paused(ctx, paused)
    }
}
