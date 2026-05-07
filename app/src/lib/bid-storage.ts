import BN from "bn.js";
import { PublicKey } from "@solana/web3.js";

const DB_NAME = "poolver-v1";
const STORE = "bid_secrets";
const DB_VERSION = 1;
const LS_PREFIX = "poolver:bid:";
export const BID_NONCE_LEN = 16;

export interface BidSecret {
  /** `${poolAddress}|${month}|${userPubkey}` */
  key: string;
  poolAddress: string;
  month: number;
  userPubkey: string;
  /** 16 raw bytes encoded as a number array for JSON safety. */
  nonce: number[];
  /** microUSDC, base-10 string (BN-safe). */
  bidAmountMicro: string;
  /** 32 raw bytes encoded as a number array for JSON safety. */
  commitHash: number[];
  committedAt: number;
  signature?: string;
  revealedAt?: number;
}

export function bidKey(
  poolAddress: PublicKey | string,
  month: number,
  userPubkey: PublicKey | string
): string {
  const pool = typeof poolAddress === "string" ? poolAddress : poolAddress.toBase58();
  const user = typeof userPubkey === "string" ? userPubkey : userPubkey.toBase58();
  return `${pool}|${month}|${user}`;
}

export function generateBidNonce(): Uint8Array {
  const out = new Uint8Array(BID_NONCE_LEN);
  if (typeof globalThis.crypto?.getRandomValues !== "function") {
    throw new Error("Web Crypto getRandomValues unavailable");
  }
  globalThis.crypto.getRandomValues(out);
  return out;
}

/**
 * sha256( amount_le_bytes(8) || nonce(16) || pubkey(32) )
 * Matches INV-14 byte-for-byte against the on-chain Rust commit.
 */
export async function computeBidCommitHash(
  bidAmountMicro: BN,
  nonce: Uint8Array,
  user: PublicKey
): Promise<Uint8Array> {
  if (nonce.length !== BID_NONCE_LEN) {
    throw new Error(`nonce must be ${BID_NONCE_LEN} bytes, got ${nonce.length}`);
  }
  const amountLE = new Uint8Array(bidAmountMicro.toArrayLike(Buffer, "le", 8));
  const userBytes = user.toBytes();
  const buf = new Uint8Array(amountLE.length + nonce.length + userBytes.length);
  buf.set(amountLE, 0);
  buf.set(nonce, amountLE.length);
  buf.set(userBytes, amountLE.length + nonce.length);
  const digest = await globalThis.crypto.subtle.digest("SHA-256", buf);
  return new Uint8Array(digest);
}

function hasIndexedDB(): boolean {
  return typeof window !== "undefined" && typeof window.indexedDB !== "undefined";
}

function hasLocalStorage(): boolean {
  return typeof window !== "undefined" && typeof window.localStorage !== "undefined";
}

function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = window.indexedDB.open(DB_NAME, DB_VERSION);
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains(STORE)) {
        db.createObjectStore(STORE, { keyPath: "key" });
      }
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

async function idbPut(secret: BidSecret): Promise<void> {
  const db = await openDb();
  await new Promise<void>((resolve, reject) => {
    const tx = db.transaction(STORE, "readwrite");
    tx.objectStore(STORE).put(secret);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
  db.close();
}

async function idbGet(key: string): Promise<BidSecret | null> {
  const db = await openDb();
  const result = await new Promise<BidSecret | null>((resolve, reject) => {
    const tx = db.transaction(STORE, "readonly");
    const req = tx.objectStore(STORE).get(key);
    req.onsuccess = () => resolve((req.result as BidSecret | undefined) ?? null);
    req.onerror = () => reject(req.error);
  });
  db.close();
  return result;
}

async function idbDelete(key: string): Promise<void> {
  const db = await openDb();
  await new Promise<void>((resolve, reject) => {
    const tx = db.transaction(STORE, "readwrite");
    tx.objectStore(STORE).delete(key);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
  db.close();
}

async function idbList(): Promise<BidSecret[]> {
  const db = await openDb();
  const result = await new Promise<BidSecret[]>((resolve, reject) => {
    const tx = db.transaction(STORE, "readonly");
    const req = tx.objectStore(STORE).getAll();
    req.onsuccess = () => resolve((req.result as BidSecret[]) ?? []);
    req.onerror = () => reject(req.error);
  });
  db.close();
  return result;
}

export async function saveBidSecret(secret: BidSecret): Promise<void> {
  if (typeof window === "undefined") return;
  if (hasIndexedDB()) {
    try {
      await idbPut(secret);
      return;
    } catch {
      // fall through to localStorage
    }
  }
  if (hasLocalStorage()) {
    window.localStorage.setItem(LS_PREFIX + secret.key, JSON.stringify(secret));
  }
}

export async function loadBidSecret(key: string): Promise<BidSecret | null> {
  if (typeof window === "undefined") return null;
  if (hasIndexedDB()) {
    try {
      const fromIdb = await idbGet(key);
      if (fromIdb) return fromIdb;
    } catch {
      // fall through to localStorage
    }
  }
  if (hasLocalStorage()) {
    const raw = window.localStorage.getItem(LS_PREFIX + key);
    if (raw) {
      try {
        return JSON.parse(raw) as BidSecret;
      } catch {
        return null;
      }
    }
  }
  return null;
}

export async function clearBidSecret(key: string): Promise<void> {
  if (typeof window === "undefined") return;
  if (hasIndexedDB()) {
    try {
      await idbDelete(key);
    } catch {
      // ignore
    }
  }
  if (hasLocalStorage()) {
    window.localStorage.removeItem(LS_PREFIX + key);
  }
}

export async function listBidSecrets(opts?: {
  user?: PublicKey | string;
}): Promise<BidSecret[]> {
  if (typeof window === "undefined") return [];
  let all: BidSecret[] = [];
  if (hasIndexedDB()) {
    try {
      all = await idbList();
    } catch {
      all = [];
    }
  }
  if (all.length === 0 && hasLocalStorage()) {
    for (let i = 0; i < window.localStorage.length; i++) {
      const k = window.localStorage.key(i);
      if (!k || !k.startsWith(LS_PREFIX)) continue;
      const raw = window.localStorage.getItem(k);
      if (!raw) continue;
      try {
        all.push(JSON.parse(raw) as BidSecret);
      } catch {
        // skip
      }
    }
  }
  if (opts?.user) {
    const u = typeof opts.user === "string" ? opts.user : opts.user.toBase58();
    return all.filter((s) => s.userPubkey === u);
  }
  return all;
}
