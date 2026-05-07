export type {
  PoolView,
  PoolMonthState,
  ParticipantView,
  UserReputationView,
  ReserveFundView,
  TierName,
  KycLevelName,
} from "@poolver/client";

import type { PoolView } from "@poolver/client";

export type PoolDisplayStatus = "forming" | "active" | "completed";

export function derivePoolStatus(pool: PoolView): PoolDisplayStatus {
  if (pool.isComplete) return "completed";
  if (pool.currentMonth === 0) return "forming";
  return "active";
}
