import type { UserReputationView } from "../queries/reputation";

export type RepTier = "gray" | "green" | "yellow" | "red";

export interface RepTierInfo {
  tier: RepTier;
  /** A small label suitable for a badge or tooltip */
  label: string;
  /** Longer human description */
  description: string;
  /** Suggested CSS color (matches the app's design tokens). */
  color: string;
}

/**
 * Map a participant's lifetime reputation onto a 4-color tier.
 *
 *   gray   = new user with no completed/defaulted history (neutral)
 *   green  = proven good — has completed pools and never defaulted
 *   yellow = mixed — has defaulted at least once but ALSO completed pools
 *   red    = only-bad history — defaulted but never completed a pool
 *
 * Pass `null` for not-yet-initialized reputation accounts; result is gray.
 */
export function repTier(rep: UserReputationView | null | undefined): RepTierInfo {
  if (!rep) {
    return {
      tier: "gray",
      label: "New",
      description: "No reputation account yet",
      color: "var(--fg-4)",
    };
  }
  const completed = rep.poolsCompleted ?? 0;
  const defaulted = rep.poolsDefaulted ?? 0;

  if (defaulted === 0 && completed === 0) {
    return {
      tier: "gray",
      label: "Neutral",
      description: "No completed or defaulted pools yet",
      color: "var(--fg-4)",
    };
  }
  if (defaulted === 0 && completed > 0) {
    return {
      tier: "green",
      label: "Trusted",
      description: `Completed ${completed} pool${completed === 1 ? "" : "s"} · 0 defaults`,
      color: "var(--ok, #4ade80)",
    };
  }
  if (defaulted > 0 && completed > 0) {
    return {
      tier: "yellow",
      label: "Mixed",
      description: `${completed} completed · ${defaulted} defaulted`,
      color: "var(--warn, #facc15)",
    };
  }
  // defaulted > 0 && completed == 0
  return {
    tier: "red",
    label: "Risky",
    description: `${defaulted} default${defaulted === 1 ? "" : "s"} · 0 completed`,
    color: "var(--err, #ef4444)",
  };
}
