# Poolver V1 — Implementation Specification for Claude Code

You are implementing the Solana programs for **Poolver V1**, an on-chain consórcio (rotating savings and credit association / ROSCA) protocol. This document is the complete specification. Follow it precisely. Where it is silent, prefer the simplest, most auditable implementation that does not violate any stated invariant.

---

## 0. How to Use This Spec

**Use the `solana-architect` agent for this work.** This project is a multi-program Solana protocol with non-trivial economic invariants, cross-program invocations, account-layout decisions, and security requirements that benefit from specialized Solana expertise. Before writing any program code, invoke the `solana-architect` agent to:

1. Review this entire specification end-to-end
2. Propose the account layouts, PDA seeds, and CPI boundaries
3. Surface any Solana-specific concerns (compute budget limits, account size limits, rent exemption, transaction size, CPI depth)
4. Validate the program split and instruction granularity
5. Identify any places where this spec contradicts Solana/Anchor best practices, and flag them in `QUESTIONS.md` rather than silently working around them

After the architect has reviewed and produced an architecture document, proceed with implementation. Re-engage the agent at every major program boundary (e.g., before starting `poolver-yield-defi`, before integrating Kamino, before writing the default cascade).

For routine implementation work inside an established architecture (writing instruction handlers, tests, SDK methods), normal mode is fine.

---

## 1. Mission

Build a set of Solana programs (Anchor framework, Rust) that implement a 12-participant, 12-month rotating contribution pool with sealed-bid winner selection, graduated collateral, tier-segregated reserve funds, and pluggable yield strategies. The product is described in detail below.

Goals, in priority order:

1. **Correctness under adversarial conditions.** Every state transition must preserve solvency invariants. A user must never be able to extract more than they are owed.
2. **Auditability.** Code must be readable by a Solana security auditor (Ottersec, Neodyme, Offside Labs) with minimal explanation. Prefer explicit, repetitive code over clever abstractions.
3. **Testability.** Every instruction must have unit tests and integration tests. Critical flows (winner selection, default cascade, yield deployment) must have fork-mainnet tests.
4. **Upgradability with bounded scope.** Use Anchor's upgrade authority pattern, but minimize what an upgrade can do — admin must NEVER be able to drain user funds, even via upgrade.

---

## 2. Tech Stack & Conventions

- **Language:** Rust
- **Framework:** Anchor (latest stable, 0.30+)
- **Network:** Solana mainnet-beta target; develop on devnet and fork-mainnet
- **Asset:** USDC SPL token (`EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` on mainnet) only
- **Randomness:** Switchboard VRF for lottery selection
- **Time:** `Clock` sysvar; do not trust client-supplied timestamps for state transitions
- **Math:** All percentage math uses basis points (bps). 100% = 10_000 bps. Use `u64` for token amounts, `u128` for intermediate multiplication, then cast back. Never use floats.
- **Errors:** Use Anchor's `#[error_code]` with descriptive messages. One error variant per failure mode.
- **Events:** Emit Anchor events for every state-changing instruction. These are the source of truth for the indexer/UI.
- **PDAs:** Document seeds in code comments. Use `bump` from `find_program_address`, store the bump in account state, use `with_bump` for re-derivation.
- **Naming:** snake_case for everything. No abbreviations except `pubkey`, `bps`, `cpi`.

### Repository structure

```
poolver/
├── Anchor.toml
├── Cargo.toml
├── programs/
│   ├── poolver-core/         # main pool logic
│   ├── poolver-reserve/      # reserve fund management
│   ├── poolver-yield-vault/  # Tier 0 adapter (no-yield)
│   └── poolver-yield-defi/   # Tier 1 adapter (Kamino)
├── tests/
│   ├── unit/
│   ├── integration/
│   └── fork-mainnet/
├── client/
│   └── src/                  # TypeScript SDK
├── scripts/
│   ├── deploy.ts
│   └── seed-reserve.ts
└── docs/
    ├── architecture.md
    └── invariants.md
```

---

## 3. Domain Model

### Pool

A `Pool` is a single 12-participant, 12-month consórcio cohort. Pool parameters are immutable after creation.

**Pool config (set at creation):**
- `pool_id`: u64, unique per creator
- `creator`: Pubkey
- `tier`: enum { Vault, DeFi }
- `contribution_amount`: u64 (in USDC, 6 decimals). Range: 100_000_000 (100 USDC) to 10_000_000_000 (10,000 USDC).
- `participant_count`: u8, fixed at 12
- `total_months`: u8, fixed at 12
- `start_timestamp`: i64, set when pool fills
- `month_duration_seconds`: i64, default 2_592_000 (30 days). Configurable per pool but only by protocol admin within bounds.
- `bid_window_seconds`: i64, default 172_800 (48 hours), but typical

**Pool state (mutable):**
- `participants`: Vec<Pubkey>, length 0..=12
- `current_month`: u8, 0..=12 (0 = filling, 1-12 = active month, 13 = complete)
- `winners`: Vec<MonthWinner>, one entry per completed month
- `current_month_started_at`: i64
- `bid_window_ends_at`: i64
- `total_contributed`: u64
- `total_distributed`: u64
- `is_complete`: bool

**MonthWinner:**
- `month`: u8
- `winner`: Pubkey
- `winning_bid`: u64 (0 if lottery)
- `gross_payout`: u64
- `net_payout`: u64 (gross − winning_bid)
- `selected_at`: i64
- `selection_method`: enum { Lottery, Bid }

### Participant

One `Participant` PDA per (pool, user). Created when user joins the pool.

- `pool`: Pubkey
- `user`: Pubkey
- `joined_at`: i64
- `paid_months`: u16 bitmap (bit N = month N+1 paid)
- `has_won`: bool
- `win_month`: u8 (0 if not won)
- `bid_amount_when_won`: u64
- `collateral_locked`: u64 (current locked amount; decreases as user pays after winning)
- `collateral_initial`: u64 (locked at win time)
- `is_defaulted`: bool
- `defaulted_at`: i64
- `completed_cycles_at_join`: u8 (snapshot of user's reputation when joining this pool)

### Bid

One `Bid` PDA per (pool, month, user) during a bid window. Uses commit-reveal.

- `pool`: Pubkey
- `month`: u8
- `user`: Pubkey
- `commit_hash`: [u8; 32] // sha256(bid_amount || nonce || user_pubkey)
- `committed_at`: i64
- `revealed`: bool
- `revealed_amount`: u64
- `revealed_at`: i64
- `is_winner`: bool

### ReserveFund

One `ReserveFund` PDA per tier (2 total, global, not per-pool).

- `tier`: enum
- `total_balance`: u64 (USDC)
- `total_inflows`: u64 (lifetime)
- `total_outflows`: u64 (lifetime, used to cover defaults)
- `usdc_vault`: Pubkey (token account holding the funds)

### UserReputation

One `UserReputation` PDA per user (global).

- `user`: Pubkey
- `pools_joined`: u32
- `pools_completed`: u32
- `pools_defaulted`: u32
- `total_contributed_lifetime`: u64
- `total_received_lifetime`: u64
- `kyc_status`: enum { None, Light, Full }
- `kyc_attestation`: Pubkey (PDA of the attestation account)
- `last_kyc_at`: i64

### KycAttestation (MOCKED FOR V1)

For V1, KYC is **mocked** — there is no real off-chain KYC integration. The data structure and integration points are preserved so real KYC can slot in later without protocol changes.

Mock implementation: an admin-controlled instruction `mock_issue_kyc` creates `KycAttestation` PDAs for any user. In production this will be replaced by an oracle-signed flow (Idwall/Sumsub).

KycAttestation fields (unchanged from production design):
- `user`: Pubkey
- `level`: enum { Light, Full }
- `issued_by`: Pubkey (admin pubkey for V1; real oracle later)
- `issued_at`: i64
- `expires_at`: i64 (12 months default)
- `cpf_hash`: [u8; 32] (for V1 mock: zeroed or arbitrary; real CPF hash later)
- `sanctions_clean`: bool (always true in V1 mock)

**V1 mock behavior:**
- Admin (or anyone in dev environments) calls `mock_issue_kyc(user, level)` to grant attestation
- All instructions that check KYC use the same verification logic as production — they don't know the attestation is mocked
- This means the protocol enforces KYC gates correctly throughout, and swapping to real KYC is purely a matter of changing who can call the issuance instruction

A `// MOCK_KYC:` comment marker should appear at every code site that will need attention when swapping to real KYC, so a future engineer can grep for them.

---

## 4. Mathematical Reference

All math in basis points unless stated otherwise.

### Fees (per monthly contribution)
- Protocol fee: **150 bps (1.5%)**
- Reserve contribution by tier:
  - Vault (Tier 0): **150 bps (1.5%)**
  - DeFi (Tier 1): **250 bps (2.5%)**

### Bid distribution
- To remaining participants (next-month discount): **7500 bps (75%)**
- To reserve fund of pool's tier: **2000 bps (20%)**
- To protocol: **500 bps (5%)**

### Yield distribution (Tier 1 only — Tier 0 generates no yield)
- To pool participants: **7000 bps (70%)**
- To reserve fund of pool's tier: **2000 bps (20%)**
- To protocol: **1000 bps (10%)**

### Bid cap
- Maximum bid per month: **2000 bps (20%) of monthly pot**
- Monthly pot = `participant_count × contribution_amount` = 12 × contribution_amount

### Collateral at win
- Baseline = `(total_months − win_month) × contribution_amount`
- Reputation multiplier (applied to baseline):
  - 0 completed cycles: **10000 bps (100%)**
  - 1 completed cycle: **7000 bps (70%)**
  - 2+ completed cycles: **5000 bps (50%)**
- Bid premium = `winning_bid × 2` (added on top of adjusted baseline)
- Total collateral required at win = `(baseline × reputation_multiplier / 10000) + bid_premium`

### Collateral release schedule
- After each on-time payment post-win: release `collateral_initial / months_remaining_at_win`
- All collateral fully released by final payment

### Tier 1 (DeFi) capital allocation
- Maximum deployed to Kamino: **7500 bps (75%) of pool idle capital**
- Minimum liquid (held as USDC): **2500 bps (25%)**
- Idle capital = `total_contributed − total_distributed − total_collateral_locked`

### Tier 1 circuit breakers (auto-unwind triggers)
- Kamino USDC utilization > **9500 bps (95%)**
- Oracle deviation from expected > **200 bps (2%)**
- Kamino program in paused state
- Any of above triggers full unwind of pool's Kamino position back to USDC

### Default cascade
- Day 0: payment due
- Day 1-5: grace period, 200 bps (2%) penalty accrues, notifications emitted
- Day 6: position suspended (no bidding, no joining new pools), full collateral marked at-risk
- Day 30: liquidation. Liquidate collateral to cover all remaining payments. If shortfall, draw from reserve fund of pool's tier. User reputation marked defaulted.

---

## 5. Programs and Instructions

### 5.1 `poolver-core`

The main program. Contains pool lifecycle, contributions, bidding, winner selection, distributions, collateral management, default handling, and the V1 mock KYC issuance.

#### Instructions

**`initialize_protocol`**
- One-time setup. Creates global protocol config PDA.
- Signer: deployer (becomes initial admin)
- Stores: admin pubkey, protocol fee vault, allowed tiers, paused flag
- Note: no separate `kyc_oracle` field for V1 — admin doubles as mock KYC issuer. A `kyc_oracle` field placeholder may be added now to reduce migration friction, but in V1 it is set equal to admin.

**`mock_issue_kyc`** *(V1 only — replace with oracle-signed instruction in production)*
- Admin-only. Creates a `KycAttestation` PDA for a given user at a given level.
- Args: `user: Pubkey`, `level: KycLevel`
- Signer: admin
- Marked with `// MOCK_KYC:` comment for later replacement
- In production, this will be replaced by `issue_kyc_attestation` callable by a designated KYC oracle pubkey, which itself is fed by an off-chain Idwall/Sumsub integration

**`create_pool`**
- Creator initializes a new pool.
- Args: `tier`, `contribution_amount`, optional `month_duration_seconds` override (within bounds)
- Validates: contribution within bounds, creator has sufficient USDC for first contribution
- Creates Pool PDA, Participant PDA for creator (if creator is also joining), pool-specific token vaults
- Emits `PoolCreated`

**`join_pool`**
- A user joins an open pool.
- Pre: pool not yet started (`current_month == 0`), participants < 12, user has Light KYC, user not already in pool
- Creates Participant PDA, transfers first month's contribution + reserve fee + protocol fee
- Snapshots user's `pools_completed` count into `completed_cycles_at_join` for reputation purposes
- If pool reaches 12 participants, automatically triggers `start_pool` logic
- Emits `ParticipantJoined`, possibly `PoolStarted`

**`contribute`**
- Existing participant pays the current month's contribution.
- Pre: pool active, participant in pool, current month not already paid by this participant, within month window
- Transfers `contribution_amount` to pool vault, splits fees:
  - Protocol fee → protocol vault
  - Reserve fee → tier reserve vault
  - Net contribution → pool USDC vault (or yield strategy if Tier 1)
- Updates `paid_months` bitmap
- If user has previously won, releases scheduled collateral chunk back to user
- Emits `Contribution`

**`commit_bid`**
- Submit a sealed bid for the current month's pot.
- Pre: bid window open, user is participant, user has not won yet, user has Full KYC (required for win, so check now to avoid late surprise)
- Args: `commit_hash` ([u8; 32])
- Creates Bid PDA, locks small bid stake (TBD: 1% of contribution as anti-spam) until reveal
- One commit per user per month; cannot replace
- Emits `BidCommitted`

**`reveal_bid`**
- Reveal bid after window closes.
- Pre: bid window closed, reveal window open (next 24 hours)
- Args: `bid_amount`, `nonce`
- Verify hash matches commit
- Validate: bid_amount ≤ bid cap (20% of pot), bid_amount > 0
- Updates Bid PDA with revealed amount
- Emits `BidRevealed`

**`select_winner`**
- Permissionless instruction; anyone can call after reveal window closes.
- Pre: month is active, current_month winner not yet selected, reveal window closed
- Logic:
  1. Filter eligible bids (revealed, valid amount, user has Full KYC, user not won)
  2. If any eligible bids: select highest. Ties broken by VRF.
  3. If no bids: trigger Switchboard VRF for lottery among non-winners with Full KYC
  4. After winner determined, calculate collateral required, mark winner pending claim
- Note: lottery via VRF requires async pattern — initial call requests randomness, callback completes selection
- Emits `WinnerSelected`

**`claim_winning`**
- Winner posts collateral and claims net payout.
- Pre: caller is selected winner of current month, has Full KYC, has not yet claimed
- Calculate required collateral per §4 math
- User transfers `required_collateral` to collateral vault PDA
- Calculate `gross_payout = monthly_pot`, `net_payout = gross_payout − winning_bid`
- Transfer `net_payout` USDC to winner from pool vault (unwinding from yield strategy if needed)
- Distribute bid value:
  - 75% to remaining participants (credited as reduction in their next contribution)
  - 20% to tier reserve
  - 5% to protocol
- Mark Participant.has_won = true, store collateral details
- Update UserReputation
- Emits `WinningClaimed`, `BidDistributed`

**`advance_month`**
- Permissionless; anyone can call when current month duration has elapsed and winner has claimed (or claim window expired).
- Validates: pool active, time elapsed
- Increments `current_month`
- Resets bid window for new month
- If `current_month > total_months`: marks pool complete, processes final settlement
- Emits `MonthAdvanced`, possibly `PoolCompleted`

**`mark_late_payment`**
- Permissionless; anyone can call after grace period if a participant hasn't paid current month.
- Marks participant as in-grace, applies penalty fee (added to their next contribution due)
- Emits `LatePayment`

**`suspend_participant`**
- Permissionless; called after grace period (day 6).
- Suspends participant from new bids/joins. Position locked.
- Emits `ParticipantSuspended`

**`liquidate_default`**
- Permissionless; called at day 30 of unpaid status.
- For winners who defaulted: liquidates their locked collateral, distributes to cover remaining payments other participants on the schedule. Any shortfall drawn from reserve fund.
- For non-winners who defaulted: forfeit their contributions to the reserve, removed from pool participation, lottery pool adjusts.
- Updates UserReputation.pools_defaulted++
- Emits `DefaultLiquidated`, possibly `ReserveDrawn`

**`distribute_yield`**
- Permissionless; called periodically (daily?) for Tier 1 pools to harvest and distribute accrued yield.
- Reads current yield from yield adapter via CPI
- Splits per §4 (70% participants, 20% reserve, 10% protocol)
- Participant share is held as a credit reducing future contributions, not paid out immediately (avoids gas cost per user)
- Emits `YieldDistributed`
- No-op for Tier 0 pools

**`emergency_pause`**
- Admin-only. Pauses new pool creation and new contributions. Existing pools continue, defaults still process. Used for incident response.
- Emits `ProtocolPaused`

**`emergency_unpause`**
- Admin-only.
- Emits `ProtocolUnpaused`

#### Critical invariants for poolver-core

1. **Solvency:** at all times, `pool_vault_balance + total_collateral_locked + tier_reserve_share ≥ total_obligations_to_participants`
2. **Single winner per month:** no two `MonthWinner` entries for the same `month`
3. **No double payment:** `paid_months` bitmap can only have bits set, never cleared
4. **Collateral monotonic decrease:** post-win collateral can only decrease via scheduled release or default; never increase
5. **KYC gate:** `claim_winning` MUST verify Full KYC attestation is non-expired (mock or real, same enforcement)
6. **Tier immutability:** once pool is created, `tier` cannot change
7. **Reserve isolation:** Tier 0 reserve never funds Tier 1 defaults, and vice versa
8. **Admin powerlessness:** there is NO instruction that allows admin to transfer user funds out of any vault. Admin can only pause, set fees within bounds, mock-issue KYC (V1) / rotate KYC oracle (production), and upgrade program (subject to multisig)

### 5.2 `poolver-reserve`

Manages per-tier reserve fund accounts (2 tiers: Vault and DeFi). Receives inflows from fees and bids, pays outflows for default coverage. Called via CPI from `poolver-core`.

#### Instructions

**`initialize_reserve`**
- Admin creates the two tier reserves at deployment time (one for Vault, one for DeFi).

**`deposit`** (CPI-only, called by core)
- Increases reserve balance, records inflow
- Both Vault reserve and DeFi reserve hold funds as raw USDC. The DeFi reserve does **not** itself take Kamino risk — it sits in plain USDC so it can pay out when defaults hit.
- Emits `ReserveDeposit`

**`draw`** (CPI-only, called by core during default liquidation)
- Decreases reserve balance, records outflow
- Reverts if insufficient balance (this is a critical failure mode that core must handle gracefully)
- Emits `ReserveDraw`

**`seed`** (admin)
- Allows admin to add USDC to a reserve fund (used at launch to seed initial coverage)

#### Invariants
- Reserve balance must never go negative (use checked arithmetic; revert on underflow)
- Both reserves hold raw USDC only; no yield strategy applied to reserve capital in V1

### 5.3 Yield adapter programs

Both adapters implement the same instruction interface for uniformity. `poolver-core` invokes via CPI based on pool tier.

Common interface:
- `deposit(amount: u64)` — adapter receives USDC, deploys to strategy
- `withdraw(amount: u64)` — adapter returns USDC; must succeed for amounts ≤ available liquidity
- `get_balance() -> u64` — read-only, returns current claim value in USDC equivalent
- `harvest() -> u64` — realizes yield since last harvest, returns the yield amount in USDC
- `emergency_unwind()` — full withdrawal triggered by circuit breaker

#### `poolver-yield-vault` (Tier 0)
- Holds USDC in a PDA-owned token account. No external interactions.
- `harvest` always returns 0
- `get_balance` returns vault token account balance
- Trivial implementation; serves as the default and as a reference implementation for the adapter interface

#### `poolver-yield-defi` (Tier 1)
- Integrates with Kamino Lend USDC supply.
- `deposit`: supply USDC to Kamino, receive kTokens; respect the 75% cap (track via state)
- `withdraw`: redeem kTokens, return USDC
- `get_balance`: kToken balance × current exchange rate
- `harvest`: yield = current_balance − last_recorded_balance, then update last_recorded_balance
- **Circuit breaker logic:**
  - On every instruction that interacts with Kamino, check: utilization rate, oracle freshness, paused state
  - If any trigger fires, automatically call `emergency_unwind`, set `tripped = true`, refuse further deposits until admin resets
  - `tripped` state is read by core; pools cannot make new contributions to a tripped tier without explicit admin reset
- **Liquidity buffer:**
  - Adapter tracks pool's deployed amount and maintains 25% liquid USDC at all times
  - On deposit, only 75% goes to Kamino; 25% stays in adapter PDA
  - On withdraw, drain liquid first; only redeem from Kamino if liquid is insufficient
- Read Kamino program ID and account layout from their official docs; use their published Rust SDK if available
- Engage `solana-architect` agent before starting Kamino integration to validate CPI structure and account size constraints

### 5.4 KYC verification (V1: mocked)

Not a separate program; a verification helper used by core.

For V1:
- Admin calls `mock_issue_kyc(user, level)` to create a `KycAttestation` PDA for any user
- All consuming instructions (`join_pool`, `commit_bid`, `claim_winning`) verify attestation existence, level, expiry, and `sanctions_clean` flag — exactly as they will in production
- The only difference between V1 and production is who can issue: admin pubkey vs. KYC oracle pubkey
- All call sites that touch KYC must include a `// MOCK_KYC:` comment so a future engineer can grep them

For production migration:
- Replace `mock_issue_kyc` with `issue_kyc_attestation` whose signer must equal `protocol_config.kyc_oracle`
- Off-chain Idwall/Sumsub integration produces the inputs that the oracle then signs and submits
- CPF is hashed before storage; on-chain we only ever store `cpf_hash` ([u8; 32], sha256)

---

## 6. Events

Emit Anchor events for every state-changing instruction. Indexers will rebuild full state from events. Required events:

- `PoolCreated`, `ParticipantJoined`, `PoolStarted`
- `Contribution`, `LatePayment`
- `BidCommitted`, `BidRevealed`, `WinnerSelected`, `WinningClaimed`
- `BidDistributed` (one per recipient discount, or one summary event)
- `MonthAdvanced`, `PoolCompleted`
- `ParticipantSuspended`, `DefaultLiquidated`
- `ReserveDeposit`, `ReserveDraw`, `ReserveSeeded`
- `YieldHarvested`, `YieldDistributed`
- `KycAttestationIssued` (mock or real, same event), `KycAttestationRevoked`
- `CircuitBreakerTripped`, `CircuitBreakerReset`
- `ProtocolPaused`, `ProtocolUnpaused`

Each event must carry enough data for an indexer to rebuild state without re-reading account state. Include pubkeys, amounts, timestamps.

---

## 7. Errors

Define one variant per failure mode. Examples (non-exhaustive):

- `PoolFull`, `PoolNotFull`, `PoolNotStarted`, `PoolAlreadyStarted`, `PoolComplete`
- `NotAParticipant`, `AlreadyParticipant`, `AlreadyWon`, `NotWinner`
- `BidWindowClosed`, `BidWindowOpen`, `BidExceedsCap`, `BidCommitMissing`, `BidRevealMismatch`
- `ContributionAlreadyMade`, `ContributionInsufficient`
- `KycMissing`, `KycExpired`, `KycInsufficientLevel`, `KycSanctionsHit`
- `CollateralInsufficient`, `CollateralLocked`
- `GracePeriodNotElapsed`, `DefaultThresholdNotReached`, `AlreadyLiquidated`
- `InvalidTier`, `TierMismatch`
- `CircuitBreakerTripped`, `YieldStrategyError`
- `ReserveInsufficient`
- `Unauthorized`, `ProtocolPaused`
- `MathOverflow`, `InvalidAmount`

Use descriptive messages. Auditors and integrators will read these.

---

## 8. Testing Requirements

### Unit tests (per program)
- Every instruction: happy path + every error variant
- Math functions: edge cases (zero, max u64, overflow boundaries)
- State transitions: every legal transition + every illegal transition rejected

### Integration tests (cross-program)
- Full pool lifecycle: create → fill → 12 months of contributions and winners → completion
- Test both tiers end-to-end (Vault and DeFi)
- Bid + reveal + winner claim flow with collateral and distribution
- Default cascade: missed payment → grace → suspend → liquidation, both for winners and non-winners
- Yield harvest and distribution for Tier 1 (DeFi)
- Reserve depletion scenario: artificially induce default that exceeds collateral, verify reserve covers it
- Reserve insufficiency scenario: verify graceful failure mode
- Mock KYC flow: admin issues attestation, user joins/bids/claims successfully; user without attestation correctly rejected at every gate

### Fork-mainnet tests
- Tier 1 (Kamino) integration: deposit, withdraw, harvest with real Kamino state
- Circuit breaker triggers: simulate utilization spike, oracle staleness, paused state
- Liquidity buffer: ensure withdrawals work even when Kamino is at 100% utilization

### Stress tests
- 100 pools running simultaneously
- Default cascade with 3+ defaults in same pool
- Bid window with all 12 participants bidding
- VRF callback timing edge cases

### Property-based tests
- For any sequence of valid instructions on a pool, solvency invariant holds
- For any participant, total received ≤ total contributed + their winning entitlement
- Reserve balance is always equal to (total inflows − total outflows)

Test framework: Anchor's built-in mocha tests for TypeScript-side, plus Rust unit tests with `solana-program-test`. Use `bankrun` for fast integration tests where mainnet fork is not needed.

---

## 9. Security Requirements

1. **Reentrancy:** Solana programs are not reentrant by default, but CPI to yield adapters could create unexpected paths. Validate state before and after any CPI; never trust yield adapter return values without bounds-checking.
2. **Account validation:** every account passed to every instruction must be validated against expected PDA derivation. Use Anchor's `seeds` and `bump` constraints, never raw pubkey comparison alone.
3. **Signer checks:** every privileged operation. Don't rely on Anchor's `Signer<'info>` type alone; double-check semantically (e.g., signer is the pool creator, not just any signer).
4. **Arithmetic:** use `checked_add`, `checked_sub`, `checked_mul`, `checked_div` everywhere. Overflow should error, never wrap.
5. **PDA bump storage:** store bumps in account state and use `with_bump` consistently. Never re-derive bumps in instruction handlers.
6. **Token account ownership:** every token account passed in must be verified as owned by the expected authority (a PDA, never a user, for protocol-controlled vaults).
7. **Time validation:** when comparing timestamps, always validate `current_time >= expected_time` (not equal); clock can be slightly nondeterministic.
8. **Front-running:** assume all transaction parameters are public. The commit-reveal pattern for bids handles this for bid amounts; nothing else should be MEV-sensitive.
9. **Upgrade authority:** initial deployment with admin upgrade authority; document plan to migrate to multisig (Squads 3-of-5) before mainnet launch.
10. **No `init_if_needed`:** explicit `init` only. Prevents account-existence-based confusion attacks.
11. **Mock KYC scope:** the `mock_issue_kyc` instruction must NEVER ship to mainnet. Use a Cargo feature flag (`mock-kyc`) that gates the instruction's compilation, and ensure mainnet builds use `--no-default-features` or an explicit `--release` profile that excludes it. Engage `solana-architect` to design this guard correctly.

---

## 10. Off-Chain Components (out of scope for this implementation, but design must support)

- **Indexer:** subscribes to events, builds queryable database. Your event design must enable this.
- **Frontend:** reads pool state and history. SDK in `client/` should provide ergonomic queries.
- **KYC oracle (production only):** off-chain service integrated with Idwall, holds private key for KYC oracle role, signs attestations. Out of scope for V1 (mocked); program structure must accept this swap cleanly.
- **Keeper bot:** calls permissionless instructions (`select_winner`, `advance_month`, `mark_late_payment`, `suspend_participant`, `liquidate_default`, `distribute_yield`) on schedule. Program must work even if keepers are slow — anyone can call these instructions, including affected users.

---

## 11. Out of Scope for V1

Do not implement these, even if natural extensions:
- Multiple stablecoins (USDC only)
- Real off-chain KYC integration (mocked for V1, see §5.4)
- Tier 2 or any additional yield strategy beyond Vault and DeFi (Kamino)
- Pool creator-defined parameters beyond tier and contribution_amount
- Cross-pool features
- Secondary market for participant slots
- Slashing of bidder stake on no-reveal (just forfeit the small anti-spam stake)
- Governance token, voting, or DAO
- Pool size or duration variants
- Refund mechanics for partial cancellation
- Insurance products beyond the reserve fund

If you find yourself implementing any of these, stop and add a TODO comment instead.

---

## 12. Deliverables

When complete, the repository should contain:

1. All four programs (`poolver-core`, `poolver-reserve`, `poolver-yield-vault`, `poolver-yield-defi`), building cleanly with `anchor build`
2. Full test suite passing: `anchor test`
3. Fork-mainnet test scripts that can be run with a single command, with documentation on required RPC and setup
4. TypeScript SDK in `client/` with typed methods for every instruction
5. Deployment scripts for devnet and mainnet (mainnet script must require explicit confirmation, AND must refuse to deploy if `mock-kyc` feature is enabled)
6. `docs/architecture.md`: high-level component diagram, sequence diagrams for happy-path pool lifecycle, default cascade, and yield harvest. **Initial draft produced by the `solana-architect` agent** before implementation begins.
7. `docs/invariants.md`: enumerated list of every solvency, security, and correctness invariant, with the test that verifies each
8. `docs/mock-to-production.md`: explicit checklist of every `// MOCK_KYC:` comment site and what needs to change in each one for production migration
9. `README.md`: build instructions, test instructions, deployment instructions, audit-readiness checklist

---

## 13. Style and Implementation Notes

- Prefer explicit over implicit. An auditor reading the code should not need to understand a clever trick.
- Comment WHY, not WHAT. The code shows what; comments explain why a particular constraint or check exists, often referencing the §X of this spec.
- One responsibility per instruction. If `claim_winning` is doing five things, it's doing too many — split it.
- Validate inputs at instruction boundary; never trust them inside.
- Keep account contexts (`#[derive(Accounts)]` structs) tight. Only include accounts the instruction actually uses.
- Use `require!` and `require_eq!` for invariant checks with clear errors. Don't silently fail.
- Log critical state changes with `msg!` for on-chain debugging, in addition to events.
- When in doubt about a design choice, choose the version that's easier to audit, even at modest performance cost.

---

## 14. Order of Implementation (suggested)

If implementing from scratch, this is a sensible order. Engage `solana-architect` agent at each major boundary:

1. **Architecture phase:** invoke `solana-architect`, produce `docs/architecture.md`, validate spec against Solana constraints
2. `poolver-yield-vault` (simplest, serves as reference implementation of the adapter interface)
3. `poolver-reserve` (simple state machine, two tiers)
4. `poolver-core` skeleton: protocol init, mock KYC issuance, pool creation, joining, basic contributions
5. `poolver-core`: month advance, contribution accounting
6. `poolver-core`: bid commit-reveal flow
7. `poolver-core`: winner selection (with VRF for lottery) — engage `solana-architect` for VRF integration
8. `poolver-core`: collateral lock and claim winning
9. `poolver-core`: bid distribution and yield distribution math
10. `poolver-core`: default cascade
11. **Architecture phase 2:** invoke `solana-architect` to validate Kamino integration approach
12. `poolver-yield-defi`: Kamino integration with circuit breakers
13. Full integration tests
14. Fork-mainnet tests
15. SDK and deployment scripts
16. Mainnet build guard against `mock-kyc` feature
17. Docs

After step 10, the protocol works end-to-end at Tier 0 (Vault) and is auditable as a vertical slice. Tier 1 (DeFi/Kamino) is additive.

---

## 15. Questions to Surface, Not Assume

If during implementation you encounter ambiguity in this spec, do not make a silent choice. Add a `// SPEC_QUESTION:` comment with the question, choose the most conservative interpretation, and continue. Examples of likely questions:

- Switchboard VRF pricing and account setup specifics
- Kamino program ID, exact account layout, and SDK availability
- Whether to use Anchor `init` or manual account creation for very large accounts
- Whether bid stake on no-reveal goes to reserve or protocol
- Exact mechanism for compile-time exclusion of `mock_issue_kyc` from mainnet builds (Cargo feature `mock-kyc` is the recommended approach)

Surface these in a `QUESTIONS.md` file at the repo root for the human to resolve before mainnet deployment. The `solana-architect` agent should also contribute to this file during architecture review.

---

This spec is the source of truth. If anything else in the repository conflicts with it, this spec wins. Build carefully.
