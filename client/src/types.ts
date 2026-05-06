/**
 * Re-exports of generated IDL types plus a few SDK-augmented aliases.
 *
 * The IDLs and TS type files live in `src/idls/` so the SDK is
 * self-contained — clients do not need to point at `target/types/`.
 * To refresh, see `client/README.md` § "Regenerating the IDL bundle".
 */
export type { PoolverCore } from "./idls/poolver_core";
export type { PoolverReserve } from "./idls/poolver_reserve";
export type { PoolverYieldVault } from "./idls/poolver_yield_vault";
export type { PoolverYieldDefi } from "./idls/poolver_yield_defi";

import { Idl } from "@coral-xyz/anchor";

// Raw IDL JSON imports — use these to construct anchor.Program instances.
import poolverCoreIdlJson from "./idls/poolver_core.json";
import poolverReserveIdlJson from "./idls/poolver_reserve.json";
import poolverYieldVaultIdlJson from "./idls/poolver_yield_vault.json";
import poolverYieldDefiIdlJson from "./idls/poolver_yield_defi.json";

export const POOLVER_CORE_IDL = poolverCoreIdlJson as Idl;
export const POOLVER_RESERVE_IDL = poolverReserveIdlJson as Idl;
export const POOLVER_YIELD_VAULT_IDL = poolverYieldVaultIdlJson as Idl;
export const POOLVER_YIELD_DEFI_IDL = poolverYieldDefiIdlJson as Idl;

// ─────────────────────────── Convenience aliases ─────────────────────

export type { TierName, KycLevelName } from "./constants";
