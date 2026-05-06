use anchor_lang::prelude::*;

/// Tier 1 yield-adapter state. Layout fixed by arch §3.9 (target ≈ 251
/// bytes including Anchor's 8-byte discriminator). Field order MUST
/// stay stable so a future upgrade can swap the mock-only fields in the
/// reserved tail for real Kamino account references without account
/// reallocation. SPEC_QUESTION-19 / Q-20.
///
/// `total_deposited` / `total_deployed_to_kamino` / `liquid_reserved`
/// are bookkeeping ledgers, not balances. The authoritative USDC
/// balances live in the two PDA-owned token accounts (`usdc_vault` =
/// the liquid 25%, `ktoken_vault` = the simulated Kamino position
/// holding the deployed 75%). We never trust these ledger fields for
/// solvency checks — INV-21 / spec §9.1.
#[account]
#[derive(InitSpace)]
pub struct DefiAdapterState {
    /// The pool this adapter belongs to (foreign key into `poolver-core`).
    pub pool: Pubkey, // 32

    /// Liquid USDC vault (the 25% kept on-hand for fast withdrawals).
    pub usdc_vault: Pubkey, // 32

    /// "kToken" vault. SPEC_QUESTION-19: in the V1 mock this is just a
    /// second USDC token account simulating the Kamino kToken position
    /// (deployed 75%). When real Kamino lands, this stays a kToken
    /// account; the type is forward-compatible.
    pub ktoken_vault: Pubkey, // 32

    /// Placeholder for the Kamino reserve account reference.
    /// SPEC_QUESTION-19: in the V1 mock, set to `Pubkey::default()`.
    /// In production, this is the Kamino-Lend reserve account whose
    /// liquidity we supply into.
    pub kamino_reserve: Pubkey, // 32

    /// Cumulative net deposit ledger. Bumped by `deposit`, decremented
    /// (saturating) by `withdraw` / `emergency_unwind`.
    pub total_deposited: u64, // 8

    /// Bookkeeping for the deployed-to-Kamino (75%) leg.
    pub total_deployed_to_kamino: u64, // 8

    /// Bookkeeping for the liquid (25%) leg.
    pub liquid_reserved: u64, // 8

    /// Snapshot of `usdc_vault.amount + ktoken_vault.amount` taken at
    /// the last `harvest()`. The next `harvest()` returns the delta vs.
    /// this baseline. Initialized to 0 in `initialize_adapter`.
    pub last_recorded_balance: u64, // 8

    /// Circuit-breaker latch. Set by any failing safety check on
    /// `deposit` / `withdraw` / `harvest`; cleared by
    /// `reset_circuit_breaker` (admin-only). While `tripped == true`,
    /// every state-changing instruction except `reset_circuit_breaker`
    /// rejects with `CircuitBreakerTripped` (spec §4 + §5.3).
    pub tripped: bool, // 1

    /// Trip timestamp. 0 ⇔ `tripped == false`.
    pub tripped_at: i64, // 8

    /// Trip reason discriminant; values defined in `constants::TRIP_*`.
    /// `0` ⇔ `tripped == false`. Kept as `u8` so the field is upgrade-
    /// safe; future variants append to the constant set without state
    /// migration.
    pub tripped_reason: u8, // 1

    // ───── Mock-only safety inputs (SPEC_QUESTION-19/20/23) ─────────────
    //
    // These three fields fake the on-chain readings the production
    // adapter would pull from Kamino + Pyth. They live in the tail of
    // the state account so the real-Kamino swap zeroes them in place
    // without needing reallocation. The `mock_set_*` instructions
    // (gated by `mock-yield`) write them; the safety check in
    // `deposit` / `withdraw` reads them. Production replaces this trio
    // with `kamino_reserve.utilization_bps()` + Pyth oracle reads.
    pub mock_utilization_bps: u16,      // 2
    pub mock_oracle_deviation_bps: u16, // 2
    pub mock_kamino_paused: bool,       // 1

    /// Stored canonical bump for `DefiAdapterState`. Saves CU vs.
    /// `find_program_address` per arch §4 + INV-29.
    pub bump: u8, // 1

    /// Reserved tail for forward compat. Sized to land the struct near
    /// arch §3.9's 251-byte target (Anchor adds the 8-byte
    /// discriminator on top). Mock fields above eat 5 bytes of the
    /// nominal 64-byte reserve, so 56 left here keeps the total
    /// constant.
    pub _reserved: [u8; 56], // 56
}
