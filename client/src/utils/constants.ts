/**
 * Re-export of cross-cutting client constants (USDC mint and decimals only;
 * for protocol PDA seeds and program IDs, import from `../constants`).
 */
export {
  USDC_DECIMALS,
  USDC_MINT_DEVNET_DEFAULT,
  USDC_MINT_MAINNET,
} from "../constants";

import BN from "bn.js";

/** Convert a human USDC amount (e.g. `12.34`) to microUSDC (`12_340_000`). */
export function humanUsdcToMicro(amount: number | string): BN {
  const s = typeof amount === "number" ? amount.toString() : amount;
  if (!/^\d+(\.\d+)?$/.test(s)) {
    throw new Error(`invalid USDC amount: ${s}`);
  }
  const [whole, frac = ""] = s.split(".");
  const padded = (frac + "000000").slice(0, 6);
  return new BN(whole).muln(1_000_000).add(new BN(padded || "0"));
}

/** Format microUSDC as a human string with 6-decimal precision. */
export function microUsdcToHuman(micro: BN): string {
  const s = micro.toString().padStart(7, "0");
  const whole = s.slice(0, -6);
  const frac = s.slice(-6).replace(/0+$/, "");
  return frac ? `${whole}.${frac}` : whole;
}
