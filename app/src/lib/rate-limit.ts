import "server-only";

interface Bucket {
  count: number;
  resetAt: number;
}

const buckets = new Map<string, Bucket>();

/**
 * Returns true when the request is allowed. Lazy expiry on insert.
 * Resets on cold-start. Demo-only — for production swap to Upstash/Vercel KV.
 */
export function rateLimit(
  scope: "ip" | "recipient",
  id: string,
  max: number,
  windowMs: number
): boolean {
  const key = `${scope}:${id}`;
  const now = Date.now();
  const existing = buckets.get(key);
  if (!existing || existing.resetAt <= now) {
    buckets.set(key, { count: 1, resetAt: now + windowMs });
    return true;
  }
  if (existing.count >= max) return false;
  existing.count += 1;
  return true;
}

export function getClientIp(req: Request): string {
  const xff = req.headers.get("x-forwarded-for");
  if (xff) return xff.split(",")[0]!.trim();
  const real = req.headers.get("x-real-ip");
  if (real) return real.trim();
  return "unknown";
}

/** Strip anything that looks like a base58 pubkey from error messages. */
export function scrubError(message: string): string {
  return message
    .replace(/[1-9A-HJ-NP-Za-km-z]{32,}/g, "<pubkey>")
    .slice(0, 200);
}
