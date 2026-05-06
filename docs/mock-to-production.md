# Mock-to-Production Migration Checklist

> Every site marked with `// MOCK_KYC:`, `// SPEC_QUESTION-19:`, `// SPEC_QUESTION-20:`, `// SPEC_QUESTION-21:`, `// SPEC_QUESTION-23:` and what must change for production.
>
> Originally drafted as a prospective spec; finalized for the V1 submission against the actual code at the paths cited below. Every `// MOCK_KYC:` site in the program codebase has a matching entry here. CI should fail if a marker is added without an accompanying checklist update.

---

## V1 Build-Guard Verification (concrete commands)

Per QUESTIONS.md Q-33 (RESOLVED). These commands form the deploy-time gate
that mainnet builds pass before `anchor deploy` runs (see
`scripts/deploy.ts::verifyMockFree`):

### 1. IDL check via `jq`

```bash
# poolver-core mainnet build must NOT expose mock_issue_kyc
jq -e '.instructions | map(.name) | index("mock_issue_kyc")' \
  target/idl/poolver_core.json && echo "REFUSE" || echo "OK"

# poolver-yield-defi mainnet build must NOT expose any mock_*
for needle in mock_inject_yield mock_set_utilization \
              mock_set_oracle_deviation mock_set_kamino_paused; do
  jq -e ".instructions | map(.name) | index(\"$needle\")" \
    target/idl/poolver_yield_defi.json && echo "REFUSE" || echo "OK"
done
```

### 2. `.so` panic-string check via `strings(1)`

Stripped Solana `.so` files have **no symbol table** — `nm` returns "no
symbols". The reliable detector is `strings`: Anchor embeds an
`Instruction: <PascalName>` panic message per dispatched instruction, so
the absence of those strings proves the instruction is not in the
dispatch table.

```bash
# Should print NOTHING for a mainnet build:
strings target/deploy/poolver_yield_defi.so | grep -E 'Instruction: Mock'

# Should also print NOTHING:
strings target/deploy/poolver_core.so | grep -E 'Instruction: MockIssueKyc'
```

### Verified output (V1, 2026-04-30)

```text
$ cargo build-sbf --manifest-path programs/poolver-yield-defi/Cargo.toml --no-default-features
   Finished `release` profile [optimized] target(s)

$ ls -la target/deploy/poolver_yield_defi.so
-rwxr-xr-x  271040 bytes   (vs 756472 with mocks ON)

$ strings target/deploy/poolver_yield_defi.so | grep -E 'Instruction: Mock'
(no output — guard passes)

$ strings target/deploy/poolver_yield_defi.so | grep -E 'Instruction:'
Instruction: ResetCircuitBreakerInstruction: InitializeAdapterInstruction: DepositInstruction: WithdrawInstruction: HarvestInstruction: EmergencyUnwind
```

`reset_circuit_breaker` is **NOT** a mock — it's a real admin-only
recovery instruction that survives `--no-default-features`. The guard
does not flag it. Confirmed surviving instructions: `initialize_adapter`,
`deposit`, `withdraw`, `harvest`, `emergency_unwind`,
`reset_circuit_breaker`.

For `poolver-core`, the survivors are the 14 production instructions:
`initialize_protocol`, `initialize_user_reputation`, `create_pool`,
`join_pool`, `contribute`, `advance_month`, `commit_bid`, `reveal_bid`,
`select_winner`, `claim_winning`, `distribute_yield`, `mark_late_payment`,
`suspend_participant`, `liquidate_default`. Only `mock_issue_kyc` drops
out.

---

## How the Mock Pattern Works

Per spec §5.4 and §9.11:

1. **The verification logic is identical between V1 and production.** Every instruction that consults KYC reads a `KycAttestation` PDA and checks: existence, level, expiry, sanctions_clean. Same code path.
2. **Only issuance differs.** V1 has `mock_issue_kyc(user, level)` callable by `admin`. Production has `issue_kyc_attestation(...)` callable by `protocol_config.kyc_oracle`, which is set by an oracle integration to a key controlled by the off-chain Idwall/Sumsub flow.
3. **The mock is compile-time gated** behind Cargo feature `mock-kyc`. Production builds set `--no-default-features --features production` and the mock instruction does not exist in the deployed `.so`.

---

## Migration Checklist

Each entry below maps a code site (with the `// MOCK_KYC:` or `// SPEC_QUESTION-{19,20,21,23}:` marker) to the production change required. File:line references are cross-checked against the V1 codebase as of the submission cutoff.

### V1 Marker Inventory (verified at submission)

`grep -rn '// MOCK_KYC:\|// SPEC_QUESTION-19:\|// SPEC_QUESTION-20:\|// SPEC_QUESTION-21:\|// SPEC_QUESTION-23:' programs/ --include='*.rs' | grep -v '/tests/'`:

| File:line | Marker | Class |
|---|---|---|
| `programs/poolver-core/src/lib.rs:40` | MOCK_KYC | dispatch (cfg-gated) |
| `programs/poolver-core/src/instructions/mock_issue_kyc.rs:1,31,86,89` | MOCK_KYC | mock issuer (file feature-gated) |
| `programs/poolver-core/src/instructions/initialize_protocol.rs:15,56` | MOCK_KYC | admin == kyc_oracle in V1 |
| `programs/poolver-core/src/instructions/join_pool.rs:61,324` | MOCK_KYC | gate against KycAttestation PDA |
| `programs/poolver-core/src/instructions/commit_bid.rs:78,182` | MOCK_KYC | gate against KycAttestation PDA |
| `programs/poolver-core/src/state.rs:441,444` | MOCK_KYC | KycAttestation field placeholders |
| `programs/poolver-core/src/instructions/select_winner.rs:14,240,614` | SPEC_QUESTION-21 | mocked VRF entropy |
| `programs/poolver-core/src/events.rs:141` | SPEC_QUESTION-21 | event annotated for indexer hint |
| `programs/poolver-yield-defi/src/lib.rs:25,31,34,71` | SPEC_QUESTION-19/20/23 | dispatch table comment + cfg gate |
| `programs/poolver-yield-defi/src/constants.rs:14` | SPEC_QUESTION-19 | DEFI_ADAPTER_KTOKEN_SEED comment |
| `programs/poolver-yield-defi/src/state.rs:7,24,31,67` | SPEC_QUESTION-19/20/23 | mock-only state fields |
| `programs/poolver-yield-defi/src/events.rs:75` | SPEC_QUESTION-19 | indexer hint |
| `programs/poolver-yield-defi/src/instructions/initialize_adapter.rs:17,66,92` | SPEC_QUESTION-19 | mock kToken vault setup |
| `programs/poolver-yield-defi/src/instructions/deposit.rs:60,113` | SPEC_QUESTION-19 | mock 75/25 split via internal transfer |
| `programs/poolver-yield-defi/src/instructions/harvest.rs:60` | SPEC_QUESTION-19 | mock yield read |
| `programs/poolver-yield-defi/src/instructions/withdraw.rs:80,108` | SPEC_QUESTION-19 | mock kToken redeem |
| `programs/poolver-yield-defi/src/instructions/emergency_unwind.rs:16,87` | SPEC_QUESTION-19 | mock kToken full unwind |
| `programs/poolver-yield-defi/src/instructions/reset_circuit_breaker.rs:45` | SPEC_QUESTION-19 | comment only — instruction is real |
| `programs/poolver-yield-defi/src/instructions/mock_helpers.rs` (entire file) | SPEC_QUESTION-19/20/23 | dev-only mock helpers (feature-gated) |

`programs/poolver-core/src/kyc.rs` is intentionally **NOT** marked: per
INV-25 the verification logic is mock-agnostic and must not change
between V1 and production.

### Site 1 — `mock_issue_kyc` instruction handler
- **File (future):** `programs/poolver-core/src/instructions/mock_issue_kyc.rs`
- **Spec ref:** §5.1, §5.4
- **Marker:** `// MOCK_KYC: V1-only instruction; replaced in production by issue_kyc_attestation.`
- **Production change:**
  1. Delete the file (or feature-gate it under `mock-kyc` exclusively).
  2. Add `programs/poolver-core/src/instructions/issue_kyc_attestation.rs` with identical body except: signer = `protocol_config.kyc_oracle` (constraint), no admin check; instead, accept a payload signed off-chain by the oracle key (Ed25519 signature verification on-chain or implicit via signer requirement).
  3. Update `lib.rs` `#[program]` mod to expose `issue_kyc_attestation` (no `cfg`).

### Site 2 — `mock_issue_kyc` route declaration in `lib.rs`
- **File (future):** `programs/poolver-core/src/lib.rs`
- **Spec ref:** §9.11
- **Marker:** `// MOCK_KYC: feature-gated; absent in production builds.`
- **Production change:**
  1. Remove the `#[cfg(feature = "mock-kyc")]` attribute and rename to `issue_kyc_attestation`.
  2. Add CI assertion: built IDL contains `issue_kyc_attestation` and does NOT contain `mock_issue_kyc`.

### Site 3 — `protocol_config.kyc_oracle` initialization
- **File (future):** `programs/poolver-core/src/instructions/initialize_protocol.rs`
- **Spec ref:** §5.1 `initialize_protocol`
- **Marker:** `// MOCK_KYC: V1 sets kyc_oracle = admin. Production must set to dedicated oracle keypair.`
- **Production change:**
  1. Either pass `kyc_oracle: Pubkey` as an arg to `initialize_protocol`, or add a separate one-shot `set_kyc_oracle` admin instruction.
  2. Document the oracle keypair custody (HSM-backed; off-chain Idwall integration holds the signing key).
  3. Verify mainnet deploy script sets it post-init.

### Site 4 — `KycAttestation.cpf_hash` zeroing
- **File (future):** `programs/poolver-core/src/instructions/mock_issue_kyc.rs`
- **Spec ref:** §3 KycAttestation
- **Marker:** `// MOCK_KYC: cpf_hash is zeroed in V1 mock; production stores sha256(CPF).`
- **Production change:**
  1. The production `issue_kyc_attestation` accepts `cpf_hash: [u8; 32]` from the oracle (oracle has computed sha256 off-chain).
  2. On-chain: store as-is. Never accept raw CPF on-chain.

### Site 5 — `KycAttestation.sanctions_clean` always-true
- **File (future):** `programs/poolver-core/src/instructions/mock_issue_kyc.rs`
- **Spec ref:** §3 KycAttestation
- **Marker:** `// MOCK_KYC: V1 always sets sanctions_clean=true; production reads from off-chain check.`
- **Production change:**
  1. Production accepts `sanctions_clean: bool` from oracle.
  2. Off-chain: oracle queries OFAC + Brazilian sanctions list before issuing.

### Site 6 — `require_full_kyc` helper (NO change needed in production)
- **File (future):** `programs/poolver-core/src/kyc.rs`
- **Spec ref:** §5.4 ("All instructions that check KYC use the same verification logic as production")
- **Marker:** *None.* This file is intentionally not marked, because it does not change.
- **Production change:** none. **This is the contract:** verification logic is mock-agnostic.
- **CI assertion:** `grep -r "MOCK_KYC" programs/poolver-core/src/kyc.rs` returns empty.

### Site 7 — `claim_winning` Full-KYC gate
- **File (future):** `programs/poolver-core/src/instructions/claim_winning.rs`
- **Spec ref:** §5.1 `claim_winning`, §5.1.5
- **Marker:** `// MOCK_KYC: gates pass identically against mock or real KycAttestation.`
- **Production change:** none. Comment is for grep/audit hygiene only.

### Site 8 — `commit_bid` Full-KYC gate
- **File (future):** `programs/poolver-core/src/instructions/commit_bid.rs`
- **Spec ref:** §5.1 `commit_bid`
- **Marker:** `// MOCK_KYC: same verification helper as claim_winning.`
- **Production change:** none.

### Site 9 — `join_pool` Light-KYC gate
- **File (future):** `programs/poolver-core/src/instructions/join_pool.rs`
- **Spec ref:** §5.1 `join_pool`
- **Marker:** `// MOCK_KYC: same verification helper.`
- **Production change:** none.

### Site 10 — `KycAttestationIssued` event reuse
- **File (future):** `programs/poolver-core/src/events.rs`
- **Spec ref:** §6 ("`KycAttestationIssued` (mock or real, same event)")
- **Marker:** `// MOCK_KYC: event identical for mock and production issuers; indexer cannot distinguish.`
- **Production change:** none. Optional enhancement: add `issuer_kind: u8` field for indexer to distinguish; document as a forward-compatible field.

### Site 11 — Cargo feature flag declaration
- **File (future):** `programs/poolver-core/Cargo.toml`
- **Spec ref:** §9.11
- **Marker:** comment near the feature: `# MOCK_KYC: gates compilation of mock_issue_kyc.`
- **Production change:**
  1. Mainnet build: `anchor build --provider.cluster mainnet -- --no-default-features --features production`
  2. CI runs both build configs to ensure both compile.

### Site 12 — Deploy script guard
- **File (future):** `scripts/deploy.ts`
- **Spec ref:** §9.11, §12.5
- **Marker:** `// MOCK_KYC: refuse mainnet deploy if mock-kyc feature is in compiled IDL.`
- **Production change:**
  1. Script reads `target/idl/poolver_core.json`, asserts `instructions[].name` does NOT include `mock_issue_kyc` when `cluster === 'mainnet-beta'`.
  2. Refuse with non-zero exit otherwise.

### Site 13 — `rotate_kyc_oracle` admin instruction
- **File (future):** `programs/poolver-core/src/instructions/rotate_kyc_oracle.rs`
- **Spec ref:** SPEC_QUESTION-24 (this file pre-empts the production migration question by adding rotation now)
- **Marker:** `// MOCK_KYC: V1 sets kyc_oracle = admin; this instruction lets production rotate to a real oracle without program upgrade.`
- **Production change:**
  1. Use this instruction at production cutover to set `kyc_oracle = <oracle_pubkey>`.
  2. After cutover, `mock_issue_kyc` (if still present in pre-production builds) is dead code — every gate uses `kyc_oracle`, not admin.

### Site 14 — Test fixtures
- **File (future):** `tests/common/kyc.ts`
- **Spec ref:** §8 ("Mock KYC flow")
- **Marker:** `// MOCK_KYC: test helper; calls mock_issue_kyc.`
- **Production change:**
  1. Replace with test helper that signs an attestation payload with a test oracle keypair (different keypair, same shape).
  2. The on-chain assertions in tests do not change.

### Site 15 — Frontend KYC banner
- **File (future):** `app/src/components/KycBanner.tsx` (or equivalent)
- **Spec ref:** outside spec
- **Marker:** `// MOCK_KYC: shows "demo KYC" banner in V1; remove in production.`
- **Production change:** remove banner; replace with real Idwall/Sumsub integration UI.

---

## Production Cutover Runbook (Future)

Sequencing (executed once Idwall/Sumsub integration is ready):

1. **Off-chain:** generate KYC oracle keypair (HSM). Custody documented.
2. **On-chain (program upgrade):** deploy new program build (`--features production`). The `mock_issue_kyc` instruction no longer exists in the deployed `.so`.
3. **On-chain (admin tx):** call `rotate_kyc_oracle(<new_oracle_pubkey>)`.
4. **Off-chain:** start KYC oracle service. Begin issuing real attestations via `issue_kyc_attestation`.
5. **Migration:** existing V1 `KycAttestation` PDAs remain valid (same on-chain data shape); they are honored until expiry. New issuances use the production flow.
6. **Telemetry:** indexer monitors `KycAttestationIssued` events; verify `issued_by` matches the new oracle.
7. **Frontend:** swap KycBanner for real Idwall/Sumsub UI.
8. **Audit close-out:** confirm no `// MOCK_KYC:` markers remain in the program codebase except in tests and comments documenting historical context.

---

## CI Hygiene

Add to CI:

```text
# Fail if MOCK_KYC sites in code don't match this checklist.
mock_kyc_in_code=$(grep -r "// MOCK_KYC:" programs/ scripts/ app/src/ | wc -l)
mock_kyc_in_doc=$(grep -c "^### Site " docs/mock-to-production.md)
[[ "$mock_kyc_in_code" -le "$mock_kyc_in_doc" ]] || { echo "Unmatched MOCK_KYC sites"; exit 1; }

# Fail mainnet build if mock_issue_kyc in IDL.
if [[ "$CLUSTER" == "mainnet-beta" ]]; then
  ! grep -q "mock_issue_kyc" target/idl/poolver_core.json
fi
```

---

*End of mock-to-production.md. Update as `// MOCK_KYC:` sites are added during implementation.*
