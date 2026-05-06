<div align="center">

# Poolver V1

### On-chain rotating savings + credit (consórcio / ROSCA) protocol on Solana

[![Built on Solana](https://img.shields.io/badge/Built%20on-Solana-9945FF?style=for-the-badge&logo=solana&logoColor=white)](https://solana.com)
[![Anchor](https://img.shields.io/badge/Anchor-1.0.0-1E88E5?style=for-the-badge)](https://www.anchor-lang.com)
[![USDC](https://img.shields.io/badge/Settlement-USDC-2775CA?style=for-the-badge)](https://www.circle.com/usdc)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow?style=for-the-badge)](LICENSE)
[![Solana Colosseum 2026](https://img.shields.io/badge/Solana-Colosseum%202026-14F195?style=for-the-badge&logo=solana&logoColor=black)](https://colosseum.com)

[Architecture](docs/architecture.md) · [Invariants](docs/invariants.md) · [Mock-to-Production](docs/mock-to-production.md) · [Spec](docs/SPEC_V1.md) · [Open Questions](QUESTIONS.md)

</div>

---

## What it is

A **consórcio** (Brazilian ROSCA — Rotating Savings and Credit Association) lets a group of participants pool monthly contributions; each month one member receives the full pot to make a high-value purchase. The model moves $500B+ annually across Latin America, Africa, and Asia, but is gated by 10–20% intermediary fees, opaque "lottery" selection, and country-locked operators.

Poolver replaces the intermediary with four composable Solana programs. Pools are 12-participant / 12-month, denominated in USDC, with **commit-reveal sealed bids** (lance) for winner selection, **tier-segregated reserve funds** that absorb defaults, and a **pluggable yield adapter** that earns Kamino-style yield on idle balances. Protocol fees: **1.5%** (vs. 10–20% incumbent).

## Architecture

Four Anchor programs (see [`docs/architecture.md`](docs/architecture.md) for full diagrams):

| Program | Role | Program ID (devnet) |
|---|---|---|
| `poolver-core` | Pool lifecycle, bidding, default cascade, mock KYC | `2SsxJqMCYKCYesfzfXASgAPPz153j8tYMXpMKKmt2QXk` |
| `poolver-reserve` | Tier-segregated reserve fund custody | `CfxRT3jsXWQZRev67ztqaNKCrHaKF6ieW9a1E8NDPvnx` |
| `poolver-yield-vault` | Tier-0 adapter — passive USDC custody | `A3ERUDLAdqdwgqgAoYLftxA6F1QtxSHZYu8DpNDXyyUp` |
| `poolver-yield-defi` | Tier-1 adapter — Kamino integration (mocked in V1) | `DAitPF7KHzRDVWcV4XM3J7dYGrKJkH332dQHPYUiP7UP` |

Both adapters share a **byte-identical** instruction surface (`initialize_adapter`, `deposit`, `withdraw`, `harvest`, `emergency_unwind`) so `poolver-core` dispatches on `pool.tier` against a single CPI shape (arch §13). Tier-1 callers append a single `remaining_account` (the kToken vault) per arch §13 / Q-36.

## Build

```bash
# Toolchain: Rust 1.75+, Solana CLI 2.x, Anchor 1.0.0, Node 18+, Yarn
yarn install
anchor build
```

For the mainnet artifact path (no mock instructions in dispatch table or .so):

```bash
anchor build -- --no-default-features
```

## Test

```bash
# 168 tests across the workspace (Mollusk + LiteSVM + per-program suites)
cargo test --workspace --tests
```

Per-program quick filters:

```bash
cargo test -p poolver-core            # core lifecycle, bidding, default cascade
cargo test -p poolver-reserve         # reserve fund accounting (INV-2/3/4)
cargo test -p poolver-yield-vault     # Tier-0 adapter
cargo test -p poolver-yield-defi      # Tier-1 mock adapter + circuit breaker (INV-23)
```

## Deploy

```bash
# Devnet (no confirmation prompt)
npx tsx scripts/deploy.ts --cluster devnet

# Mainnet — operator must type the exact phrase "DEPLOY MAINNET POOLVER-V1"
# AND the build must be mock-free (jq IDL check + strings .so check)
npx tsx scripts/deploy.ts --cluster mainnet-beta

# Initialize singletons (idempotent)
npx tsx scripts/initialize.ts --cluster devnet --usdc-mint <USDC>

# Top up tier reserve
npx tsx scripts/seed-reserve.ts --tier vault --amount 10000 --cluster devnet
```

See [`scripts/README.md`](scripts/README.md) for the full deploy + safeguard documentation.

## Repo layout

```
.
├── programs/                    4 Anchor programs (see Architecture above)
│   ├── poolver-core/
│   ├── poolver-reserve/
│   ├── poolver-yield-vault/
│   └── poolver-yield-defi/
├── client/                      TypeScript SDK (@poolver/client)
│   ├── src/instructions/        one wrapper per ix
│   ├── src/queries/             account fetchers + decoders
│   ├── src/idls/                self-contained copies of IDLs + TS types
│   ├── src/pdas.ts              all PDA derivations
│   └── src/utils/bid_hash.ts    INV-14 commit-hash builder
├── scripts/                     deploy / initialize / seed-reserve
├── docs/
│   ├── architecture.md          component overview, account layouts, CPI matrix
│   ├── invariants.md            36 INV entries with verifying tests
│   ├── mock-to-production.md    every // MOCK_KYC: site + production migration
│   └── SPEC_V1.md               source of truth for divergence
├── archive/                     legacy V0 program (frontend still references)
├── app/                         Next.js frontend (currently broken vs new IDL — V2)
├── deployments/                 deploy.ts writes JSON receipts here
├── QUESTIONS.md                 36 spec questions, several RESOLVED
└── Anchor.toml
```

## V1 scope (what works)

- ✅ 4 programs build cleanly with `anchor build`
- ✅ Both tiers (Tier-0 vault + Tier-1 with mocked Kamino)
- ✅ Default cascade (`mark_late_payment` → `suspend_participant` → `liquidate_default`)
- ✅ Commit-reveal sealed bids with sha256 binding (INV-14)
- ✅ Tier-segregated reserve funds with structural isolation (INV-4)
- ✅ Bid credit ledger with pro-rata distribution (Q-1)
- ✅ Mock KYC (compile-time gated; production hook ready)
- ✅ Yield distribution with circuit breaker (INV-23)
- ✅ Full TypeScript SDK + deploy / initialize / seed scripts
- ✅ 168 passing tests across the workspace

## V2 roadmap (deferred per Q-19/20/14)

- ⏳ **Real Kamino integration** — replace `poolver-yield-defi` mock with live CPI
- ⏳ **Real KYC oracle** — Idwall/Sumsub off-chain integration; rotate `kyc_oracle` per spec §5.4
- ⏳ **Switchboard On-Demand VRF** — replace deterministic mock entropy in `select_winner`
- ⏳ **Fork-mainnet test harness** — required for Tier-1 once real Kamino lands
- ⏳ **Multisig admin** — move admin authority to Squads
- ⏳ **Frontend rebuild** — `app/` is broken since the V1 program rebuild; needs to be wired against the new SDK
- ⏳ **Address Lookup Table** — needed at full 12-bid `select_winner` (arch §8)

## Audit-readiness checklist

- [x] **Solvency invariant proof sketch** documented (arch §12, INV-1)
- [x] **36 invariants enumerated** with verifying tests (`docs/invariants.md`)
- [x] **All 14 production instructions tested** including reject paths
- [x] **Mock guards documented** with concrete jq + strings commands (`docs/mock-to-production.md` + `scripts/deploy.ts`)
- [x] **No `init_if_needed`** anywhere in the codebase (Q-12)
- [x] **No `unwrap()` / `expect()`** in program code (only in tests)
- [x] **All arithmetic is checked** (`checked_add` / `checked_sub` / `checked_mul`)
- [x] **All PDAs use stored canonical bumps** (Q-13)
- [x] **Tier isolation enforced structurally** via PDA seeds (INV-4)
- [x] **CPI auth via `core_invoker` PDA** with `seeds::program = poolver_core::ID` on every callee (arch §5.2)
- [x] **No admin-drain paths** — admin can pause but cannot move user funds
- [x] **Mock instructions excluded from mainnet builds** — verified via jq + strings on `--no-default-features` artifacts
- [ ] **External audit** — pending V2

## Links

- **Pitch deck**: `Poolver — Solana Colosseum 2026.pdf`
- **Spec V1**: [`docs/SPEC_V1.md`](docs/SPEC_V1.md)
- **Architecture**: [`docs/architecture.md`](docs/architecture.md)
- **Invariants**: [`docs/invariants.md`](docs/invariants.md)
- **Mock-to-production**: [`docs/mock-to-production.md`](docs/mock-to-production.md)
- **Open questions**: [`QUESTIONS.md`](QUESTIONS.md)
- **SDK**: [`client/README.md`](client/README.md)
- **Scripts**: [`scripts/README.md`](scripts/README.md)

## License

MIT — see [LICENSE](LICENSE).

---

<div align="center">

Built for the [Solana Colosseum 2026](https://colosseum.com) by the Poolver team.

</div>
