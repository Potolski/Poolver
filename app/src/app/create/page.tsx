"use client";

import { useMemo, useState, type ReactNode } from "react";
import { useRouter } from "next/navigation";
import { toast } from "sonner";
import BN from "bn.js";
import { useAppKit } from "@reown/appkit/react";
import {
  createPoolIx,
  humanUsdcToMicro,
  type TierName,
} from "@poolver/client";

import { PoolverMark } from "@/components/brand/PoolverLogo";
import { usePoolver } from "@/providers/PoolverProvider";
import { useOnboarding } from "@/hooks/useOnboarding";
import { sendIxs } from "@/lib/tx-helpers";
import { USDC_MINT_DEVNET_DEFAULT } from "@/lib/constants";

type DurationPreset = "2m" | "10m" | "1h" | "1d" | "30d";

interface PoolConfig {
  name: string;
  monthly: number;
  tier: TierName;
  duration: DurationPreset;
}

const DURATION_SECONDS: Record<DurationPreset, number> = {
  "2m": 120,
  "10m": 600,
  "1h": 3_600,
  "1d": 86_400,
  "30d": 30 * 86_400,
};

const DURATION_LABEL: Record<DurationPreset, string> = {
  "2m": "2 minutes (fast demo)",
  "10m": "10 minutes",
  "1h": "1 hour",
  "1d": "1 day",
  "30d": "30 days (default)",
};

function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <div className="create-field">
      <label>{label}</label>
      {hint && <div className="create-hint">{hint}</div>}
      {children}
    </div>
  );
}

function Kv({
  k,
  v,
  accent,
}: {
  k: string;
  v: string;
  accent?: boolean;
}) {
  return (
    <div className="review-kv-row">
      <span>{k}</span>
      <b style={{ color: accent ? "var(--acc)" : "var(--fg)" }}>{v}</b>
    </div>
  );
}

export default function CreatePage() {
  const router = useRouter();
  const { client, connected } = usePoolver();
  const {
    state: onboardingState,
    ensureReputation,
    ensureKyc,
  } = useOnboarding();
  const [onboardingBusy, setOnboardingBusy] = useState<
    "reputation" | "kyc" | null
  >(null);

  async function handleEnsureReputation() {
    setOnboardingBusy("reputation");
    const toastId = toast.loading("Initializing reputation account…");
    try {
      const sig = await ensureReputation();
      toast.success("Reputation initialized", {
        id: toastId,
        description: `sig: ${sig.slice(0, 12)}…`,
      });
    } catch (e) {
      toast.error("Failed", {
        id: toastId,
        description: (e instanceof Error ? e.message : String(e)).slice(0, 200),
      });
    } finally {
      setOnboardingBusy(null);
    }
  }

  async function handleEnsureKyc() {
    setOnboardingBusy("kyc");
    const toastId = toast.loading("Issuing demo KYC…");
    try {
      const { signature, idempotent } = await ensureKyc();
      toast.success(
        idempotent ? "KYC already issued" : "Demo KYC granted",
        {
          id: toastId,
          description: signature ? `sig: ${signature.slice(0, 12)}…` : undefined,
        }
      );
    } catch (e) {
      toast.error("KYC failed", {
        id: toastId,
        description: (e instanceof Error ? e.message : String(e)).slice(0, 200),
      });
    } finally {
      setOnboardingBusy(null);
    }
  }
  const { open } = useAppKit();
  const [step, setStep] = useState(1);
  const [deploying, setDeploying] = useState(false);
  const [cfg, setCfg] = useState<PoolConfig>({
    name: "",
    monthly: 1000,
    tier: "vault",
    duration: "30d",
  });

  const set = <K extends keyof PoolConfig>(k: K, v: PoolConfig[K]) =>
    setCfg((c) => ({ ...c, [k]: v }));

  const monthlyMicro = useMemo(() => humanUsdcToMicro(cfg.monthly), [cfg.monthly]);
  const lifetimePool = cfg.monthly * 12;
  const protocolFee = (cfg.monthly * 12) * 0.015;
  const reserveFee = cfg.monthly * 12 * (cfg.tier === "vault" ? 0.015 : 0.025);
  const netReceive = lifetimePool - protocolFee - reserveFee;

  async function launch() {
    if (!connected) {
      open();
      return;
    }
    if (onboardingState !== "ready") {
      toast.error("Complete onboarding first", {
        description:
          onboardingState === "needs_reputation"
            ? "Initialize your reputation account."
            : onboardingState === "needs_kyc"
              ? "Verify identity (demo KYC)."
              : "Loading onboarding state…",
      });
      return;
    }
    setDeploying(true);
    const toastId = toast.loading("Deploying pool…");
    try {
      const poolId = new BN(Date.now());
      const { ix, pool } = await createPoolIx(client, {
        poolId,
        tier: cfg.tier,
        contributionAmount: monthlyMicro,
        monthDurationSeconds: new BN(DURATION_SECONDS[cfg.duration]),
        usdcMint: USDC_MINT_DEVNET_DEFAULT,
      });
      const sig = await sendIxs(client, [ix]);
      toast.success("Pool deployed", {
        id: toastId,
        description: `sig: ${sig.slice(0, 12)}…`,
      });
      router.push(`/pool/${pool.toBase58()}`);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      toast.error("Deploy failed", { id: toastId, description: msg.slice(0, 200) });
    } finally {
      setDeploying(false);
    }
  }

  const onboardingBlocking =
    connected && onboardingState !== "ready" && onboardingState !== "loading";

  return (
    <div
      className="shell"
      style={{ padding: "48px 0 80px", position: "relative", zIndex: 1 }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 14,
          marginBottom: 8,
          fontFamily: "var(--mono)",
          fontSize: 11,
          color: "var(--fg-3)",
          letterSpacing: "0.1em",
        }}
      >
        <button className="btn ghost sm" onClick={() => router.push("/pools")}>
          ← Cancel
        </button>
        <span>CREATE / new-pool / step {step} of 4</span>
      </div>
      <h1
        className="hero-headline"
        style={{ fontSize: "clamp(36px, 4.4vw, 56px)", margin: "12px 0 8px" }}
      >
        Launch a <em>Poolver</em>.
      </h1>
      <p className="hero-deck" style={{ maxWidth: "62ch" }}>
        12 participants · 12 months · USDC. Configure the parameters; deploy as
        a new PDA on Solana. Anyone with the link can join once it&apos;s live.
      </p>

      <div className="create-steps">
        {["Basics", "Tier", "Demo", "Review"].map((lbl, i) => {
          const n = i + 1;
          const state = step === n ? "on" : step > n ? "done" : "";
          return (
            <div key={n} style={{ display: "contents" }}>
              <button
                className={`create-step ${state}`}
                onClick={() => step > n && setStep(n)}
                disabled={step <= n && step !== n}
              >
                <span className="n">{String(n).padStart(2, "0")}</span>
                <span className="l">{lbl}</span>
              </button>
              {n < 4 && <div className="create-step-sep">───</div>}
            </div>
          );
        })}
      </div>

      <div className="create-grid">
        <div className="create-body">
          {step === 1 && (
            <>
              <h3 className="create-h">Pool basics</h3>
              <div className="create-fields">
                <Field
                  label="Pool name"
                  hint="Local-only label (not stored on-chain)."
                >
                  <input
                    className="create-input"
                    placeholder="e.g. Frontend devs Q2"
                    value={cfg.name}
                    onChange={(e) => set("name", e.target.value)}
                  />
                </Field>
                <Field
                  label={`Monthly contribution — $${cfg.monthly.toLocaleString()}`}
                  hint="Each of 12 members pays this every month. 100 ≤ amount ≤ 10,000 USDC."
                >
                  <input
                    type="range"
                    min={100}
                    max={10000}
                    step={50}
                    value={cfg.monthly}
                    onChange={(e) => set("monthly", Number(e.target.value))}
                    className="create-slider"
                  />
                  <div className="seg" style={{ marginTop: 8 }}>
                    {[100, 500, 1000, 2500, 5000, 10000].map((v) => (
                      <button
                        key={v}
                        className={`seg-btn ${cfg.monthly === v ? "on" : ""}`}
                        onClick={() => set("monthly", v)}
                      >
                        ${v}
                      </button>
                    ))}
                  </div>
                </Field>
                <Field
                  label="Pool size"
                  hint="V1 protocol constant: 12 participants × 12 months. Not configurable."
                >
                  <div
                    style={{
                      fontFamily: "var(--mono)",
                      fontSize: 12,
                      color: "var(--fg-3)",
                      padding: "10px 12px",
                      border: "1px dashed var(--line-2)",
                      borderRadius: 3,
                    }}
                  >
                    12 participants × 12 months · enforced on-chain
                  </div>
                </Field>
              </div>
            </>
          )}

          {step === 2 && (
            <>
              <h3 className="create-h">Tier &amp; yield strategy</h3>
              <div className="create-fields">
                <Field
                  label="Tier"
                  hint="Vault: passive USDC custody, 1.5% reserve. DeFi: Kamino-mocked yield, 2.5% reserve. Choose Vault for the safest demo."
                >
                  <div className="seg">
                    <button
                      className={`seg-btn ${cfg.tier === "vault" ? "on" : ""}`}
                      onClick={() => set("tier", "vault")}
                    >
                      Tier 0 · Vault
                    </button>
                    <button
                      className={`seg-btn ${cfg.tier === "defi" ? "on" : ""}`}
                      onClick={() => set("tier", "defi")}
                    >
                      Tier 1 · DeFi (experimental)
                    </button>
                  </div>
                  <div
                    style={{
                      fontFamily: "var(--mono)",
                      fontSize: 11,
                      color: "var(--fg-3)",
                      marginTop: 8,
                      lineHeight: 1.6,
                    }}
                  >
                    {cfg.tier === "vault" ? (
                      <>
                        ◆ Selected:{" "}
                        <b style={{ color: "var(--acc)" }}>Tier 0 · Vault</b>{" "}
                        — passive USDC custody. Recommended for V1 demo.
                      </>
                    ) : (
                      <>
                        ⚠ Selected:{" "}
                        <b style={{ color: "var(--warn)" }}>
                          Tier 1 · DeFi
                        </b>{" "}
                        — Kamino integration is mocked in V1; yield arrives via{" "}
                        <code className="docs-code">mock_inject_yield</code>.
                      </>
                    )}
                  </div>
                </Field>
                <Field
                  label="Bidding"
                  hint="Sealed-bid commit-reveal auction is V1 baseline (not optional)."
                >
                  <div
                    style={{
                      fontFamily: "var(--mono)",
                      fontSize: 12,
                      color: "var(--fg-3)",
                      padding: "10px 12px",
                      border: "1px dashed var(--line-2)",
                      borderRadius: 3,
                    }}
                  >
                    ✓ Always on · 1% stake · 20% of net pot bid cap · INV-14
                  </div>
                </Field>
                <Field
                  label="Collateral"
                  hint="V1 uses reputation-graduated collateral computed on-chain (100/70/50% × baseline). Not configurable."
                >
                  <div
                    style={{
                      fontFamily: "var(--mono)",
                      fontSize: 12,
                      color: "var(--fg-3)",
                      padding: "10px 12px",
                      border: "1px dashed var(--line-2)",
                      borderRadius: 3,
                    }}
                  >
                    Reputation-graduated · enforced on-chain
                  </div>
                </Field>
              </div>
            </>
          )}

          {step === 3 && (
            <>
              <h3 className="create-h">Demo cadence</h3>
              <div className="create-fields">
                <Field
                  label="Month duration"
                  hint="Default = 30 days. Pick a shorter duration to demo a full cycle in minutes."
                >
                  <div className="seg">
                    {(["2m", "10m", "1h", "1d", "30d"] as const).map((d) => (
                      <button
                        key={d}
                        className={`seg-btn ${cfg.duration === d ? "on" : ""}`}
                        onClick={() => set("duration", d)}
                      >
                        {d}
                      </button>
                    ))}
                  </div>
                  <div
                    style={{
                      fontFamily: "var(--mono)",
                      fontSize: 11,
                      color: "var(--fg-3)",
                      marginTop: 8,
                    }}
                  >
                    Selected:{" "}
                    <b style={{ color: "var(--acc)" }}>
                      {DURATION_LABEL[cfg.duration]}
                    </b>{" "}
                    · {DURATION_SECONDS[cfg.duration].toLocaleString()} seconds
                  </div>
                </Field>
                <Field
                  label="poolId"
                  hint="Auto-generated from current timestamp. Used in the PDA derivation; doesn't need to be remembered."
                >
                  <div
                    style={{
                      fontFamily: "var(--mono)",
                      fontSize: 12,
                      color: "var(--fg-3)",
                      padding: "10px 12px",
                      border: "1px dashed var(--line-2)",
                      borderRadius: 3,
                    }}
                  >
                    poolId = Date.now() at deploy
                  </div>
                </Field>
              </div>
            </>
          )}

          {step === 4 && (
            <>
              <h3 className="create-h">Review &amp; deploy</h3>
              <div
                style={{
                  padding: 20,
                  border: "1px solid var(--line)",
                  borderRadius: 3,
                  background: "var(--bg-1)",
                }}
              >
                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 12,
                    marginBottom: 16,
                  }}
                >
                  <PoolverMark size={24} />
                  <div>
                    <div
                      style={{
                        fontFamily: "var(--mono)",
                        fontSize: 13,
                        color: "var(--fg)",
                      }}
                    >
                      {cfg.name || "PLVR-XXXX"}
                    </div>
                    <div
                      style={{
                        fontSize: 11,
                        color: "var(--fg-4)",
                        letterSpacing: "0.1em",
                        textTransform: "uppercase",
                        marginTop: 2,
                      }}
                    >
                      Solana · USDC · Tier {cfg.tier === "vault" ? "0" : "1"}
                    </div>
                  </div>
                </div>
                <div className="review-kv">
                  <Kv
                    k="Pool size (lifetime)"
                    v={`$${lifetimePool.toLocaleString()}`}
                  />
                  <Kv
                    k="Each member pays"
                    v={`$${cfg.monthly.toLocaleString()} × 12 = $${(cfg.monthly * 12).toLocaleString()}`}
                  />
                  <Kv
                    k="Each member receives"
                    v={`$${Math.round(netReceive).toLocaleString()} (gross − fees)`}
                    accent
                  />
                  <Kv
                    k="Tier"
                    v={cfg.tier === "vault" ? "Tier 0 · Vault" : "Tier 1 · DeFi"}
                  />
                  <Kv k="Month duration" v={DURATION_LABEL[cfg.duration]} />
                  <Kv k="Protocol fee" v="1.50% per month" />
                  <Kv
                    k="Reserve fee"
                    v={cfg.tier === "vault" ? "1.50%" : "2.50%"}
                  />
                  <Kv k="Bid cap" v="20% of net pot" />
                  <Kv k="Bid stake" v="1% of contribution (refundable)" />
                </div>
              </div>

              <div className="docs-callout" style={{ marginTop: 20 }}>
                <div className="docs-callout-title">◆ Deploy sequence</div>
                <div
                  style={{
                    fontFamily: "var(--mono)",
                    fontSize: 12,
                    color: "var(--fg-2)",
                    lineHeight: 1.7,
                  }}
                >
                  1. Sign{" "}
                  <code className="docs-code">create_pool</code> via wallet
                  <br />
                  2. Pool PDA derived from{" "}
                  <code className="docs-code">[b&quot;pool&quot;, creator, poolId]</code>
                  <br />
                  3. Pool appears in /pools with status{" "}
                  <span style={{ color: "var(--warn)" }}>FORMING</span>
                  <br />
                  4. Auto-starts month 1 when 12/12 wallets join
                </div>
              </div>

              {onboardingBlocking && (
                <div
                  style={{
                    marginTop: 20,
                    padding: 14,
                    border: "1px solid var(--warn)",
                    borderRadius: 3,
                    background: "var(--bg-1)",
                    fontFamily: "var(--mono)",
                    fontSize: 12,
                    color: "var(--warn)",
                    display: "flex",
                    flexDirection: "column",
                    gap: 10,
                  }}
                >
                  <span>
                    ⚠ One step before deploy:{" "}
                    {onboardingState === "needs_reputation"
                      ? "initialize your reputation account."
                      : "verify identity (demo KYC)."}
                  </span>
                  {onboardingState === "needs_reputation" && (
                    <button
                      className="btn primary"
                      onClick={handleEnsureReputation}
                      disabled={onboardingBusy !== null}
                      style={{ alignSelf: "flex-start" }}
                    >
                      {onboardingBusy === "reputation"
                        ? "Signing…"
                        : "▶ Initialize account"}
                    </button>
                  )}
                  {onboardingState === "needs_kyc" && (
                    <button
                      className="btn primary"
                      onClick={handleEnsureKyc}
                      disabled={onboardingBusy !== null}
                      style={{ alignSelf: "flex-start" }}
                    >
                      {onboardingBusy === "kyc"
                        ? "Issuing…"
                        : "▶ Verify (demo KYC)"}
                    </button>
                  )}
                </div>
              )}
            </>
          )}

          <div className="create-nav">
            {step > 1 && (
              <button className="btn lg" onClick={() => setStep(step - 1)}>
                ← Back
              </button>
            )}
            <div style={{ flex: 1 }} />
            {step < 4 && (
              <button
                className="btn primary lg"
                onClick={() => setStep(step + 1)}
              >
                Continue →
              </button>
            )}
            {step === 4 && (
              <button
                className="btn primary lg"
                onClick={launch}
                disabled={deploying}
              >
                {deploying
                  ? "Deploying…"
                  : connected
                    ? "▶ Deploy pool"
                    : "▶ Connect wallet to deploy"}
              </button>
            )}
          </div>
        </div>

        <aside className="create-summary">
          <div
            style={{
              fontFamily: "var(--mono)",
              fontSize: 10,
              color: "var(--fg-4)",
              letterSpacing: "0.16em",
              textTransform: "uppercase",
              marginBottom: 12,
              display: "flex",
              alignItems: "center",
              gap: 6,
            }}
          >
            <PoolverMark size={11} /> Live summary
          </div>
          <div className="summary-headline">
            ${lifetimePool.toLocaleString()}
          </div>
          <div className="summary-sub">
            lifetime · 12 members × 12 months
          </div>
          <hr className="rule-dashed" style={{ margin: "16px 0" }} />
          <div className="summary-kv">
            <div>
              <span>Monthly</span>
              <b>${cfg.monthly.toLocaleString()}</b>
            </div>
            <div>
              <span>Tier</span>
              <b>{cfg.tier === "vault" ? "Vault" : "DeFi"}</b>
            </div>
            <div>
              <span>Month dur.</span>
              <b>{cfg.duration}</b>
            </div>
            <div>
              <span>Protocol fee</span>
              <b>1.5%</b>
            </div>
            <div>
              <span>Reserve fee</span>
              <b>{cfg.tier === "vault" ? "1.5%" : "2.5%"}</b>
            </div>
            <div>
              <span>Bid cap</span>
              <b>20%</b>
            </div>
          </div>
          <hr className="rule-dashed" style={{ margin: "16px 0" }} />
          <div
            style={{
              fontFamily: "var(--mono)",
              fontSize: 10,
              color: "var(--fg-4)",
              letterSpacing: "0.14em",
              marginBottom: 6,
            }}
          >
            NET PER WINNER
          </div>
          <div
            style={{
              fontFamily: "var(--display)",
              fontSize: 20,
              color: "var(--acc)",
            }}
          >
            ${Math.round(netReceive).toLocaleString()}
          </div>
          <div
            style={{
              fontFamily: "var(--mono)",
              fontSize: 10.5,
              color: "var(--fg-3)",
              marginTop: 4,
            }}
          >
            gross pool − fees · before bids
          </div>
        </aside>
      </div>
    </div>
  );
}
