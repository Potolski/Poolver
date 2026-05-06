# Poolver V1 — Open Questions

> Surfaced by `solana-architect` during architecture review of `docs/SPEC_V1.md`.
> Resolve before mainnet deployment. Each entry cites the spec section and proposes a default if no human resolution arrives.
>
> **Format:** `// SPEC_QUESTION: <id> — <description>` is the marker engineers should write in code where the question is unresolved.

---

## V1 BLANKET RESOLUTION (2026-05-05)

**Decision (David):** for the Colosseum 2026 V1 hackathon submission, **every question below is resolved as the architect's chosen default** — i.e., the simplest path that satisfies the spec invariants.

If Poolver advances past Colosseum, each question will be revisited individually with a proper resolution. Until then, implementers should:

- Treat the **"Architecture default chosen"** line on each question as the binding V1 decision.
- Still write `// SPEC_QUESTION-<id>:` markers at every code site where the choice is load-bearing — this lets a future engineer grep them in one pass when revisiting.
- Not silently invent a third path. If the architect's default appears wrong during implementation, surface it; don't drift.

This banner does **not** delete the question text below — the analysis remains the authoritative reference for V2 work.

---

## A. Spec Ambiguity

### SPEC_QUESTION-1 — Bid distribution: per-recipient transfer vs pooled credit
**Spec ref:** §5.1 `claim_winning` ("75% to remaining participants — credited as reduction in their next contribution") vs §6 events ("`BidDistributed` (one per recipient discount, or one summary event)").
**Question:** is the 75% bid share (a) credited to a per-participant ledger field that they redeem on next `contribute`, or (b) deposited into a pool-wide `bid_credit_balance` that's divided pro-rata at the moment of the next `contribute`?
**Architecture default chosen:** (b) — pool-wide `bid_credit_balance`, divided by N remaining participants at time of each subsequent `contribute`. Reduces `claim_winning` from N writes to 1 write, saving ~120k CU and avoiding any 11-account loop.
**Implication if wrong:** if spec author intends per-participant ledger, change `Participant.pending_credit` accumulation logic in `claim_winning` instead of `contribute`. Same end-state, different write timing.

### SPEC_QUESTION-2 — Tie-breaking among bids
**Spec ref:** §5.1 `select_winner` "ties broken by VRF".
**Question:** is VRF strictly required for ties, or is a deterministic hash-based tiebreaker (sha256(pool || month || bidder)) acceptable?
**Architecture default chosen:** deterministic hash. Avoids forcing every contested month into async VRF flow. Probability of tie at u64 USDC granularity in a 12-bidder pool with 20% bid cap is statistically negligible (<<1%).
**Implication if wrong:** add VRF request branch when `top_bid_count > 1`. Would require `Pool.vrf_in_flight` to be set in this branch too.

### SPEC_QUESTION-3 — Bid stake on no-reveal
**Spec ref:** §11 ("Slashing of bidder stake on no-reveal: just forfeit the small anti-spam stake") and §15.
**Question:** where does forfeited stake go — protocol fee vault, tier reserve, or pool vault?
**Architecture default chosen:** tier reserve (consistent with the pattern that punitive captures fund risk-mutualization).
**Implication if wrong:** trivial change in one place.

### SPEC_QUESTION-4 — Reveal window length
**Spec ref:** §3 lists `bid_window_seconds` (commit window) but not a reveal window.
**Question:** how long does the reveal window last?
**Architecture default chosen:** 24 hours (`reveal_window_seconds = 86_400`), tracked in `Pool.reveal_window_ends_at`.
**Implication if wrong:** adjust constant; no architectural change.

### SPEC_QUESTION-5 — Claim window for winner
**Spec ref:** §5.1 `advance_month` ("when current month duration has elapsed and winner has claimed (or claim window expired)").
**Question:** how long does the winner have to claim before forfeit, and what happens to the unclaimed pot?
**Architecture default chosen:** 24h claim window. If unclaimed: month winner forfeit; lottery re-run via VRF among remaining. Add `consume_unclaimed_winner` instruction.
**Implication if wrong:** non-trivial logic change.

### SPEC_QUESTION-6 — Late penalty distribution
**Spec ref:** §4 ("200 bps (2%) penalty accrues") — accrues where?
**Question:** is the penalty added to the participant's next contribution due, or split between protocol/reserve?
**Architecture default chosen:** added to participant's next `contribute` amount and routed to `bid_credit_balance` for remaining participants (penalty makes other participants whole for the keeper-bot work and risk).
**Implication if wrong:** trivial constant change.

### SPEC_QUESTION-7 — Reputation snapshot timing
**Spec ref:** §3 Participant `completed_cycles_at_join` and §4 reputation multiplier.
**Question:** does the multiplier use the snapshot at join, or the live count at win-time? Spec says snapshot, but ambiguous if user completes another pool concurrently.
**Architecture default chosen:** snapshot at join. Live updates do NOT propagate retroactively into already-active pools.
**Implication if wrong:** change collateral calc to read `UserReputation` directly at `claim_winning`.

### SPEC_QUESTION-8 — `participants` as Vec or fixed array
**Spec ref:** §3 ("`participants`: Vec<Pubkey>, length 0..=12").
**Question:** is this literally a `Vec` or a fixed-size structure?
**Architecture default chosen:** fixed-size `[Option<Pubkey>; 12]` (rationale in `docs/architecture.md` §7). The spec says `Vec` but the size is bounded and known.
**Implication if wrong:** trivial change to use `Vec` with `init_space` derived from MAX_LEN.

### SPEC_QUESTION-9 — Bid amount granularity
**Spec ref:** §4 bid cap "20% of monthly pot", USDC has 6 decimals.
**Question:** is bid amount allowed at 1-microUSDC granularity or rounded to whole USDC?
**Architecture default chosen:** 1-microUSDC granularity (no rounding).
**Implication if wrong:** add rounding in `reveal_bid`.

### SPEC_QUESTION-10 — Contribution vs reserve fee timing
**Spec ref:** §4 "Reserve contribution by tier: Vault 150 bps, DeFi 250 bps".
**Question:** is the reserve fee a deduction from the participant's contribution (so net into pool is contrib − fees), or an additional charge on top (participant pays contrib + fees)?
**Architecture default chosen:** deduction (matches typical DeFi pattern — quoted contribution is gross, net into vault is post-fee). This means the "monthly pot" is `12 × (contrib − total_fees)`, NOT `12 × contrib`.
**Implication if wrong:** redefine pot as `12 × contrib` and have user transfer `contrib + fees`. Affects bid cap math (which is % of pot).
**HIGH PRIORITY:** this affects every other calculation. Resolve first.

### SPEC_QUESTION-11 — Mid-pool reputation update on default
**Spec ref:** §5.1 `liquidate_default` ("UserReputation.pools_defaulted++").
**Question:** does default in pool A affect ongoing eligibility in pool B (where the user is mid-cycle)?
**Architecture default chosen:** suspends user from new joins/bids globally, but does NOT yank them from existing pools (that would create cascading liquidations).
**Implication if wrong:** add cross-pool default propagation logic — major cross-cutting change.

---

## B. Solana Best-Practice Flags

### SPEC_QUESTION-12 — `init_if_needed` ban vs adapter init
**Spec ref:** §9.10 ("No `init_if_needed`").
**Question:** how does a user join a pool whose adapter state may or may not exist? `create_pool` initializes the adapter; subsequent joins should not. We use explicit `init` in `create_pool` and `Account` (read) in `join_pool`. Confirms compliance.
**Architecture decision:** comply with the ban. Adapter is initialized exactly once in `create_pool`. Documented.

### SPEC_QUESTION-13 — Anchor vs `Pinocchio` for performance
**Spec ref:** §2 "Framework: Anchor".
**Question:** spec mandates Anchor. None of our hot paths exceed budget per CU analysis (architecture.md §6), so no performance-driven case for Pinocchio. Keep Anchor.
**Architecture decision:** Anchor only.

### SPEC_QUESTION-14 — Single multi-program transaction limits
**Spec ref:** none directly.
**Question:** `claim_winning` triggers reserve CPI + adapter CPI + token transfers. Anchor CPI depth limit is 4. We are at depth 2. Safe.
**Architecture decision:** confirmed safe. Documented in architecture.md §6.

### SPEC_QUESTION-15 — Stack frame budget for `Pool` size
**Spec ref:** §3 Pool fields.
**Question:** Solana BPF stack is 4KB per frame. Loading `Pool` (~1965 bytes) into a stack-allocated copy on entry could pressure the frame.
**Architecture decision:** use `Box<Account<'info, Pool>>` for any handler that also has other large stack locals. Document in code style guide.

### SPEC_QUESTION-16 — ALT for `select_winner`
**Spec ref:** none.
**Question:** the bid-path `select_winner` with all 12 bids needs ALT for safe tx-size. Spec doesn't address ALT setup.
**Architecture decision:** keeper bot creates and warms a per-pool ALT containing the 12 candidate Bid PDAs and shared accounts. Document in client SDK and keeper bot spec.

### SPEC_QUESTION-17 — Event size budget
**Spec ref:** §6 events.
**Question:** Anchor events are emitted via CPI to a self-program; large events (e.g., per-recipient `BidDistributed` for 11 recipients) consume CU.
**Architecture decision:** emit one summary event with arrays, not one per recipient. Indexer expansion is cheap.

### SPEC_QUESTION-18 — Reentrancy guard for adapter CPIs
**Spec ref:** §9.1.
**Question:** Solana programs are not reentrant by default but the spec flags the concern. Our adapters are first-party; we trust them. For Tier 1, Kamino is third-party — does it call back into our adapter? No, Kamino's surface is request/response. Reentrancy not a concern.
**Architecture decision:** no explicit guard; document the trust assumption.

---

## C. External Dependency Unknowns

### SPEC_QUESTION-19 — Kamino program ID and SDK
**Spec ref:** §5.3 ("Read Kamino program ID and account layout from their official docs").
**Question:** What is the exact Kamino Lend program ID on mainnet and devnet? Is there a published Rust SDK or do we hand-roll the CPI?
**Architecture default:** treat Kamino integration as `// SPEC_QUESTION-19` stub with a feature-gated mock for V1 hackathon submission. The `poolver-yield-defi` program builds with a `kamino-mock` feature for tests, real Kamino integration deferred.
**Action:** investigate Kamino's GitHub (`Kamino-Finance/klend`); document program ID, reserve account layout, and supply/redeem instruction discriminators in `docs/kamino-integration.md` once known.

### SPEC_QUESTION-20 — Kamino on devnet
**Spec ref:** §2 "develop on devnet and fork-mainnet".
**Question:** Does Kamino have a devnet deployment, or do we test only via fork-mainnet?
**Architecture default:** assume no devnet deployment; develop adapter against a local mock that mimics Kamino's instruction surface; verify on fork-mainnet for the final pass.
**Action:** confirm.

### SPEC_QUESTION-21 — Switchboard VRF on devnet
**Spec ref:** §2.
**Question:** Switchboard On-Demand VRF requires (a) an oracle queue account, (b) a function account, (c) a callback instruction. Devnet queue addresses?
**Architecture default:** use Switchboard On-Demand (newer, simpler than legacy V2). Devnet queue: `5JZ6kYnWBBcPm6mLmKaUnxwLLDS8XThbi8KPE6PcLp4t` (legacy; verify current).
**Action:** verify with Switchboard docs at `https://docs.switchboard.xyz` before integration.

### SPEC_QUESTION-22 — USDC mint on devnet
**Spec ref:** §2 ("USDC SPL token (`EPjFWdd5...` on mainnet) only").
**Question:** What USDC mint to use on devnet for testing?
**Architecture default:** Circle's official devnet USDC mint: `4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU` (verify currency). Faucet: `https://faucet.circle.com`.
**Action:** confirm and document in `tests/common/usdc.ts`.

### SPEC_QUESTION-23 — Pyth/oracle for Tier 1 deviation check
**Spec ref:** §4 ("Oracle deviation from expected > 200 bps").
**Question:** Which oracle? USDC/USD price (Pyth)? Kamino's internal exchange-rate oracle?
**Architecture default:** assume USDC/USD via Pyth (`Gnt27xtC473ZT2Mw5u8wZ68Z3gULkSTb5DuxJy7eJotD` mainnet). On 200bps deviation from $1.00, trip circuit breaker.
**Action:** confirm scope of "expected" — is it USDC peg or kToken/USDC ratio?

### SPEC_QUESTION-24 — `kyc_oracle` upgrade path
**Spec ref:** §5.4 production migration.
**Question:** When migrating from V1 mock to real KYC, can `protocol_config.kyc_oracle` be rotated by admin instruction, or does it require a program upgrade?
**Architecture default:** add `rotate_kyc_oracle` admin instruction now (admin-only, signer = current admin); set `kyc_oracle = admin` initially. Production migration is a single tx, no upgrade needed.
**Action:** confirm.

### SPEC_QUESTION-25 — Multisig vendor and threshold
**Spec ref:** §9.9 ("multisig (Squads 3-of-5)").
**Question:** Squads V4 SDK on mainnet requires the multisig PDA be created and funded before deploy. Out-of-scope for code, but block deploy until set up.
**Action:** before mainnet deploy, set up Squads multisig and document the PDA.

### SPEC_QUESTION-26 — Frontend program rebuild blast radius
**Spec ref:** outside spec.
**Question:** `app/src/lib/program.ts` and `app/src/lib/idl/` reference the legacy `poolver` program. The new four-program structure breaks every frontend call. The user has accepted this. Sequencing: rebuild frontend after `poolver-core` skeleton compiles.
**Action:** plan a `app/src/lib/program-v2.ts` or replace in place once new IDLs exist.

### SPEC_QUESTION-27 — Hackathon submission & mainnet readiness divergence
**Spec ref:** outside spec.
**Question:** Colosseum 2026 is in ~1 week. Mainnet-quality production guards (multisig, real KYC, Kamino integration) are not feasible by deadline.
**Architecture default:** ship to devnet for hackathon. Build with `--features mock-kyc` and `kamino-mock`. Mainnet build path documented but not exercised. Demo runs Tier 0 end-to-end and Tier 1 with mock adapter.
**Action:** confirm with David that hackathon submission target is devnet, not mainnet.

### SPEC_QUESTION-28 — Anchor 0.30+ vs 0.31+ feature surface
**Spec ref:** §2 ("Anchor latest stable, 0.30+").
**Question:** Existing program uses `anchor-lang = "1.0.0"` (which is the new versioning scheme; effectively 0.31+). Choose explicit version for new programs.
**Architecture default:** pin to `anchor-lang = "0.31.1"` (or whatever latest is at start of implementation; verify against Anchor changelog).
**Action:** confirm at implementation start.

---

## D. Process

### SPEC_QUESTION-29 — Audit timing
**Spec ref:** §1 ("auditable by Ottersec, Neodyme, Offside Labs").
**Question:** is an external audit gating mainnet, or is internal review sufficient given the V1 mock-KYC and small TVL target?
**Architecture default:** internal review for hackathon; external audit before mainnet TVL > $50k.

### SPEC_QUESTION-30 — VRF cost
**Spec ref:** §15 ("Switchboard VRF pricing").
**Question:** at what cost-per-callback does VRF become uneconomic, and what's the fallback?
**Architecture default:** Switchboard On-Demand: ~0.001 SOL per request. Pool pays from `Pool.bid_credit_balance` or protocol fee vault. Document in fee model.

---

## D. Surfaced during step-4 implementation (poolver-core skeleton)

### SPEC_QUESTION-31 — `Pool` `_reserved` padding trimmed for BPF stack budget
**Spec ref:** docs/architecture.md §3.2 (Pool layout).
**Background:** Architecture sets `Pool._reserved = [u8; 128]` and `MonthWinner._reserved = [u8; 32]`. With `Box<Account<'info, Pool>>` plus 17 sibling accounts in `JoinPool`, Anchor 1.0's generated `try_accounts` overflowed the 4 KB-per-frame BPF stack at runtime ("Access violation in stack frame 5"). Trimming reserve sizes to `Pool._reserved = [u8; 16]` and `MonthWinner._reserved = [u8; 8]` brought the frame under budget.
**Decision (V1):** kept reduced sizes. `Pool` total drops from arch's nominal ~1965 bytes to ~1726 bytes; still well under 10 MB account-size limit. Forward-compatibility headroom preserved (16 + 12×8 = 112 reserved bytes total).
**Implication if wrong:** if a future field needs >16 bytes, the chosen path is realloc on a versioned-migration instruction (we already have `Pool.version: u8`), not enlarging the pre-allocated reserve. Document in the Pool struct comments.
**Status:** resolved as architect default for V1. Re-evaluate when adding fields in V2.

### SPEC_QUESTION-32 — Manual deserialization of `protocol_config` and `user_kyc` in `join_pool`
**Spec ref:** docs/architecture.md §3.1, §3.7; docs/SPEC_V1.md §9.5.
**Background:** Same BPF stack-budget issue as Q-31. `JoinPool`'s account-validation frame remained over-budget even after padding trims. Worked around by converting `protocol_config` and `user_kyc` to `UncheckedAccount<'info>` (with `seeds = [...]` constraints intact for PDA binding) and deserializing manually inside the handler — including ownership and discriminator checks via `Account::try_deserialize`.
**Decision (V1):** keep the manual-deser pattern. Security is identical: Anchor's `Account<'info, T>` does owner+discriminator+bump checks; the handler now does owner check via `require_keys_eq!(*acct.owner, crate::ID, ...)` and discriminator check via `try_deserialize`. PDA seed binding remains in the `Accounts` macro.
**Implication if wrong:** auditor may want explicit owner constraint in the macro. If so, switch back to `Account<'info, T>` and instead split JoinPool into multiple sub-account-structs called via `remaining_accounts`.
**Status:** resolved as architect default for V1.

### SPEC_QUESTION-33 — Production deploy guard via IDL grep is fragile
**Spec ref:** docs/architecture.md §10.4 (deploy script guard).
**Background:** Architecture proposed `grep -q "mock_issue_kyc" target/idl/poolver_core.json` as the mainnet deploy guard. The literal string `mock_issue_kyc` appears in the IDL even with the feature disabled — inside docstrings (e.g., the `KycAttestationIssued.is_mock` field comment). A naive grep would refuse to deploy a correctly-built production binary.
**Decision (V1):** the guard MUST extract the `instructions[]` list with `jq` (or equivalent) and check that no entry has `name == "mock_issue_kyc"`, not a free-text grep. Concretely:
```bash
if jq -e '.instructions | map(.name) | index("mock_issue_kyc")' target/idl/poolver_core.json >/dev/null; then
  echo "REFUSING: mock_issue_kyc instruction is enabled"; exit 1
fi
```
Update arch §10.4 to this snippet. The .so symbol-table check (`nm $SO_FILE | grep mock_issue_kyc`) is also reliable and cheaper.
**Status:** introduced; arch §10.4 needs the doc update.

### SPEC_QUESTION-34 — Month-12 winner collateral edge case
**Spec ref:** docs/SPEC_V1.md §4 (collateral release schedule).
**Background:** spec §4 says `baseline = (total_months - win_month) × contribution_amount` and `total_collateral = baseline × rep_multiplier + bid_premium`. For a month-12 winner, `baseline = 0`, leaving only `bid_premium = winning_bid × 2`. The release schedule then divides `collateral_initial / months_remaining_at_win` — which is `bid_premium / 0` (division by zero), and there are no future contributions to enforce against anyway.
**Decision (V1):** at `claim_winning`, when `win_month == TOTAL_MONTHS` (=12), post the `bid_premium` collateral as required by spec §4 (so the SPL transfer chain stays uniform), then **immediately refund** it back to the winner inside the same instruction. Net result: `participant.collateral_locked = 0`, `participant.collateral_initial = bid_premium` (preserved for indexer history), `collateral_release_per_month = bid_premium` (moot since locked is already 0). No future `contribute` ever fires the release branch for a month-12 winner.
**Implication if wrong:** if the spec actually wanted the bid_premium to remain locked through some post-cycle settlement, this refund would be premature. We chose the simpler-and-still-solvent route; revisit when finalize_pool / step 11 ships if there's a structural reason to keep the lock.
**Status:** resolved as architect default for V1. See `programs/poolver-core/src/instructions/claim_winning.rs` step-8 implementation.

### SPEC_QUESTION-35 — Default cascade timing constants and reputation gate threshold
**Spec ref:** docs/SPEC_V1.md §4 (default cascade) + §5.1 `liquidate_default` + Q-11.
**Background:** step 10 implements the `mark_late` (day 1..=5) → `suspend` (day 6+) → `liquidate` (day 30+) cascade. Two design choices were not fully nailed down by the spec:

1. **Cure-window for `contribute`.** Spec §5.1 doesn't say whether a participant who's been suspended at day 6 can still cure by paying. We accept contributions across the entire `[month_start, month_end + 30 days)` window, and clear `is_late`/`is_suspended` on successful payment. The hard cutoff is the same threshold that unlocks `liquidate_default`. Once liquidation is allowed, the cure path closes.

2. **Reputation gate threshold for `join_pool`.** Q-11 resolved that `pools_defaulted++` blocks "new joins/bids globally." V1 enforces the strictest version: ANY non-zero `pools_defaulted` blocks new joins (`require!(user_reputation.pools_defaulted == 0)`). Production may relax to e.g. `<= 1` after observability data accumulates.

**Decision (V1):** both choices kept as-is. `LATE_PENALTY_BPS = 200`, `GRACE_PERIOD_SECS = 5 days`, `SUSPENSION_THRESHOLD_SECS = 6 days`, `LIQUIDATION_THRESHOLD_SECS = 30 days` are all centralized in `programs/poolver-core/src/constants.rs` for easy tuning. The cure-window relaxation is documented inline in `contribute.rs`.

**Implication if wrong:** if the spec wanted cure-after-suspension blocked, swap the `is_suspended` check from a soft (clears on cure) to a hard reject in `contribute`. If Q-11's "globally" was meant looser than zero-defaults-allowed, change the join_pool gate threshold.

**Status:** resolved as architect defaults for V1.

### SPEC_QUESTION-36 — Tier-1 dispatch wiring inside poolver-core
**Spec ref:** spec §5.1 (`create_pool` / `contribute` / `claim_winning` / `distribute_yield`) + arch §13 (common adapter interface).
**Background:** step 12 shipped `poolver-yield-defi` with a byte-identical instruction surface to `poolver-yield-vault` so core can dispatch on `pool.tier`. The adapter is buildable, fully unit-tested (18 tests, fake-core stub), and integrates the Tier-1 features (75/25 split, circuit breakers, latched-tripped state). Wiring the core side — branching the four (in practice five — `join_pool` also CPIs deposit) CPI sites by `tier` — is non-trivial because Anchor's `try_accounts` already pushes the contribute / claim_winning contexts close to the 4 KB BPF stack budget (SPEC_QUESTION-15). Two viable routes:
  (a) Add both adapter-program account triples (vault_state+vault_usdc + defi_state+defi_usdc+defi_ktoken) to each context and use the matching one in the handler. Increases tx size and `try_accounts` stack pressure.
  (b) Pass tier-specific accounts through `remaining_accounts` and dispatch in the handler.

**V1 decision:** route (b). The dispatch lives in `programs/poolver-core/src/adapter_cpi/adapter.rs`; per-instruction `cpi_adapter_*` helpers were collapsed into a single tier-aware family (`cpi_adapter_initialize`, `cpi_adapter_deposit`, `cpi_adapter_withdraw`, `cpi_adapter_harvest`). Each helper takes the leading byte-identical fixed accounts plus a `&[AccountInfo]` slice for the Tier-1 surplus (`adapter_ktoken_vault` at index 0). Adapter program ID is validated against `pool.tier` in the helper (replaces the dropped Anchor `address = ...` constraint).

The `address = poolver_yield_vault::ID` constraints on `yield_vault_program` were dropped from `create_pool`, `join_pool`, `contribute`, `claim_winning`, and `distribute_yield`. The field was renamed to `yield_adapter_program` (UncheckedAccount) so callers can pass either canonical ID. The `seeds::program = poolver_yield_vault::ID` clauses on `distribute_yield`'s adapter accounts were also dropped; Anchor seeds binding moved to handler-side tier-aware re-derivation (defense-in-depth) plus the adapter program's own seed validation on its side.

**Status:** RESOLVED-36 (step 13, 2026-04-30). `Tier::DeFi` `create_pool` works end-to-end. Cross-tier rejection (Tier 0 pool with yield-defi program ID, and reciprocal) is structurally enforced via `require_adapter_program_for_tier`. Integration tests in `programs/poolver-core/tests/test_integration.rs`.

---

*End of QUESTIONS.md. Update as new questions arise during implementation. When resolving, change the heading to `RESOLVED-<id>` and append the decision and date.*
