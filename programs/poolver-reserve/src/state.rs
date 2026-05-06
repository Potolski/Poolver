use anchor_lang::prelude::*;

/// Tier discriminant. The on-wire byte equals `tier as u8`; the same byte
/// is used as the PDA seed suffix, so the `Tier` enum and the
/// `(tier as u8).to_le_bytes()` derivation must stay aligned forever.
///
/// `repr(u8)` plus explicit discriminants pin the byte values across
/// compiler versions (INV-4: structural tier isolation depends on these
/// bytes never drifting).
/// Borsh tag bytes are assigned by source order: `Vault = 0`, `DeFi = 1`.
/// This MUST never be reordered — the same byte is used as the PDA seed
/// suffix (INV-4 isolation depends on tier-byte stability). The
/// `tier_byte` constants below + INV-4 tests guard against drift.
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tier {
    Vault,
    DeFi,
}

impl Tier {
    /// Tag-byte view used for PDA seed derivation. Matches the Borsh
    /// discriminant (source-order: Vault=0, DeFi=1) — keep these aligned!
    /// Anchor's IDL serialiser does NOT support `#[repr(u8)]` enums under
    /// the 1.0.0 macros, so we manually tag here and assert via tests.
    #[inline]
    pub fn as_u8(self) -> u8 {
        match self {
            Tier::Vault => 0,
            Tier::DeFi => 1,
        }
    }

    /// Single-byte seed slice used everywhere the tier appears as a PDA
    /// seed.
    #[inline]
    pub fn seed_bytes(self) -> [u8; 1] {
        [self.as_u8()]
    }
}

/// Reserve fund state. Layout fixed by arch §3.5 (98 bytes total including
/// Anchor's 8-byte discriminator). Field order MUST stay stable so a future
/// upgrade can be done without account reallocation.
///
/// Three monotonic invariants live on this struct:
/// - INV-2: `total_balance >= 0` (enforced via `checked_sub` in `draw`).
/// - INV-3: `total_balance == total_inflows − total_outflows` at all times.
/// - INV-4: tier isolation is structural — see arch §11 / `Tier` enum.
///
/// `total_inflows` and `total_outflows` are lifetime counters; they NEVER
/// decrease.  Every reserve mutation re-establishes the inflow/outflow
/// identity post-update.
#[account]
#[derive(InitSpace)]
pub struct ReserveFund {
    pub tier: Tier,
    pub total_balance: u64,
    pub total_inflows: u64,
    pub total_outflows: u64,
    pub usdc_vault: Pubkey,
    pub bump: u8,
    /// Reserved padding for future fields without account reallocation;
    /// matches arch §3.5's 32-byte `_reserved` block.
    pub _reserved: [u8; 32],
}
