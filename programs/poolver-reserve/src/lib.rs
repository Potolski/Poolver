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

declare_id!("CfxRT3jsXWQZRev67ztqaNKCrHaKF6ieW9a1E8NDPvnx");

/// Tier-segregated reserve fund. Holds raw USDC per tier (Vault / DeFi)
/// and pays out during default-coverage liquidations. Spec §5.2 + arch
/// §3.5 + §11.
///
/// All mutating instructions except `initialize_reserve` and `seed` are
/// CPI-only from `poolver-core` (auth via `core_invoker` PDA, arch §5.2).
#[program]
pub mod poolver_reserve {
    use super::*;

    pub fn initialize_reserve(ctx: Context<InitializeReserve>, tier: Tier) -> Result<()> {
        handle_initialize_reserve(ctx, tier)
    }

    pub fn deposit(ctx: Context<ReserveDepositCtx>, amount: u64) -> Result<()> {
        handle_deposit(ctx, amount)
    }

    pub fn draw(ctx: Context<ReserveDrawCtx>, amount: u64) -> Result<()> {
        handle_draw(ctx, amount)
    }

    pub fn seed(ctx: Context<ReserveSeedCtx>, amount: u64) -> Result<()> {
        handle_seed(ctx, amount)
    }
}
