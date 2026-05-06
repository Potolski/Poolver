/**
 * `PoolverClient` — single-entry façade over all 4 Anchor programs.
 *
 * Owns one `AnchorProvider` and four `Program<...>` handles. Every
 * high-level method (e.g. `createPool`, `joinPool`, `contribute`) is
 * implemented in `src/instructions/<verb>.ts`. The client just wires
 * those builders to a provider and exposes the result.
 *
 * Design intent (V1):
 *   - The SDK does NOT subscribe to events, cache state, or retry.
 *   - Every method returns either a `TransactionInstruction[]` (for
 *     composition) via the `*.instructions(...)` helpers, or a tx
 *     signature via `*.rpc(...)`. The generated Anchor `program.methods`
 *     pattern handles both.
 *   - Tier dispatch is handled inside each instruction module, NOT in the
 *     client constructor. Pool tier is resolved at call time via either
 *     a `pool.tier` field or an explicit `tier` arg.
 */
import {
  AnchorProvider,
  Program,
  Wallet,
  Idl,
} from "@coral-xyz/anchor";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";

import {
  POOLVER_CORE_IDL,
  POOLVER_RESERVE_IDL,
  POOLVER_YIELD_DEFI_IDL,
  POOLVER_YIELD_VAULT_IDL,
} from "./types";

export interface PoolverClientOpts {
  connection: Connection;
  wallet: Wallet;
  /** Anchor commitment override; default `"confirmed"`. */
  commitment?: "processed" | "confirmed" | "finalized";
}

export class PoolverClient {
  public readonly connection: Connection;
  public readonly wallet: Wallet;
  public readonly provider: AnchorProvider;

  // Generic Idl typing (Program<Idl>) keeps the public surface stable
  // even if anchor's TS type generation drifts; the strongly-typed
  // accessors below cast to the per-IDL types from `src/idls/*.ts`.
  public readonly core: Program<Idl>;
  public readonly reserve: Program<Idl>;
  public readonly yieldVault: Program<Idl>;
  public readonly yieldDefi: Program<Idl>;

  constructor(opts: PoolverClientOpts) {
    this.connection = opts.connection;
    this.wallet = opts.wallet;
    this.provider = new AnchorProvider(opts.connection, opts.wallet, {
      commitment: opts.commitment ?? "confirmed",
    });

    this.core = new Program(POOLVER_CORE_IDL, this.provider);
    this.reserve = new Program(POOLVER_RESERVE_IDL, this.provider);
    this.yieldVault = new Program(POOLVER_YIELD_VAULT_IDL, this.provider);
    this.yieldDefi = new Program(POOLVER_YIELD_DEFI_IDL, this.provider);
  }

  /** Build a read-only client (no signer) from a `Connection`. */
  static readOnly(connection: Connection): PoolverClient {
    const dummy = new Wallet(Keypair.generate());
    return new PoolverClient({ connection, wallet: dummy });
  }

  /** Pubkey of the configured signer. */
  get authority(): PublicKey {
    return this.wallet.publicKey;
  }
}
