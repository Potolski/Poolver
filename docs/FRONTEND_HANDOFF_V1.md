# Poolver V1 — Frontend Handoff

**Audience:** Brenno (frontend) + anyone wiring a UI to Poolver V1.
**Status:** programs deployed to **devnet** (3/4 confirmed at write time; `poolver-core` pending one top-up).
**Source of truth:** `docs/SPEC_V1.md`, `docs/architecture.md`. This doc is the operational summary.

---

## 1. What changed (vs the old `poolver` program)

The legacy `poolver` program (`Fz4Kq...4114`) is **gone** — moved to `archive/poolver-legacy/`. It has been replaced by **four** Anchor programs implementing a different domain model.

### Naming changes

| Legacy concept | V1 concept |
|---|---|
| `Group` | `Pool` |
| `Member` | `Participant` |
| `Round` | `Month` (as `pool.current_month: u8`) |
| `commit_round` / `resolve_round` | `commit_bid` / `reveal_bid` / `select_winner` |
| Single insurance PDA | **Tier-segregated `ReserveFund`** PDAs |
| Flat collateral % | **Reputation-graduated** collateral (100/70/50% of baseline) |
| (none) | **KYC** required — Light to join, Full to bid/win |
| (none) | **Tier 0 / Tier 1** yield adapters (Vault / DeFi) |

Everything in the existing `app/src/lib/program.ts`, `app/src/lib/idl/`, and `app/src/lib/pdas.ts` references the legacy program. **All of it must be replaced.** A fresh TypeScript SDK at `client/` is provided to make this turnkey.

---

## 2. Devnet program IDs

| Program | Devnet ID | Status |
|---|---|---|
| `poolver_core` | `2SsxJqMCYKCYesfzfXASgAPPz153j8tYMXpMKKmt2QXk` | ⏳ pending top-up |
| `poolver_reserve` | `CfxRT3jsXWQZRev67ztqaNKCrHaKF6ieW9a1E8NDPvnx` | ✅ deployed |
| `poolver_yield_vault` | `A3ERUDLAdqdwgqgAoYLftxA6F1QtxSHZYu8DpNDXyyUp` | ✅ deployed |
| `poolver_yield_defi` | `DAitPF7KHzRDVWcV4XM3J7dYGrKJkH332dQHPYUiP7UP` | ✅ deployed |

Sources of truth: `Anchor.toml` `[programs.devnet]`, also exported as TypeScript constants in `client/src/constants.ts`.

### Other constants

- **USDC mint (devnet test mint):** `4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU` — Circle's official devnet USDC. Faucet at https://faucet.circle.com.
- **USDC decimals:** 6 (so 1 USDC = 1_000_000 base units / "microUSDC")
- **Pool size:** fixed at 12 participants × 12 months
- **Contribution range:** 100 USDC ≤ amount ≤ 10,000 USDC
- **Protocol fee:** 1.5% (150 bps)
- **Reserve fee:** 1.5% Vault / 2.5% DeFi
- **Bid cap:** 20% of monthly pot
- **Bid stake:** 1% of contribution (refundable on reveal)
- **Late penalty:** 2% (added to next contribution)

---

## 3. Use the SDK — don't roll your own client

A complete, typed SDK lives at `client/`. **Use it.** Don't import IDLs directly into `app/`.

### Install

```bash
cd app   # or wherever you want it
yarn add @poolver/client@file:../client    # local link for now
# OR: copy client/dist/ into app/lib/poolver-sdk/ if you can't link
```

Once linked:

```ts
import {
  PoolverClient,
  POOLVER_CORE_PROGRAM_ID,
  USDC_MINT_DEVNET,
  Tier,
} from "@poolver/client";

const client = new PoolverClient({
  connection,         // from @solana/web3.js
  wallet,             // from @solana/wallet-adapter-react
  cluster: "devnet",
});
```

### What the SDK gives you

- `PoolverClient.createPool(...)`, `.joinPool(...)`, `.contribute(...)`, etc. — every program instruction with a typed signature
- PDA derivers for every account (`derivePoolPda`, `deriveParticipantPda`, etc.)
- Query helpers (`fetchPool`, `fetchParticipant`, `fetchUserReputation`, `fetchReserve`)
- Bid hash util — `hashBid(bidAmount, nonce, userPubkey)` matches the Rust `sha256(amount_le || nonce_16 || user_32)` byte-for-byte
- Self-contained IDLs in `client/src/idls/` (no dependency on `target/`)

Read `client/README.md` for the full API surface.

---

## 4. Account model — the things the UI needs to read

### `ProtocolConfig` (singleton; PDA seeds `[b"protocol_config"]`)
Stores admin pubkey, USDC mint, fee bps, paused flag. Read once on app startup; rarely changes.

### `Pool` (PDA seeds `[b"pool", creator, &pool_id.to_le_bytes()]`)
The main entity. Per-pool state:
- `tier: u8` (0 = Vault, 1 = DeFi)
- `contribution_amount: u64` (in microUSDC)
- `current_month: u8` (0 = filling, 1–12 = active, 13 = complete)
- `participants: [Option<Pubkey>; 12]` — fixed array, slot N = the Nth joined participant
- `winners: [MonthWinner; 12]` — slot M-1 = winner of month M (`selected_at == 0` means not yet selected)
- `current_month_started_at: i64`
- `bid_window_ends_at: i64`, `reveal_window_ends_at: i64`
- `total_contributed: u64`, `total_distributed: u64`, `bid_credit_balance: u64`, `total_yield_distributed: u64`
- `paid_count_for_current_month: u8` — how many participants have paid this month (max 12)
- `is_complete: bool`, `completed_at: i64`

`MonthWinner`: `{ month, winner, winning_bid, gross_payout, net_payout, selected_at, selection_method (0 = Bid, 1 = Lottery), claimed }`

### `Participant` (PDA seeds `[b"participant", pool, user]`)
Per-(pool, user) state:
- `paid_months: u16` (bitmap; bit N-1 = month N paid)
- `has_won: bool`, `win_month: u8`, `bid_amount_when_won: u64`
- `collateral_locked: u64`, `collateral_initial: u64`, `collateral_release_per_month: u64`
- `is_late: bool`, `late_marked_at: i64`, `accrued_penalty: u64`
- `is_suspended: bool`, `suspended_at: i64`
- `is_defaulted: bool`, `defaulted_at: i64`, `liquidation_amount: u64`
- `completed_cycles_at_join: u8` — snapshot of user's reputation at join time

### `UserReputation` (PDA seeds `[b"reputation", user]`) — global per user
- `pools_joined: u32`, `pools_completed: u32`, `pools_defaulted: u32`
- `total_contributed_lifetime: u64`, `total_received_lifetime: u64`
- `kyc_status: u8` (0 = None, 1 = Light, 2 = Full)
- `kyc_attestation: Pubkey`, `last_kyc_at: i64`

**Must exist before user joins their first pool.** UI should call `initialize_user_reputation` once per user automatically.

### `KycAttestation` (PDA seeds `[b"kyc", user]`) — global per user
- `level: u8` (1 = Light, 2 = Full)
- `expires_at: i64`, `sanctions_clean: bool`
- `cpf_hash: [u8; 32]` (zeroed in V1 mock)

**For V1, KYC is mocked.** Admin issues attestations via `mock_issue_kyc`. UI should expose a "Verify Identity" button that hits a mocked API endpoint which calls this admin-only ix server-side. **In production this becomes a real Idwall/Sumsub flow.**

### `Bid` (PDA seeds `[b"bid", pool, &month.to_le_bytes(), user]`)
Per-(pool, month, user). Created by `commit_bid`, mutated by `reveal_bid`, read by `select_winner`.
- `commit_hash: [u8; 32]`, `committed_at`, `stake_amount`
- `revealed: bool`, `revealed_amount: u64`, `revealed_at`
- `is_winner: bool`, `stake_refunded: bool`

### `ReserveFund` (PDA seeds `[b"reserve_fund", &(tier as u8).to_le_bytes()]`) — in `poolver-reserve` program
Global per tier. UI shows aggregate "insurance available" by reading both tier 0 and tier 1 funds.
- `tier: u8`, `total_balance: u64`, `total_inflows: u64`, `total_outflows: u64`

---

## 5. Instruction reference (15 instructions across 4 programs)

All are in `poolver-core` unless noted. Table of every instruction the UI invokes:

| Instruction | Caller | Purpose | Notes |
|---|---|---|---|
| `initialize_protocol` | admin | one-time setup | not for UI; runs in `scripts/initialize.ts` |
| `mock_issue_kyc` | admin | grant KYC to user | gated by `mock-kyc` Cargo feature; **devnet only** |
| `initialize_user_reputation` | user | create user's reputation PDA | call before first `join_pool` |
| `create_pool` | creator | new pool | Tier 0 or Tier 1; pass `pool_id`, `tier`, `contribution_amount` |
| `join_pool` | user | join an open pool | requires Light KYC + reputation initialized + `pools_defaulted == 0` |
| `contribute` | user | pay monthly contribution | window: month start → month end + 30d (cure path) |
| `commit_bid` | user | sealed bid for current month's pot | requires Full KYC; locks 1% stake |
| `reveal_bid` | user | reveal bid after window | refunds stake on success |
| `select_winner` | anyone (keeper) | run after reveal window closes | bid path or lottery (mock VRF in V1) |
| `claim_winning` | winner | post collateral, take payout | requires Full KYC |
| `advance_month` | anyone (keeper) | tick month forward | after month duration elapses |
| `mark_late_payment` | anyone (keeper) | day 1–5 grace | accrues 2% penalty |
| `suspend_participant` | anyone (keeper) | day 6+ | blocks bidding |
| `liquidate_default` | anyone (keeper) | day 30+ | drains collateral + reserve drawdown |
| `distribute_yield` | anyone (keeper) | harvest + split | Tier 1 only (no-op on Tier 0) |

**"Anyone (keeper)" instructions** can be called by any wallet. For UX, the affected user's wallet, the pool creator, or a backend keeper bot all work. For V1 demo, the UI can prompt the user to call them manually ("⚠ Late payment detected — click to mark") or run a backend cron.

### Tier dispatch (important!)

5 instructions CPI to a yield adapter and need tier-specific accounts: `create_pool`, `join_pool`, `contribute`, `claim_winning`, `distribute_yield`.

The adapter accounts are passed as **`remaining_accounts`** (not fixed accounts). The SDK handles this — `client/src/instructions/_accounts.ts` builds the right account list based on the pool's tier. **Don't construct these manually unless you have a reason.**

### `mock_issue_kyc` — server-side only

This instruction is admin-only. **Don't expose it from the browser.** The flow:

1. UI: user clicks "Verify Identity"
2. UI → backend: POST /api/kyc/issue with user's wallet pubkey + selected level
3. Backend (with admin keypair): calls `mock_issue_kyc(user, level)` via the SDK
4. Backend → UI: success
5. UI: refetches `KycAttestation` PDA, updates UI state

For demo, the backend can be a single Vercel/Cloudflare function. The admin keypair is the same `deploy-keypair.json` used for deploy.

---

## 6. Critical workflows

### A) New user onboarding
```
1. user connects wallet
2. UI checks: does UserReputation PDA exist? if not → initialize_user_reputation
3. UI checks: does KycAttestation PDA exist? if not → call backend → mock_issue_kyc(user, Light)
4. user can now browse + join pools
```

### B) Create + join a pool (Tier 0 happy path)
```
1. creator: createPool({ poolId: <unique u64>, tier: Tier.Vault, contributionAmount: 1000_000_000 })
2. creator: joinPool({ pool, ... })  // creator becomes participant 0
3. share pool address; other 11 users joinPool
4. on the 12th join, pool auto-starts: current_month = 1, start_timestamp set
```

### C) Monthly contribution
```
1. UI shows: "Pay 1000 USDC for month 3"
2. user: contribute({ pool, ... })
3. UI shows: bit 2 of paid_months now set; pool.paid_count_for_current_month++
```

### D) Bid for the month's pot
```
1. UI shows: "Bid window: 47 hours remaining"
2. user picks bid amount (≤ 20% of net monthly pot — SDK helper computes the cap)
3. UI generates random 16-byte nonce, computes commitHash = sha256(amount_le || nonce || pubkey)
4. UI persists nonce locally (localStorage / IndexedDB) — needed for reveal!
5. user: commitBid({ commitHash })
6. (after bid window closes, reveal window opens — 24h)
7. UI: revealBid({ bidAmount, nonce })
8. UI shows: stake refunded
9. (after reveal window closes, anyone calls select_winner)
```

**⚠ NONCE PERSISTENCE:** if the user loses their nonce between commit and reveal, they can't reveal — their stake is forfeit. UI must persist the nonce reliably (IndexedDB recommended; localStorage at minimum) and warn the user to reveal before window closes.

### E) Claim winnings
```
1. UI watches pool.winners[current_month-1]; if winner == self.pubkey and !claimed:
2. UI calculates: required collateral (depends on completed_cycles_at_join + winning_bid)
3. UI shows: "You won! Post X USDC collateral to receive Y USDC payout"
4. user: claimWinning({ pool, ... })
5. participant.has_won = true; collateral_locked = X; net_payout transferred to user
```

### F) Default cascade UX
```
- day 1+: "⚠ Payment overdue. Pay before day 5 to avoid suspension. Late penalty: 2%."
- day 6+: "❌ Suspended. Pay before day 30 to cure. Cannot bid."
- day 30+: "💀 At liquidation risk. Anyone can call liquidate_default now."
- post-liquidation: "Defaulted. Reputation impacted. Cannot join new pools until V2 reputation review."
```

---

## 7. Migrating the existing `app/`

The current frontend at `app/` was wired against the legacy `poolver` program. **Plan to rewrite the data layer; the UI shell is mostly reusable** since the conceptual model (pool list, pool detail, create wizard) maps cleanly.

### Files to delete or rewrite

| File | Action |
|---|---|
| `app/src/lib/program.ts` | **delete** — replaced by SDK |
| `app/src/lib/idl/*` | **delete** — IDLs live in `@poolver/client/src/idls/` |
| `app/src/lib/pdas.ts` | **delete** — use SDK's `derive*Pda` exports |
| `app/src/lib/types.ts` | **rewrite** — re-export from SDK |
| `app/src/lib/mock-data.ts` | **delete or repurpose** — replace with real on-chain queries |
| `app/src/hooks/useGroup.ts` | **rename + rewrite** — `useGroup` → `usePool`; use SDK's `fetchPool` |
| `app/src/hooks/useGroups.ts` | same |
| `app/src/components/groups/*` | **rename `groups/` → `pools/`**, update imports + types |
| `app/src/app/group/[address]/page.tsx` | rename route to `/pool/[address]/page.tsx` |
| `app/src/app/create/page.tsx` | rewrite the multi-step wizard against new instruction signatures |

### Files that survive ~unchanged

- `app/src/components/layout/*` (already Poolver-branded — recent commit)
- `app/src/components/brand/*`
- `app/src/app/globals.css` (recent mobile UI work — keep)
- Wallet adapter integration — same Solana wallet-adapter, just point at devnet

### Naming/copy already updated

The recent rebrand (`circle` → `Poolver`) is in place across the UI. No copy work needed.

---

## 8. KYC mock — exact flow for the demo

Per spec §5.4, KYC is mocked in V1 but the protocol enforces it identically to production. For the Colosseum demo:

1. **Backend endpoint** (you write this, ~30 lines of code):
```ts
// /api/kyc/issue
import { PoolverClient, KycLevel } from "@poolver/client";
import { Keypair } from "@solana/web3.js";

const adminKeypair = Keypair.fromSecretKey(...); // from env
const client = new PoolverClient({ connection, wallet: adminKeypair, cluster: "devnet" });

export async function POST(req) {
  const { user, level } = await req.json();
  const sig = await client.mockIssueKyc({ user, level });
  return Response.json({ signature: sig });
}
```

2. **UI button** in the wallet menu / onboarding:
```tsx
<button onClick={async () => {
  const res = await fetch("/api/kyc/issue", {
    method: "POST",
    body: JSON.stringify({ user: publicKey.toBase58(), level: "Full" }),
  });
  // ...
}}>
  Verify Identity (Demo)
</button>
```

3. **For the demo:** issue Full KYC by default so users can immediately bid and win. Production would issue Light first, then upgrade to Full after deeper verification.

---

## 9. Things that will trip you up

1. **Account count is high on Tier 1 transactions.** `contribute` on a Tier 1 pool passes ~16 accounts including the adapter accounts via `remaining_accounts`. The SDK handles ordering; if you build manually, follow `client/src/instructions/_accounts.ts`.

2. **Reveal nonce loss = stake forfeit.** Persist the nonce *before* signing the commit transaction. Use IndexedDB.

3. **`paid_count_for_current_month` resets on `advance_month`.** UI should call `advance_month` itself when displaying month N+1 to avoid stale state, OR rely on a keeper bot.

4. **`bid_credit_balance` reduces effective contribution.** Display `effective_due = contribution_amount − pool.bid_credit_balance / unpaid_count_this_month` so users aren't confused.

5. **Tier 1 yield only flows via `mock_inject_yield`** in V1. There's no real Kamino interest. For the demo, the backend can periodically call `mock_inject_yield(amount)` on the DeFi adapter to simulate yield, then call `distribute_yield` to harvest.

6. **Switchboard VRF is mocked.** `select_winner`'s lottery branch uses `sha256(pool || month || slot)` for entropy. Deterministic given slot. For demo continuity, this is fine; production swaps to real Switchboard On-Demand.

7. **Pool `current_month_started_at + month_duration_seconds` is the deadline.** Default `month_duration_seconds = 2_592_000` (30 days). For the demo you can override to a shorter duration (e.g., 600 = 10 minutes) at create time so judges can see a full cycle.

---

## 10. Testing the integration

A judge-facing demo flow that works on devnet:

1. Hit `/api/kyc/issue` for 12 demo wallets (Full level)
2. `initialize_user_reputation` for each
3. Wallet 0 creates a Tier 0 pool with `contribution_amount = 100_000_000` (100 USDC) and `month_duration_seconds = 600` (10 min)
4. All 12 wallets join (each calls `joinPool` — gives the SDK a `participantIndex` to figure out)
5. Pool auto-starts; `current_month = 1`
6. Each wallet contributes month 1 (or skip a few to demo defaults)
7. Bid window opens → some users `commit_bid`
8. After bid window: `reveal_bid`
9. After reveal window: anyone calls `select_winner` (UI button "Run draw")
10. Winner's UI surfaces "Claim now" → `claim_winning`
11. Repeat 6–10 for months 2–12

For Tier 1 demo: same flow but `tier = Tier.DeFi`. After month 3, backend calls `mock_inject_yield(50_000_000)` (50 USDC of "Kamino interest"); UI calls `distribute_yield`; users see their next contribution reduced via `bid_credit_balance`.

---

## 11. Open issues / known limits in V1

See `QUESTIONS.md` for the full list (36 questions, all resolved per architect defaults for V1). The ones most likely to affect your work:

- **No real Kamino integration** (Q-19/20): Tier 1 yields require `mock_inject_yield`.
- **No real Switchboard VRF** (Q-21): lottery branch is deterministic per slot.
- **No real KYC** (Q-26 + spec §5.4): use the admin-issued mock.
- **Reputation default gate is binary** (Q-11): `pools_defaulted > 0` blocks all new joins. UI should handle this gracefully ("This wallet has a default in pool X. Use a different wallet or wait for V2 reputation review.")
- **Pool `_reserved` is exhausted**: any new Pool field requires either Pool size growth or carving from another struct.
- **No `emergency_pause` UI yet** (admin instruction not yet wired in scripts).

---

## 12. Quick reference — what to import from the SDK

```ts
import {
  // Client
  PoolverClient,

  // Constants
  POOLVER_CORE_PROGRAM_ID,
  POOLVER_RESERVE_PROGRAM_ID,
  POOLVER_YIELD_VAULT_PROGRAM_ID,
  POOLVER_YIELD_DEFI_PROGRAM_ID,
  USDC_MINT_DEVNET,
  USDC_DECIMALS,
  PROTOCOL_FEE_BPS,
  VAULT_RESERVE_FEE_BPS,
  DEFI_RESERVE_FEE_BPS,
  BID_CAP_BPS,
  BID_STAKE_BPS,
  LATE_PENALTY_BPS,
  GRACE_PERIOD_SECS,
  SUSPENSION_THRESHOLD_SECS,
  LIQUIDATION_THRESHOLD_SECS,

  // Enums
  Tier,                 // Vault | DeFi
  KycLevel,             // Light | Full
  SelectionMethod,      // Bid | Lottery

  // PDA derivers
  deriveProtocolConfigPda,
  derivePoolPda,
  deriveParticipantPda,
  deriveUserReputationPda,
  deriveKycAttestationPda,
  deriveBidPda,
  deriveReserveFundPda,
  // ...

  // Queries
  fetchProtocolConfig,
  fetchPool,
  fetchParticipant,
  fetchUserReputation,
  fetchKycAttestation,
  fetchReserveFund,

  // Utils
  hashBid,              // sha256(amount_le || nonce_16 || pubkey_32)
  generateNonce,        // crypto.getRandomValues(16 bytes)
  computeMonthlyPot,    // 12 × (contribution − fees)
  computeBidCap,        // pot × 2000 / 10000
  computeCollateralRequired, // baseline × rep_mult / 10000 + bid_premium
} from "@poolver/client";
```

If anything is missing from the SDK, file an issue and I'll add it. The SDK is intentionally thin — just enough to build a UI without re-deriving the math.

---

## 13. Where to find more

- **Spec:** `docs/SPEC_V1.md`
- **Architecture:** `docs/architecture.md` — full sequence diagrams, account layouts, CPI matrix
- **Invariants:** `docs/invariants.md` — 36 INVs the protocol guarantees
- **Mock-to-production:** `docs/mock-to-production.md` — what changes when V2 swaps in real KYC + Kamino + VRF
- **Open questions:** `QUESTIONS.md`
- **SDK:** `client/README.md`
- **Deploy scripts:** `scripts/README.md`

For anything blocking, ping David. KYC wiring + the demo flow are the two areas with highest friction; everything else should be mechanical UI work.
