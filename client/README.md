# @poolver/client

TypeScript SDK for Poolver V1 — on-chain rotating savings + credit
(consórcio / ROSCA) protocol on Solana.

## Install

```bash
yarn add @poolver/client @coral-xyz/anchor @solana/web3.js @solana/spl-token bn.js
```

## Quickstart

```ts
import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { Wallet } from "@coral-xyz/anchor";
import BN from "bn.js";

import {
  PoolverClient,
  USDC_MINT_DEVNET_DEFAULT,
  createPoolIx,
  joinPoolIx,
  contributeIx,
  commitBidIx,
  revealBidIx,
  humanUsdcToMicro,
} from "@poolver/client";

// 1. Wire up
const connection = new Connection("https://api.devnet.solana.com");
const wallet = new Wallet(Keypair.generate()); // your real signer
const client = new PoolverClient({ connection, wallet });

// 2. Create a Tier-0 pool with 1,000-USDC monthly contribution
const { ix: createIx, pool } = await createPoolIx(client, {
  poolId: new BN(1),
  tier: "vault",
  contributionAmount: humanUsdcToMicro("1000"),
  usdcMint: USDC_MINT_DEVNET_DEFAULT,
});

// 3. Send via your usual web3.js plumbing (sendAndConfirmTransaction etc.)

// 4. Other participants join (after each has Light KYC + Reputation)
const joinIx = await joinPoolIx(client, {
  pool,
  tier: "vault",
  usdcMint: USDC_MINT_DEVNET_DEFAULT,
});

// 5. Monthly contribution
const contribIx = await contributeIx(client, {
  pool,
  tier: "vault",
  usdcMint: USDC_MINT_DEVNET_DEFAULT,
});

// 6. Bid (commit + reveal)
const { ix: bidIx, nonce, bidAmount } = await commitBidIx(client, {
  pool,
  month: 1,
  usdcMint: USDC_MINT_DEVNET_DEFAULT,
  bidAmount: humanUsdcToMicro("50"),
});
// Persist `nonce` and `bidAmount` off-chain — you'll need them at reveal.
const revealIx = await revealBidIx(client, {
  pool,
  month: 1,
  bidAmount: bidAmount!,
  nonce: nonce!,
  usdcMint: USDC_MINT_DEVNET_DEFAULT,
});
```

## What's in the box

| Module | Purpose |
|---|---|
| `PoolverClient` | The façade — owns `Connection` + `Wallet` and four `Program<Idl>` handles |
| `instructions/*` | One file per on-chain ix; each exports `<verb>Ix(client, args)` returning a `TransactionInstruction` |
| `queries/*` | Read-only helpers — `fetchPool`, `fetchParticipant`, `fetchReserveFund`, `fetchUserReputation` |
| `pdas.ts` | All PDA derivations (`findPool`, `findParticipant`, etc.) |
| `constants.ts` | Program IDs, PDA seed prefixes, protocol numerics — mirrors Rust |
| `utils/bid_hash.ts` | Commit-hash builder bound by INV-14 (`sha256(amount‖nonce‖pubkey)`) |

## Tier dispatch

`createPool`, `joinPool`, `contribute`, `claimWinning`, and `distributeYield`
each touch the per-pool yield adapter. The SDK looks at `args.tier` and:

1. Picks the adapter program ID (`POOLVER_YIELD_VAULT_PROGRAM_ID` for
   `"vault"`, `POOLVER_YIELD_DEFI_PROGRAM_ID` for `"defi"`).
2. Derives `adapter_state` and `adapter_usdc_vault` against that program ID.
3. For Tier 1 only, appends `[adapter_ktoken_vault]` as a single
   `remainingAccount` per arch §13 / SPEC_QUESTION-36.

## Bid commit hash

The commit hash bound by INV-14 is:

```
sha256( bid_amount.to_le_bytes() (8) ‖ nonce ([u8;16]) ‖ user_pubkey (32) )
```

`utils/bid_hash.ts::buildBidCommitHash` reproduces this byte-for-byte
using Node's built-in `crypto.createHash('sha256')` — zero npm deps.

## V1 limitations

- KYC is mocked (`mockIssueKyc`). Mainnet builds drop this ix entirely
  (`--no-default-features`) and `client.core.methods.mockIssueKyc(...)`
  will throw at runtime against a production cluster.
- Tier 1 (DeFi) Kamino integration is mocked. Yield is injected via
  admin-only `mockInjectYield`; production replaces with real Kamino CPI.
- Switchboard VRF integration is stubbed. The on-chain `select_winner`
  uses a deterministic mock entropy in V1.

See `docs/mock-to-production.md` for the full migration checklist.

## Regenerating the IDL bundle

The SDK ships with copies of `target/idl/*.json` and `target/types/*.ts`
under `client/src/idls/` so it's self-contained. To refresh after
program changes:

```bash
anchor build
cp target/idl/poolver_*.json client/src/idls/
cp target/types/poolver_*.ts  client/src/idls/
yarn --cwd client build
```

## Build

```bash
cd client && yarn install && yarn build
```

Outputs to `client/dist/`.
