/**
 * Bid commit-hash construction (INV-14, spec §5.1, arch §2.1).
 *
 * The on-chain reveal handler computes:
 *
 *   sha256( bid_amount.to_le_bytes() (8) || nonce ([u8;16]) || user_pubkey (32) )
 *
 * via `solana_sha256_hasher::hashv` over the three slices. We replicate the
 * exact byte layout here using Node's built-in `crypto.createHash('sha256')`
 * so the SDK has zero npm dependency for hashing.
 *
 * Source of truth:
 *   programs/poolver-core/src/instructions/reveal_bid.rs:190-200
 */
import { createHash, randomBytes } from "crypto";

import { PublicKey } from "@solana/web3.js";
import BN from "bn.js";

/** 16 random bytes — recommended nonce length matching the Rust handler. */
export const BID_NONCE_LEN = 16;

/**
 * Build the 32-byte commit hash bound by INV-14.
 *
 * @param bidAmount   bid amount in microUSDC (u64)
 * @param nonce       16 random bytes; treat as a secret until reveal
 * @param userPubkey  the bidder's wallet pubkey (32 bytes)
 */
export function buildBidCommitHash(
  bidAmount: BN,
  nonce: Uint8Array,
  userPubkey: PublicKey
): Uint8Array {
  if (nonce.length !== BID_NONCE_LEN) {
    throw new Error(`nonce must be ${BID_NONCE_LEN} bytes; got ${nonce.length}`);
  }
  if (bidAmount.isNeg()) {
    throw new Error("bid amount must be non-negative");
  }
  // BN.toArray('le', 8) → exactly 8 LE bytes of a u64.
  const amountLe = Buffer.from(bidAmount.toArray("le", 8));
  const pubkeyBytes = userPubkey.toBuffer();

  const hash = createHash("sha256");
  hash.update(amountLe);
  hash.update(Buffer.from(nonce));
  hash.update(pubkeyBytes);
  return new Uint8Array(hash.digest());
}

/** Generate a 16-byte cryptographically-random bid nonce. */
export function randomBidNonce(): Uint8Array {
  return new Uint8Array(randomBytes(BID_NONCE_LEN));
}
