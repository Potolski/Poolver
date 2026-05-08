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
 *   gray   = new user with no history (neutral)
 *   green  = proven good — completed pools, no slashes, no defaults
 *   yellow = soft warning — has been slashed for missed months OR
 *            mixed completed/defaulted history
 *   red    = full default — only-bad history, OR slashed across many
 *            months without any completed pool
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
  const missed = rep.monthsMissedLifetime ?? 0;

  // Full default takes precedence — only-bad history.
  if (defaulted > 0 && completed === 0) {
    return {
      tier: "red",
      label: "Risky",
      description: `${defaulted} default${defaulted === 1 ? "" : "s"} · 0 completed`,
      color: "var(--err, #ef4444)",
    };
  }
  // Many missed months and no completed pools → red.
  if (missed >= 4 && completed === 0) {
    return {
      tier: "red",
      label: "Risky",
      description: `Slashed for ${missed} missed month${missed === 1 ? "" : "s"} · 0 completed`,
      color: "var(--err, #ef4444)",
    };
  }
  // Mixed history (defaults but also completed pools) OR any slashes.
  if (defaulted > 0 || missed > 0) {
    return {
      tier: "yellow",
      label: "Mixed",
      description:
        defaulted > 0
          ? `${completed} completed · ${defaulted} defaulted · ${missed} missed`
          : `${completed} completed · ${missed} missed month${missed === 1 ? "" : "s"}`,
      color: "var(--warn, #facc15)",
    };
  }
  // Clean record with completed pools.
  if (completed > 0) {
    return {
      tier: "green",
      label: "Trusted",
      description: `Completed ${completed} pool${completed === 1 ? "" : "s"} · 0 defaults · 0 missed`,
      color: "var(--ok, #4ade80)",
    };
  }
  // Brand new — no completed, no defaulted, no missed.
  return {
    tier: "gray",
    label: "Neutral",
    description: "No completed or defaulted pools yet",
    color: "var(--fg-4)",
  };
}
