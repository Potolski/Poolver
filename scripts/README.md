# Scripts

Operator scripts for deploying, initializing, and topping up Poolver V1.

All scripts are TypeScript and run via `tsx`. They assume the SDK at
`client/` builds (or at least typechecks via the source — `tsx` doesn't
require a precompile).

## Prereqs

```bash
yarn install
yarn --cwd client install
```

The deploy keypair lives at `./deploy-keypair.json`. Override with
`--wallet <path>` on any script.

## `deploy.ts`

Deploys the four programs in dependency order.

```bash
# Devnet (default)
npx tsx scripts/deploy.ts --cluster devnet

# Mainnet — requires explicit confirmation phrase
npx tsx scripts/deploy.ts --cluster mainnet-beta

# Dry run — prints commands without executing
npx tsx scripts/deploy.ts --cluster devnet --dry-run

# Reuse existing target/ artifacts (devnet only)
npx tsx scripts/deploy.ts --cluster devnet --skip-build
```

### Mainnet safeguards

The script refuses to deploy to `mainnet-beta` unless ALL of the
following pass:

1. The operator types the exact phrase `DEPLOY MAINNET POOLVER-V1`.
2. Every program's IDL has zero `mock_*` instructions
   (`jq '.instructions | map(.name)' target/idl/*.json`).
3. Every `.so` has zero `Instruction: Mock*` panic strings
   (`strings target/deploy/*.so | grep 'Instruction: Mock'`).

The `nm` symbol-table check from earlier draft is intentionally NOT
used: stripped Solana .so files have no symbols, so `nm` is a false
negative. `strings`-based detection works because Anchor embeds an
`Instruction: <PascalName>` panic message per dispatched instruction
(see `programs/poolver-yield-defi/src/lib.rs` and Anchor's macro
expansion).

The script does NOT skip the build on mainnet — it always runs
`anchor build -- --no-default-features` to ensure the artifacts being
deployed exactly match the source verifiable in git.

## `initialize.ts`

One-time post-deploy setup. Idempotent — re-running skips already-
initialized accounts.

```bash
npx tsx scripts/initialize.ts \
  --cluster devnet \
  --usdc-mint <USDC_MINT> \
  --wallet ./deploy-keypair.json
```

Creates:

- `ProtocolConfig` (singleton) via `poolver_core::initialize_protocol`
- `ReserveFund(Vault)` via `poolver_reserve::initialize_reserve(Tier::Vault)`
- `ReserveFund(DeFi)` via `poolver_reserve::initialize_reserve(Tier::DeFi)`

## `seed-reserve.ts`

Admin tops up a tier reserve.

```bash
npx tsx scripts/seed-reserve.ts \
  --tier vault \
  --amount 10000 \
  --cluster devnet \
  --usdc-mint <USDC_MINT>
```

Calls `poolver_reserve::seed(amount)`. The admin's USDC ATA must hold
the funds.

## Outputs

`deploy.ts` writes a JSON receipt to `deployments/<cluster>-<UTC-iso>.json`:

```json
{
  "cluster": "devnet",
  "deployedAt": "2026-04-30T15:00:00.000Z",
  "deployer": "...",
  "dryRun": false,
  "programs": [
    { "program": "poolver-yield-vault", "programId": "..." },
    { "program": "poolver-yield-defi",  "programId": "..." },
    { "program": "poolver-reserve",     "programId": "..." },
    { "program": "poolver-core",        "programId": "..." }
  ]
}
```
