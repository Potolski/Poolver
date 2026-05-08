import { PublicKey } from "@solana/web3.js";
import BN from "bn.js";

import { PoolverClient } from "../poolver";
import { findKycAttestation } from "../pdas";
import { KycLevelName } from "../constants";

export interface KycAttestationView {
  publicKey: PublicKey;
  user: PublicKey;
  level: KycLevelName;
  issuedBy: PublicKey;
  issuedAt: BN;
  expiresAt: BN;
  /** Hex string for ergonomic comparison; zero in V1 mock. */
  cpfHashHex: string;
  sanctionsClean: boolean;
  raw: Record<string, unknown>;
}

function decodeLevel(byte: number): KycLevelName {
  switch (byte) {
    case 0:
      return "none";
    case 1:
      return "light";
    case 2:
      return "full";
    default:
      throw new Error(`unrecognized kyc level byte: ${byte}`);
  }
}

/**
 * Reads the user's KycAttestation PDA. Returns null if it doesn't
 * exist (i.e. the user has never been KYC'd).
 *
 * The on-chain handlers all check this PDA directly (not the
 * `kycStatus` field on UserReputation, which is a planned cache that
 * V1 doesn't write). Frontends that need to gate UI on KYC should
 * read from here.
 */
export async function fetchKycAttestation(
  client: PoolverClient,
  user: PublicKey
): Promise<KycAttestationView | null> {
  const [pda] = findKycAttestation(user);
  const raw = (await (
    client.core.account as any
  ).kycAttestation.fetchNullable(pda)) as Record<string, any> | null;
  if (!raw) return null;
  const cpfBytes = raw.cpfHash as number[] | Uint8Array;
  const cpfHashHex = Array.from(cpfBytes)
    .map((b) => (b as number).toString(16).padStart(2, "0"))
    .join("");
  return {
    publicKey: pda,
    user: raw.user as PublicKey,
    level: decodeLevel(raw.level as number),
    issuedBy: raw.issuedBy as PublicKey,
    issuedAt: raw.issuedAt as BN,
    expiresAt: raw.expiresAt as BN,
    cpfHashHex,
    sanctionsClean: Boolean(raw.sanctionsClean),
    raw,
  };
}

/**
 * Returns true if the user has a non-expired KYC attestation at the
 * given level (or higher). Mirrors the on-chain validation.
 */
export function isKycValid(
  attestation: KycAttestationView | null,
  required: "light" | "full",
  nowUnixSeconds: number = Math.floor(Date.now() / 1000)
): boolean {
  if (!attestation) return false;
  if (!attestation.sanctionsClean) return false;
  if (attestation.expiresAt.toNumber() <= nowUnixSeconds) return false;
  if (required === "full" && attestation.level !== "full") return false;
  if (required === "light" && attestation.level === "none") return false;
  return true;
}
