"use client";

import { useEffect, useState } from "react";
import BN from "bn.js";
import { PublicKey } from "@solana/web3.js";
import {
  fetchUserReputation,
  microUsdcToHuman,
  type PoolView,
  type UserReputationView,
} from "@poolver/client";

import { SectionHead } from "@/components/layout/SectionHead";
import { usePoolver } from "@/providers/PoolverProvider";
import { truncateAddress } from "@/lib/utils";

interface RosterRow {
  pda: PublicKey;
  user: PublicKey;
  paidMonthsBitmap: number;
  isDefaulted: boolean;
  isSuspended: boolean;
  hasWon: boolean;
  winMonth: number;
  /** Currently locked collateral (decreases as winner pays each post-win month). */
  collateralLocked: BN;
  /** Initial collateral posted at win time — used for tooltip / display continuity. */
  collateralInitial: BN;
}

function decodeRow(
  pda: PublicKey,
  raw: Record<string, unknown>
): RosterRow {
  return {
    pda,
    user: raw.user as PublicKey,
    paidMonthsBitmap: (raw.paidMonths as number) ?? 0,
    isDefaulted: Boolean(raw.isDefaulted),
    isSuspended: Boolean(raw.isSuspended),
    hasWon: Boolean(raw.hasWon),
    winMonth: (raw.winMonth as number) ?? 0,
    collateralLocked: (raw.collateralLocked as BN) ?? new BN(0),
    collateralInitial: (raw.collateralInitial as BN) ?? new BN(0),
  };
}

interface AccountAllResult {
  publicKey: PublicKey;
  account: Record<string, unknown>;
}

interface ParticipantAccountClient {
  all: (filters?: unknown[]) => Promise<AccountAllResult[]>;
}

function PaidMonthsBar({ bitmap, totalMonths }: { bitmap: number; totalMonths: number }) {
  const cells = Array.from({ length: totalMonths }, (_, i) => i + 1);
  return (
    <div style={{ display: "inline-flex", gap: 2 }}>
      {cells.map((m) => {
        const paid = (bitmap & (1 << (m - 1))) !== 0;
        return (
          <span
            key={m}
            title={`M${String(m).padStart(2, "0")} ${paid ? "paid" : "unpaid"}`}
            style={{
              width: 8,
              height: 12,
              borderRadius: 1,
              background: paid ? "var(--acc)" : "var(--bg-3)",
              border: paid ? "none" : "1px solid var(--line)",
            }}
          />
        );
      })}
    </div>
  );
}

export function ParticipantRoster({ pool }: { pool: PoolView }) {
  const { client, publicKey } = usePoolver();
  const [rows, setRows] = useState<RosterRow[] | null>(null);
  const [reps, setReps] = useState<Map<string, UserReputationView | null>>(
    new Map()
  );
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    let cancelled = false;
    const accountClient = (
      client.core.account as unknown as { participant: ParticipantAccountClient }
    ).participant;

    const tick = async () => {
      try {
        const accounts = await accountClient.all([
          {
            memcmp: {
              offset: 8,
              bytes: pool.publicKey.toBase58(),
            },
          },
        ]);
        if (cancelled) return;
        const decoded = accounts.map(({ publicKey: pda, account }) =>
          decodeRow(pda, account)
        );
        setRows(decoded);

        // Fetch each participant's reputation in parallel — one PDA each.
        const pairs = await Promise.all(
          decoded.map(async (r) => {
            try {
              const rep = await fetchUserReputation(client, r.user);
              return [r.user.toBase58(), rep] as const;
            } catch {
              return [r.user.toBase58(), null] as const;
            }
          })
        );
        if (cancelled) return;
        setReps(new Map(pairs));
      } catch (e) {
        if (cancelled) return;
        setError(e instanceof Error ? e : new Error(String(e)));
        setRows([]);
      }
    };

    void tick();
    const id = setInterval(tick, 10_000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, [client, pool.publicKey]);

  return (
    <section className="shell section">
      <SectionHead
        n="02"
        title="Pool <em>Roster</em>"
        meta={`${rows?.length ?? "…"} of 12 participants`}
      />
      <div
        style={{
          border: "1px solid var(--line)",
          borderRadius: 3,
          overflow: "auto",
        }}
      >
        <table className="roster">
          <thead>
            <tr>
              <th></th>
              <th>Wallet</th>
              <th
                title="Reputation. Format: J·C·D — pools joined / completed / defaulted (lifetime)."
              >
                Rep
              </th>
              <th className="num" title="Collateral locked. Everyone posts 12× contribution at join (fully refundable on completion). Winners post additional collateral when they claim.">
                Collateral
              </th>
              <th>Paid months</th>
              <th>Status</th>
              <th className="num">Won</th>
            </tr>
          </thead>
          <tbody>
            {error && (
              <tr>
                <td colSpan={7} style={{ padding: 14, color: "var(--err)" }}>
                  {error.message}
                </td>
              </tr>
            )}
            {!rows && !error && (
              <tr>
                <td
                  colSpan={7}
                  style={{
                    padding: 14,
                    textAlign: "center",
                    color: "var(--fg-4)",
                    fontFamily: "var(--mono)",
                    fontSize: 12,
                  }}
                >
                  Loading participants…
                </td>
              </tr>
            )}
            {rows?.length === 0 && (
              <tr>
                <td
                  colSpan={7}
                  style={{
                    padding: 14,
                    textAlign: "center",
                    color: "var(--fg-4)",
                    fontFamily: "var(--mono)",
                    fontSize: 12,
                  }}
                >
                  No participants yet — be the first to join.
                </td>
              </tr>
            )}
            {rows?.map((r, i) => {
              const me = publicKey?.toBase58() === r.user.toBase58();
              const collateralHuman = Number(microUsdcToHuman(r.collateralLocked));
              const collateralInitialHuman = Number(microUsdcToHuman(r.collateralInitial));
              const rep = reps.get(r.user.toBase58()) ?? null;
              const repCompleted = rep?.poolsCompleted ?? 0;
              const repDefaulted = rep?.poolsDefaulted ?? 0;
              const repJoined = rep?.poolsJoined ?? 0;
              const status = r.isDefaulted
                ? { label: "Default", cls: "default" }
                : r.isSuspended
                  ? { label: "Suspended", cls: "default" }
                  : r.hasWon
                    ? { label: "Received", cls: "received" }
                    : { label: "Active", cls: "eligible" };
              return (
                <tr key={r.pda.toBase58()} className={me ? "you-row" : ""}>
                  <td className="i-cell">{String(i + 1).padStart(2, "0")}</td>
                  <td>
                    <div className="wallet-cell">
                      <div className="avatar">
                        {r.user.toBase58().slice(0, 2).toUpperCase()}
                      </div>
                      <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
                        <span className="name">
                          {truncateAddress(r.user.toBase58(), 5)}
                        </span>
                      </div>
                    </div>
                  </td>
                  <td
                    title={
                      rep
                        ? `Joined ${repJoined} pool${repJoined === 1 ? "" : "s"} · ${repCompleted} completed · ${repDefaulted} defaulted`
                        : "Reputation account not initialized"
                    }
                    style={{
                      fontFamily: "var(--mono)",
                      fontSize: 11.5,
                      fontVariantNumeric: "tabular-nums",
                    }}
                  >
                    {rep ? (
                      <span style={{ display: "inline-flex", gap: 4 }}>
                        <span style={{ color: "var(--fg)" }}>{repJoined}</span>
                        <span style={{ color: "var(--fg-4)" }}>·</span>
                        <span style={{ color: "var(--ok, var(--acc))" }}>
                          {repCompleted}
                        </span>
                        <span style={{ color: "var(--fg-4)" }}>·</span>
                        <span
                          style={{
                            color:
                              repDefaulted > 0 ? "var(--err)" : "var(--fg-4)",
                            fontWeight: repDefaulted > 0 ? 600 : 400,
                          }}
                        >
                          {repDefaulted}
                        </span>
                      </span>
                    ) : (
                      <span style={{ color: "var(--fg-4)" }}>—</span>
                    )}
                  </td>
                  <td
                    className="num"
                    title={`Initial: $${collateralInitialHuman.toLocaleString()} · Locked now: $${collateralHuman.toLocaleString()}`}
                  >
                    ${collateralHuman.toLocaleString()}
                  </td>
                  <td>
                    <PaidMonthsBar
                      bitmap={r.paidMonthsBitmap}
                      totalMonths={pool.totalMonths}
                    />
                  </td>
                  <td>
                    {me ? (
                      <>
                        <span className="badge you">You</span>{" "}
                        <span className={`badge ${status.cls}`} style={{ marginLeft: 4 }}>
                          {status.label}
                        </span>
                      </>
                    ) : (
                      <span className={`badge ${status.cls}`}>{status.label}</span>
                    )}
                  </td>
                  <td className="num">
                    {r.hasWon
                      ? `M${String(r.winMonth).padStart(2, "0")}`
                      : "—"}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </section>
  );
}
