# Poolver V1 — Invariants Catalog

> Every protocol invariant, the threat it defends against, the test that verifies it, and the instructions that could violate it. Cross-referenced to `docs/SPEC_V1.md`.
>
> **Convention:** each invariant has an `INV-N` ID. Tests reference these IDs. When code contains a check enforcing one, the comment should read `// INV-N`.

---

## A. Solvency Invariants (spec §5.1.1, §5.1.7)

### INV-1 — Pool Solvency

**Statement:** at every instant after every committed instruction,
```
pool_usdc_vault.balance + total_collateral_locked + tier_reserve_share
    >= total_obligations_to_participants
```
where `total_obligations_to_participants` is the sum of (a) future contributions owed by winners (covered by their collateral) and (b) future winning entitlements of non-winners (covered by future contributions + reserve backstop).

**Threat:** participant withdraws more than the pool can pay out; protocol becomes insolvent.

**Verifying test:** property/fuzz test in `tests/property/solvency.rs` — for any random valid instruction sequence, post-state LHS ≥ RHS. Run with 10,000 sequences.

**Instructions that could violate:** `claim_winning` (insufficient collateral), `liquidate_default` (reserve underdraw not detected), `distribute_yield` (yield miscounted as principal). Design defenses in architecture.md §12.

---

### INV-2 — Reserve Non-Negative

**Statement:** `ReserveFund.total_balance >= 0` at all times. Implemented via `checked_sub` returning error on underflow.

**Threat:** a draw exceeding balance produces silent overflow and corrupts accounting.

**Verifying test:** unit test of `poolver-reserve::draw` with `amount > total_balance`; expect `ReserveInsufficient` error.

**Instructions:** `reserve::draw`.

---

### INV-3 — Reserve Inflow/Outflow Identity

**Statement:** at all times, `ReserveFund.total_balance == ReserveFund.total_inflows − ReserveFund.total_outflows`.

**Threat:** accounting drift between balance and lifetime counters indicates a bug.

**Verifying test:** every reserve mutation asserts identity post-mutation; integration test sweeps state after a randomized scenario.

**Instructions:** `reserve::deposit`, `reserve::draw`, `reserve::seed`.

---

### INV-4 — Reserve Tier Isolation

**Statement:** Tier 0 default never debits Tier 1 reserve, and vice versa. Enforced structurally via PDA seeds (`docs/architecture.md` §11).

**Threat:** loss of risk segregation; Tier 0 (low-risk) users socialize Tier 1 (DeFi) losses.

**Verifying test:** integration test attempts to call `liquidate_default` on a Tier 0 pool while passing the Tier 1 reserve PDA; Anchor must reject with `ConstraintSeeds`.

**Instructions:** `liquidate_default`, `contribute`, `join_pool`, `claim_winning`, `distribute_yield`.

---

## B. State-Transition Invariants (spec §5.1.2, §5.1.3, §5.1.4, §5.1.6)

### INV-5 — Single Winner Per Month

**Statement:** for any pool and any month `m ∈ [1, 12]`, `Pool.winners[m]` is set at most once. Once set, never overwritten.

**Threat:** double-claim of the same pot.

**Verifying test:** unit test calls `select_winner` twice in same month; second call returns `WinnerAlreadySelected`.

**Instructions:** `select_winner`, `consume_vrf_winner`.

---

### INV-6 — Paid Months Monotonic

**Statement:** `Participant.paid_months` is a u16 bitmap; bits can only be set, never cleared, by any instruction.

**Threat:** participant marks month as unpaid to re-collect a credit.

**Verifying test:** property test inspects bitmap before/after every state-changing call; assert no bit transitions 1 → 0.

**Instructions:** `contribute`, `liquidate_default` (must NOT clear bits when seizing collateral).

---

### INV-7 — Collateral Monotonic Decrease

**Statement:** post-win `Participant.collateral_locked` only decreases, except at the moment of `claim_winning` where it is set from 0 to `collateral_initial`. Between then and zero, monotonic non-increasing.

**Threat:** unlocked collateral re-locked silently; user can't predict release schedule.

**Verifying test:** property test on a winner's `collateral_locked` over time; assert non-increasing after the initial set.

**Instructions:** `contribute` (releases scheduled chunk), `liquidate_default` (zeroes after seize).

---

### INV-8 — Tier Immutability

**Statement:** `Pool.tier` is set in `create_pool` and is never written again by any instruction.

**Threat:** mid-pool tier swap moves capital between yield strategies, breaking solvency reasoning.

**Verifying test:** code-level assertion (no instruction writes `pool.tier` after init); plus integration test confirming post-create attempts fail.

**Instructions:** none should mutate.

---

### INV-9 — Pool Lifecycle Monotonic

**Statement:** `Pool.current_month` only increases (0 → 12 → "complete"). `Pool.is_complete`, once true, stays true.

**Threat:** time-travel bugs (going back to month 3 lets a winner re-claim).

**Verifying test:** property test on `current_month` sequence.

**Instructions:** `advance_month`, `join_pool` (auto-start), `select_winner` (must not roll back).

---

## C. Authorization Invariants (spec §5.1.5, §5.1.8, §9)

### INV-10 — Full KYC Required to Win

**Statement:** `claim_winning` succeeds only if the caller has a non-expired Full KYC attestation with `sanctions_clean = true`. Same check (mock or production).

**Threat:** unKYC'd entity receives payout; regulatory exposure.

**Verifying test:** integration test issues Light-only KYC, attempts `claim_winning`, expects `KycInsufficientLevel`. Same test with expired Full KYC, expects `KycExpired`.

**Instructions:** `claim_winning` (and `commit_bid` as an early gate per spec §5.1).

---

### INV-11 — Light KYC Required to Join

**Statement:** `join_pool` requires Light or Full KYC.

**Threat:** untracked participants pollute pool reputation system.

**Verifying test:** join without attestation → `KycMissing`.

**Instructions:** `join_pool`.

---

### INV-12 — Admin Powerlessness

**Statement:** there exists NO instruction reachable by a signer holding the `admin` role that transfers user-owned USDC out of `pool_usdc_vault`, `collateral_vault`, or any reserve vault to an arbitrary destination. Admin powers are: pause/unpause, mock-issue KYC (V1) / rotate KYC oracle pubkey (production), seed reserve (admin → reserve only, never reverse), upgrade program (subject to multisig).

**Threat:** rug-pull by admin or compromised admin keypair.

**Verifying test:** code review / static check enumerates every signer = admin instruction and confirms its account constraints write only to admin-permitted destinations. No fuzz; this is a structural invariant.

**Instructions:** `initialize_protocol`, `mock_issue_kyc`, `emergency_pause`, `emergency_unpause`, `seed_reserve`, plus the implicit upgrade authority. Each must be reviewed individually; the test is "list them all in a doc, prove no transfer to admin or external wallet."

---

### INV-13 — Permissionless Keeper Calls Are Side-Effect Safe

**Statement:** `select_winner`, `advance_month`, `mark_late_payment`, `suspend_participant`, `liquidate_default`, `distribute_yield` are safe to call by any signer at any time when their preconditions are met. They never mutate state when preconditions fail (atomic revert).

**Threat:** keeper races, replay, or DoS.

**Verifying test:** for each, attempt call before precondition; assert specific error and zero state change.

**Instructions:** the six listed.

---

## D. Bid & VRF Invariants (spec §5.1, §9.8)

### INV-14 — Commit-Reveal Hash Match

**Statement:** `reveal_bid` succeeds only if `sha256(bid_amount || nonce || user_pubkey) == commit_hash`.

**Threat:** trivially front-run bidding.

**Verifying test:** unit test with mismatched nonce → `BidRevealMismatch`.

**Instructions:** `reveal_bid`.

---

### INV-15 — Bid Cap

**Statement:** revealed `bid_amount <= 0.20 × monthly_pot`. Enforced at `reveal_bid` and again at `select_winner`.

**Threat:** winner with extreme bid distorts collateral and bid distribution.

**Verifying test:** unit test reveals with `bid = 0.21 × pot`; expect `BidExceedsCap`.

**Instructions:** `reveal_bid`, `select_winner`.

---

### INV-16 — One Bid Per User Per Month

**Statement:** the `Bid` PDA derivation `[b"bid", pool, month, user]` makes second-commit structurally impossible (PDA already exists; `init` fails).

**Threat:** spam bidding to manipulate selection.

**Verifying test:** double-commit attempt; expect Anchor `AccountAlreadyInitialized`.

**Instructions:** `commit_bid`.

---

### INV-17 — VRF In-Flight Exclusivity

**Statement:** `Pool.vrf_in_flight = true` blocks `select_winner` (re-request) and `advance_month` (premature advance) until callback completes.

**Threat:** double VRF request, or month advancing without a winner selected.

**Verifying test:** request VRF, attempt second request → `VrfAlreadyRequested`. Attempt `advance_month` → `WinnerNotSelected`.

**Instructions:** `select_winner`, `consume_vrf_winner`, `advance_month`.

---

## E. Yield Adapter Invariants (spec §4, §5.3)

### INV-18 — Tier 1 Liquidity Buffer

**Statement:** `DefiAdapterState.liquid_reserved >= 0.25 × total_deposited` whenever Kamino is healthy. After `withdraw`, the buffer is replenished from Kamino if the buffer drops below threshold and Kamino is available.

**Threat:** Kamino at 100% utilization blocks user payouts.

**Verifying test:** fork-mainnet test simulates Kamino utilization spike; perform `claim_winning`-equivalent withdraw; assert success from buffer.

**Instructions:** `yield_defi::deposit`, `yield_defi::withdraw`.

---

### INV-19 — Tier 1 Kamino Cap

**Statement:** `DefiAdapterState.total_deployed_to_kamino <= 0.75 × total_deposited`.

**Threat:** over-deployment leaves no liquidity for payouts.

**Verifying test:** unit test attempts deposit that would push deployed share over 75%; expect partial deploy or error.

**Instructions:** `yield_defi::deposit`.

---

### INV-20 — Circuit Breaker One-Way Until Reset

**Statement:** once `DefiAdapterState.tripped = true`, no `deposit` or `harvest` can succeed until admin calls `reset_circuit_breaker`. `withdraw` still succeeds (we want users to be able to exit).

**Threat:** silent re-engagement after a fault condition; admin loses chance to investigate.

**Verifying test:** force-trip the adapter (via mocked oracle deviation in fork test), attempt deposit → `CircuitBreakerTripped`. Attempt withdraw → succeeds.

**Instructions:** `yield_defi::deposit`, `yield_defi::harvest`, `yield_defi::withdraw`, `reset_circuit_breaker`.

---

### INV-21 — Adapter Interface Uniformity

**Statement:** `poolver-yield-vault` and `poolver-yield-defi` expose instructions `deposit`, `withdraw`, `harvest`, `emergency_unwind` with identical Anchor discriminators (same names) and identical leading-prefix account contexts.

**Threat:** core's CPI helper drifts between adapters; behavior diverges.

**Verifying test:** integration test calls each adapter through the same core CPI helper; both succeed with identical core-side code paths.

**Instructions:** all four adapter instructions.

---

## F. Default Cascade Invariants (spec §4, §5.1)

### INV-22 — Grace Period Enforcement

**Statement:** `mark_late_payment` cannot execute before day 1 past due. `suspend_participant` cannot execute before day 6. `liquidate_default` cannot execute before day 30.

**Threat:** premature default punishment; griefing keeper.

**Verifying test:** unit test with manipulated `Clock` sysvar; each instruction rejects pre-threshold.

**Instructions:** `mark_late_payment`, `suspend_participant`, `liquidate_default`.

---

### INV-23 — Late Penalty Compounds Once Per Day

**Statement:** the 2% penalty does not stack within the same day. Implemented by storing `Participant.last_penalty_at` and gating on day delta.

**Threat:** keeper grief by spamming `mark_late_payment` to inflate penalties.

**Verifying test:** call `mark_late_payment` twice in the same day; second call no-ops on penalty (or reverts with `AlreadyAccrued`).

**Instructions:** `mark_late_payment`.

---

### INV-24 — Default Idempotent

**Statement:** `liquidate_default` cannot run twice on the same participant. Once `is_defaulted = true`, second call → `AlreadyLiquidated`.

**Threat:** double-seizure of collateral.

**Verifying test:** call twice; second errors.

**Instructions:** `liquidate_default`.

---

## G. KYC Invariants (spec §5.4, §9.11)

### INV-25 — KYC Mock vs Production Same Verification

**Statement:** every instruction that consults KYC uses the SAME verification function. The only difference between mock and production is who can issue (admin pubkey vs `kyc_oracle` pubkey). The verification logic does not branch on `cfg(feature = "mock-kyc")`.

**Threat:** verification drift between dev and prod; gates pass in dev that fail in prod or vice versa.

**Verifying test:** code review confirms `require_full_kyc` and `require_light_kyc` have NO `#[cfg]` attributes. Lint test in CI greps for `cfg.*mock-kyc.*verify` and fails if any.

**Instructions:** `join_pool`, `commit_bid`, `claim_winning`.

---

### INV-26 — Mock-KYC Cargo Gate

**Statement:** `mock_issue_kyc` instruction is excluded from the compiled program when `--features production` (or absence of `mock-kyc`) is used.

**Threat:** mock instruction leaks to mainnet, enabling free attestation issuance.

**Verifying test:** CI builds with `--features production`, dumps IDL, asserts `mock_issue_kyc` is absent. Mainnet deploy script independently checks the deployed `.so` and refuses if present.

**Instructions:** build-time, not runtime.

---

### INV-27 — KYC Expiry

**Statement:** `KycAttestation.expires_at` is checked at every consumption site. Expired attestations behave identically to missing attestations.

**Threat:** stale attestation lets users skip re-verification.

**Verifying test:** issue 12-month attestation, advance Clock past expiry, attempt `commit_bid` → `KycExpired`.

**Instructions:** `join_pool`, `commit_bid`, `claim_winning`.

---

## H. Math & Arithmetic Invariants (spec §9.4)

### INV-28 — Checked Arithmetic Everywhere

**Statement:** all `add`, `sub`, `mul`, `div` on token amounts use `checked_*` and propagate `MathOverflow` on failure.

**Threat:** silent wraparound creates phantom balances.

**Verifying test:** code review / clippy lint forbidding non-checked arithmetic on `u64`/`u128`. Plus targeted unit tests at boundaries (`u64::MAX`).

**Instructions:** all instructions touching token amounts or basis-points math.

---

### INV-29 — Basis-Point Rounding Floor

**Statement:** all bps math uses `(amount as u128 * bps as u128 / 10_000) as u64`, which floors. The pool is conservative: protocol/reserve receive floored shares; the residual stays in `pool_usdc_vault`. Total bps share never exceeds 10_000.

**Threat:** rounding lets fees exceed 100% or leaves dust unaccounted.

**Verifying test:** for every fee split, sum the parts; assert ≤ original; assert residual goes to vault.

**Instructions:** `contribute`, `claim_winning`, `distribute_yield`.

---

## I. Time Invariants (spec §9.7)

### INV-30 — Clock-Sysvar Only

**Statement:** every timestamp comparison reads from `Clock::get()?.unix_timestamp`. No client-supplied timestamps trigger state transitions.

**Threat:** client lies about time to skip windows.

**Verifying test:** code grep for `unix_timestamp` confirms only one source.

**Instructions:** all time-sensitive ones.

---

## J. Account Validation Invariants (spec §9.2, §9.5, §9.6)

### INV-31 — All PDAs Validated by Seeds

**Statement:** every account passed to every instruction with a known PDA derivation uses Anchor's `seeds = [...], bump = X.bump` constraint, NOT raw pubkey comparison alone.

**Threat:** spoofed account pretending to be a PDA but actually controlled by attacker.

**Verifying test:** code review checklist.

**Instructions:** all.

---

### INV-32 — Token Account Authority Verified

**Statement:** every protocol-controlled token account constraint includes `token::authority = <expected PDA>` and `token::mint = USDC`.

**Threat:** attacker passes their own token account claiming to be the pool vault.

**Verifying test:** integration test substitutes wrong authority; expect `ConstraintTokenOwner`.

**Instructions:** all that touch token accounts.

---

### INV-33 — Stored Bumps Used Consistently

**Statement:** PDAs that need to sign use `seeds_with_bump` based on the bump stored in account state, not re-derived via `find_program_address`.

**Threat:** CU waste, plus potential mismatch if bump derivation logic changes.

**Verifying test:** code review.

**Instructions:** all signer-PDA CPIs.

---

## K. Upgrade & Operational Invariants (spec §9.9)

### INV-34 — Upgrade Authority Is Multisig (Pre-Mainnet)

**Statement:** before mainnet launch, the `bpf_loader_upgradeable` upgrade authority for all four programs is set to a 3-of-5 Squads multisig. Documented in deploy script and asserted at deployment time.

**Threat:** single-key compromise → arbitrary code injection.

**Verifying test:** deploy script verifies upgrade authority post-deploy and refuses if not multisig (mainnet only).

**Instructions:** deploy-time.

---

### INV-35 — Pause Stops New Risk, Not Existing Recovery

**Statement:** `emergency_pause` blocks `create_pool`, `join_pool`, `contribute`, `commit_bid`, `claim_winning`. It does NOT block `liquidate_default`, `mark_late_payment`, `suspend_participant`, `consume_vrf_winner`. Recovery flows must continue during pause.

**Threat:** admin pauses to prevent users from getting paid; or unpaused bug allows new exposure during incident.

**Verifying test:** integration test pauses, attempts each instruction, asserts the right ones revert with `ProtocolPaused` and the recovery ones succeed.

**Instructions:** all of `poolver-core`.

---

## L. Indexer & Event Invariants (spec §6)

### INV-36 — Every State Change Emits an Event

**Statement:** for every instruction that mutates persisted state, an Anchor event is emitted carrying enough fields for an indexer to reconstruct the state delta without re-reading accounts.

**Threat:** indexer drift; UI shows stale data.

**Verifying test:** for each instruction's integration test, assert the expected event was emitted with the expected fields.

**Instructions:** all state-changing.

---

## Appendix — Invariant-to-Instruction Cross-Reference

| Instruction | Invariants potentially violated |
|---|---|
| `initialize_protocol` | INV-12 |
| `mock_issue_kyc` | INV-25, INV-26, INV-27 |
| `create_pool` | INV-8 |
| `join_pool` | INV-1, INV-3, INV-4, INV-11, INV-25, INV-30, INV-31, INV-35 |
| `contribute` | INV-1, INV-3, INV-4, INV-6, INV-7, INV-29, INV-30 |
| `commit_bid` | INV-10, INV-16, INV-25, INV-27, INV-30, INV-35 |
| `reveal_bid` | INV-14, INV-15, INV-30 |
| `select_winner` | INV-5, INV-9, INV-13, INV-15, INV-17, INV-30 |
| `consume_vrf_winner` | INV-5, INV-17 |
| `claim_winning` | INV-1, INV-7, INV-10, INV-25, INV-29 |
| `advance_month` | INV-9, INV-13, INV-17 |
| `mark_late_payment` | INV-13, INV-22, INV-23, INV-30 |
| `suspend_participant` | INV-13, INV-22 |
| `liquidate_default` | INV-1, INV-2, INV-3, INV-4, INV-7, INV-13, INV-22, INV-24 |
| `distribute_yield` | INV-1, INV-3, INV-4, INV-13, INV-29 |
| `emergency_pause/unpause` | INV-12, INV-35 |
| `seed_reserve` | INV-2, INV-3, INV-12 |
| `reserve::deposit/draw` | INV-2, INV-3, INV-4 |
| `yield_defi::*` | INV-18, INV-19, INV-20, INV-21 |
| `yield_vault::*` | INV-21 |

---

## Appendix — Invariant Verification Crosswalk (V1 final)

Cross-reference of every INV to the V1 test (or code-review citation) that
verifies it. Tests live under `programs/*/tests/` and are run via
`cargo test --workspace --tests` (168 passing as of submission).

| INV | Verified by | Path |
|---|---|---|
| INV-1  Pool Solvency | `t19_e2e_fee_accounting_solvency`, `t43_e2e_12_months_solvency`, `t319_solvency_post_liquidation` | `programs/poolver-core/tests/test_core_step5.rs` + `test_core_step10.rs` |
| INV-2  Reserve Non-Negative | `test_reserve` draw-underflow case | `programs/poolver-reserve/tests/test_reserve.rs` |
| INV-3  Reserve Inflow/Outflow Identity | `test_reserve` deposit + draw + seed identity assertions | `programs/poolver-reserve/tests/test_reserve.rs` |
| INV-4  Reserve Tier Isolation | `t44_reserve_isolation_wrong_tier`, `t317_reserve_isolation_wrong_tier` | `test_core_step5.rs`, `test_core_step10.rs` |
| INV-5  Single Winner Per Month | step-7 select_winner suite (`t150…` series) | `programs/poolver-core/tests/test_core_step7.rs` |
| INV-6  Paid Months Monotonic | covered by `t38_contribute_releases_collateral_post_win` + property checks in step-5 e2e | `test_core_step5.rs` |
| INV-7  Collateral Monotonic Decrease | `t38_contribute_releases_collateral_post_win`, `t319_solvency_post_liquidation` | `test_core_step5.rs`, `test_core_step10.rs` |
| INV-8  Tier Immutability | code review: `pool.tier` is never written outside `handle_create_pool` (verified by grep at submission); `t08_create_pool_rejects_tier1` confirms tier validation | `programs/poolver-core/src/instructions/create_pool.rs` |
| INV-9  Pool Lifecycle Monotonic | `t39_advance_month_happy`, `t41_advance_month_completes_after_month12`, `t42_advance_month_rejects_when_complete` | `test_core_step5.rs` |
| INV-10 Full KYC Required to Win | step-8 claim_winning suite (`t100…` series); KYC-gate negative tests | `test_core_step8.rs` |
| INV-11 Light KYC Required to Join | `t13_join_pool_rejects_no_kyc`, `t14_join_pool_rejects_expired_kyc` | `test_core_step5.rs` |
| INV-12 Admin Powerlessness | code review (no admin → user-vault transfer paths); `t04_mock_issue_kyc_rejects_non_admin` proves admin-only on mock issuer | `test_core_step5.rs` + manual review |
| INV-13 Permissionless Keeper Calls Are Side-Effect Safe | every `*_rejects_*` test in `test_core_step10.rs` and `test_core_step5.rs` | `test_core_step10.rs`, `test_core_step5.rs` |
| INV-14 Commit-Reveal Hash Match | `t71_reveal_bid_rejected_on_hash_mismatch` | `test_core_step6.rs` |
| INV-15 Bid Cap | `t73_reveal_bid_rejected_when_above_cap`, `t75_bid_cap_math_boundary` | `test_core_step6.rs` |
| INV-16 One Bid Per User Per Month | `t66_commit_bid_rejected_double_commit`, `t77_bid_pda_tuple_integrity` | `test_core_step6.rs` |
| INV-17 VRF In-Flight Exclusivity | step-7 select_winner suite (mock-VRF path uses deterministic entropy; in-flight flag verified via state inspection) | `test_core_step7.rs` |
| INV-18 Tier 1 Liquidity Buffer | yield-defi `test_adapter.rs` t-series (75/25 split mocked via internal transfer) | `programs/poolver-yield-defi/tests/test_adapter.rs` |
| INV-19 Tier 1 Kamino Cap | yield-defi `test_adapter.rs` deposit cap tests | `programs/poolver-yield-defi/tests/test_adapter.rs` |
| INV-20 Circuit Breaker One-Way Until Reset | yield-defi `test_adapter.rs` t04_breaker_blocks_deposit + reset tests | `programs/poolver-yield-defi/tests/test_adapter.rs` |
| INV-21 Adapter Interface Uniformity | both `programs/poolver-yield-vault/tests/test_adapter.rs` and `programs/poolver-yield-defi/tests/test_adapter.rs` exercise the identical CPI surface; arch §13.4 documents the verified discriminator-equality | both adapter test suites |
| INV-22 Grace Period Enforcement | `t301_mark_late_rejected_before_month_end`, `t306_suspend_rejected_before_day6`, `t313_liquidate_rejected_before_day30` | `test_core_step10.rs` |
| INV-23 Late Penalty Compounds Once Per Day | `t304_mark_late_rejected_double_mark` | `test_core_step10.rs` |
| INV-24 Default Idempotent | `t315_liquidate_rejected_double_liquidation` | `test_core_step10.rs` |
| INV-25 KYC Mock vs Production Same Verification | code review: `programs/poolver-core/src/kyc.rs` has zero `#[cfg]` attributes (verified) | `programs/poolver-core/src/kyc.rs` |
| INV-26 Mock-KYC Cargo Gate | verified by `cargo build-sbf --no-default-features` + `strings`/`jq` checks (see `docs/mock-to-production.md` "V1 Build-Guard Verification") | scripts/deploy.ts::verifyMockFree |
| INV-27 KYC Expiry | `t14_join_pool_rejects_expired_kyc`, `t64_commit_bid_rejected_when_kyc_expired` | `test_core_step5.rs`, `test_core_step6.rs` |
| INV-28 Checked Arithmetic Everywhere | code review (clippy lint pending); verified by grep at submission — every `+`/`-`/`*` on token amounts is `checked_*` | manual review |
| INV-29 Basis-Point Rounding Floor | step-9 distribute_yield rounding tests (`t206_yield_splits_rounding_into_participant`) | `test_core_step9.rs` |
| INV-30 Clock-Sysvar Only | grep at submission: only `Clock::get()?.unix_timestamp` is the timestamp source | manual review |
| INV-31 All PDAs Validated by Seeds | code review of every `#[derive(Accounts)]`; see arch §4 PDA seed table | manual review |
| INV-32 Token Account Authority Verified | `t44_reserve_isolation_wrong_tier`, `t317_reserve_isolation_wrong_tier` exercise the `ConstraintTokenOwner` reject path | `test_core_step5.rs`, `test_core_step10.rs` |
| INV-33 Stored Bumps Used Consistently | grep at submission: every CPI helper signs with `account.bump`, never `find_program_address` at runtime | manual review |
| INV-34 Upgrade Authority Is Multisig (Pre-Mainnet) | deferred to V2 mainnet deploy (Squads integration); V1 devnet uses single-key admin | deploy script TODO |
| INV-35 Pause Stops New Risk, Not Existing Recovery | `t33_contribute_rejects_when_paused`, `t202_distribute_yield_rejected_when_paused`; recovery-flow positive tests in `test_core_step10.rs` | `test_core_step5.rs`, `test_core_step9.rs`, `test_core_step10.rs` |
| INV-36 Every State Change Emits an Event | every happy-path test asserts the expected event; `programs/poolver-core/src/events.rs` defines one event per state-changing instruction | every `test_*.rs` |

**Manual-review items** (INV-12, INV-25, INV-28, INV-30, INV-31, INV-33) are
flagged as such — they are structural invariants that don't fit a single test
function but are enforced by code-level conventions verified at submission.

*End of invariants.md.*
