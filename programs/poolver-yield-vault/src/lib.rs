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

declare_id!("A3ERUDLAdqdwgqgAoYLftxA6F1QtxSHZYu8DpNDXyyUp");

/// Tier 0 yield adapter — holds USDC in a PDA-owned token account, no
/// external strategy. See spec §5.3 + arch §13 for the common adapter
/// interface this implements.
#[program]
pub mod poolver_yield_vault {
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
}
