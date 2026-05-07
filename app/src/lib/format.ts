import BN from "bn.js";
import { microUsdcToHuman } from "@poolver/client";

export function fmtUSD(n: number): string {
  if (n >= 1_000_000)
    return `$${(n / 1_000_000).toFixed(n % 1_000_000 === 0 ? 0 : 2)}M`;
  if (n >= 1_000) return `$${(n / 1_000).toFixed(0)}K`;
  return `$${n.toLocaleString()}`;
}

export function fmtUSDC(microUSDC: BN | number | string): string {
  const bn = BN.isBN(microUSDC)
    ? (microUSDC as BN)
    : new BN(microUSDC.toString());
  const human = Number(microUsdcToHuman(bn));
  if (Number.isNaN(human)) return "$0";
  return fmtUSD(human);
}

export function fmtCountdown(secsRemaining: number): string {
  if (!Number.isFinite(secsRemaining)) return "—";
  if (secsRemaining <= 0) return "NOW";
  const d = Math.floor(secsRemaining / 86_400);
  const h = Math.floor((secsRemaining % 86_400) / 3_600);
  const m = Math.floor((secsRemaining % 3_600) / 60);
  if (d > 0) return `${d}d ${String(h).padStart(2, "0")}h`;
  if (h > 0) return `${h}h ${String(m).padStart(2, "0")}m`;
  return `${m}m`;
}
