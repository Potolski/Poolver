"use client";

import { useEffect, useState, type ReactNode } from "react";
import { PoolverMark } from "@/components/brand/PoolverLogo";

interface Section {
  id: string;
  t: string;
}

const SECTIONS: Section[] = [
  { id: "01", t: "What Poolver is" },
  { id: "02", t: "Lifecycle" },
  { id: "03", t: "What you pay, when" },
  { id: "04", t: "The draw (VRF)" },
  { id: "05", t: "Bidding (Lance)" },
  { id: "06", t: "Post-win enforcement" },
  { id: "07", t: "Protocol economics" },
  { id: "08", t: "Wallet reputation" },
  { id: "09", t: "Risks & disclaimers" },
];

function DocSection({
  id,
  n,
  title,
  children,
}: {
  id: string;
  n: string;
  title: string;
  children: ReactNode;
}) {
  return (
    <section id={id} className="doc-section">
      <div className="doc-section-head">
        <span className="section-num">
          <PoolverMark size={12} />
          {n}
        </span>
        <h2>{title}</h2>
      </div>
      {children}
    </section>
  );
}

function Callout({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="docs-callout">
      <div className="docs-callout-title">◆ {title}</div>
      <div>{children}</div>
    </div>
  );
}

function Ascii({ children }: { children: ReactNode }) {
  return <pre className="docs-ascii">{children}</pre>;
}

function Code({ children }: { children: ReactNode }) {
  return <code className="docs-code">{children}</code>;
}

export default function DocsPage() {
  const [active, setActive] = useState("01");

  useEffect(() => {
    const obs = new IntersectionObserver(
      (entries) => {
        entries.forEach((e) => {
          if (e.isIntersecting) setActive(e.target.id);
        });
      },
      { rootMargin: "-20% 0px -70% 0px" }
    );
    SECTIONS.forEach((s) => {
      const el = document.getElementById(s.id);
      if (el) obs.observe(el);
    });
    return () => obs.disconnect();
  }, []);

  const jump = (id: string) => {
    const el = document.getElementById(id);
    if (el) {
      window.scrollTo({
        top: el.getBoundingClientRect().top + window.scrollY - 80,
        behavior: "smooth",
      });
    }
  };

  return (
    <div className="shell docs-shell">
      <section
        style={{
          padding: "48px 0 24px",
          borderBottom: "1px solid var(--line)",
          marginBottom: 40,
        }}
      >
        <div className="hero-kicker">
          <span className="sq" />
          PROTOCOL DOCUMENTATION · v0.1.0
        </div>
        <h1
          className="hero-headline"
          style={{ fontSize: "clamp(40px, 5vw, 68px)", margin: "16px 0 14px" }}
        >
          How <em>Poolver</em> works.
        </h1>
        <p className="hero-deck" style={{ maxWidth: "64ch" }}>
          A programmatic rotating savings protocol. Read once; save over and
          over.
        </p>
      </section>

      <div className="docs-grid">
        <aside className="docs-toc">
          <div
            style={{
              fontSize: 10,
              color: "var(--fg-4)",
              letterSpacing: "0.18em",
              textTransform: "uppercase",
              marginBottom: 14,
              display: "flex",
              alignItems: "center",
              gap: 6,
            }}
          >
            <PoolverMark size={11} /> Contents
          </div>
          {SECTIONS.map((s) => (
            <button
              key={s.id}
              className={`toc-link ${active === s.id ? "on" : ""}`}
              onClick={() => jump(s.id)}
            >
              <span className="toc-n">{s.id}</span>
              {s.t}
            </button>
          ))}
          <div
            style={{
              marginTop: 20,
              padding: 12,
              border: "1px dashed var(--line-2)",
              borderRadius: 2,
              fontSize: 10.5,
              color: "var(--fg-3)",
              fontFamily: "var(--mono)",
              lineHeight: 1.5,
            }}
          >
            PROGRAM_ID
            <br />
            <span style={{ color: "var(--fg-2)" }}>5x7Kq9FrA…Vault</span>
            <br />
            IDL v0.1.0 · devnet
          </div>
        </aside>

        <article className="docs-body">
          <DocSection id="01" n="01" title="What Poolver is">
            <p>
              Poolver is an on-chain implementation of the <b>ROSCA</b> —
              rotating savings and credit association. Known in Brazil as{" "}
              <i>consórcio</i>, in Kenya as <i>chama</i>, in West Africa as{" "}
              <i>tontine</i>, in the Caribbean as <i>susu</i>. Globally, the
              informal market is estimated at over $500B annually.
            </p>
            <p>
              The mechanism is simple: <b>N wallets</b> commit to contributing
              a fixed amount in USDC every round. Each round, one wallet
              receives the entire pool. After N rounds, everyone has paid N and
              received 1 full pool. No interest; rotation of liquidity, not
              creation of it.
            </p>
            <p>
              Poolver replaces the village elder, the office administrator, and
              the broker with a Solana program. No administrator. No custodian.
              No permission. The code is the contract.
            </p>
            <Callout title="Why on-chain">
              Traditional ROSCAs are limited by trust — you must know your
              group. Poolver replaces social trust with{" "}
              <b>collateral + verifiable reputation</b>, so strangers across the
              world can pool savings safely.
            </Callout>
          </DocSection>

          <DocSection id="02" n="02" title="Lifecycle">
            <Ascii>
{`┌─ JOIN ───┐  ┌─ PAY ───┐  ┌─ DRAW ──┐  ┌─ SETTLE ─┐  ┌─ EXIT ──┐
│ deposit  │→ │ monthly │→ │   VRF   │→ │  tranche │→ │ reclaim │
│ collat.  │  │ 7-day   │  │  commit │  │  release │  │  collat │
│ + first  │  │ window  │  │  reveal │  │  to wnr  │  │ + rep   │
└──────────┘  └─────────┘  └─────────┘  └──────────┘  └─────────┘`}
            </Ascii>
            <ol className="docs-ol">
              <li>
                <b>JOIN.</b> Pool opens with a target size (e.g. 20 members ×
                $2,500 = $50K/round). You deposit <b>25% collateral</b> +{" "}
                <b>first month&apos;s contribution</b>. Your slot activates
                when the pool fills.
              </li>
              <li>
                <b>PAY.</b> Each round has a 7-day payment window.
                Contributions route to the pool PDA automatically if
                you&apos;ve granted a payment authority; otherwise you sign.
                Missing the window moves you to &quot;late&quot; (day 8–14)
                and eventually &quot;default&quot; (day 15+).
              </li>
              <li>
                <b>DRAW.</b> Once contributions are collected, Switchboard VRF
                selects the round&apos;s recipient from wallets that have not
                yet received and are in good standing.
              </li>
              <li>
                <b>SETTLE.</b> The recipient receives the pool in tranches (see
                §06) minus the 1.5% protocol fee.
              </li>
              <li>
                <b>EXIT.</b> On pool completion, collateral unlocks. Reputation
                accrues.
              </li>
            </ol>
          </DocSection>

          <DocSection id="03" n="03" title="What you pay, when">
            <p>
              A worked example. Pool <Code>PLVR-4A9F</Code>: 20 members ×
              $2,500/mo × 20 rounds. Collateral ratio 25%.
            </p>

            <div className="docs-table-wrap">
              <table className="docs-table">
                <thead>
                  <tr>
                    <th>Phase</th>
                    <th>When</th>
                    <th className="num">Amount</th>
                    <th>Destination</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td>Collateral deposit</td>
                    <td>T0 (join)</td>
                    <td className="num">$625</td>
                    <td>collateral_pda (locked)</td>
                  </tr>
                  <tr>
                    <td>First monthly contribution</td>
                    <td>T0 (join)</td>
                    <td className="num">$2,500</td>
                    <td>pool_pda</td>
                  </tr>
                  <tr>
                    <td>Monthly contribution × 19</td>
                    <td>Each round, 7-day window</td>
                    <td className="num">$2,500</td>
                    <td>pool_pda</td>
                  </tr>
                  <tr>
                    <td>Insurance allocation (auto)</td>
                    <td>From each contribution</td>
                    <td className="num">−$125 (5%)</td>
                    <td>insurance_pda</td>
                  </tr>
                  <tr>
                    <td>Optional bid</td>
                    <td>Before draw, round 2+</td>
                    <td className="num">$0–$X</td>
                    <td>pool_pda (reduces future obligations)</td>
                  </tr>
                </tbody>
              </table>
            </div>

            <h4>What you receive</h4>
            <div className="docs-table-wrap">
              <table className="docs-table">
                <thead>
                  <tr>
                    <th>Round won</th>
                    <th>Pool</th>
                    <th>Fee (1.5%)</th>
                    <th>Net</th>
                    <th>Release</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td>Any</td>
                    <td className="num">$50,000</td>
                    <td className="num">−$750</td>
                    <td className="num">
                      <b style={{ color: "var(--acc)" }}>$49,250</b>
                    </td>
                    <td>Tranched — see §06</td>
                  </tr>
                </tbody>
              </table>
            </div>

            <Callout title="Total cost over 20 rounds">
              You pay <b>$2,500 × 20 = $50,000</b> in contributions. You
              receive <b>$49,250</b>. Collateral ($625) returns on pool
              completion. Net cost = 1.5% protocol fee. If you bid, your bid
              replaces some of your own future contributions.
            </Callout>
          </DocSection>

          <DocSection id="04" n="04" title="The draw (VRF)">
            <p>
              The recipient is chosen by <b>Switchboard VRF</b>, a verifiable
              random function oracle. Selection is a <b>commit-reveal</b>{" "}
              sequence: the oracle publishes a commitment in block{" "}
              <Code>N</Code> and the random seed in a later block. The seed is
              signed by the oracle&apos;s Ed25519 key; anyone can verify the
              proof against the oracle&apos;s on-chain pubkey.
            </p>
            <Ascii>
{`request → commit(hash)  ...  reveal(seed, σ)  →  select(eligible, seed)
   │           │                   │                    │
 user        slot N            slot N+K            slot N+K (atomic)`}
            </Ascii>
            <p>
              <b>Eligibility:</b> any wallet in the pool that (a) has not yet
              received, (b) is current on payments, (c) is not in slashing
              penalty. Defaulted wallets are excluded. In early rounds all
              non-winners are eligible equally; later rounds narrow the set.
            </p>
            <p>
              <b>Front-running:</b> impossible. The seed does not exist until
              the oracle reveals, and by that slot the eligible-set is already
              committed on-chain.
            </p>
          </DocSection>

          <DocSection id="05" n="05" title="Bidding (Lance)">
            <p>
              From round 2 onwards a second disbursement slot is auctioned.
              Bidding lets you <b>buy priority</b> if you need the liquidity
              sooner.
            </p>
            <ol className="docs-ol">
              <li>
                During a round&apos;s collection window, any eligible wallet
                may submit a bid in USDC, on top of their monthly contribution.
              </li>
              <li>
                At draw time: <b>Track A</b> (main) is selected by VRF.{" "}
                <b>Track B</b> (auction) goes to the highest bidder.
              </li>
              <li>
                The winning bid is added to the pool (benefiting everyone). In
                return, the bidder&apos;s <b>future monthly contributions are
                reduced</b> by the bid amount, spread over remaining rounds.
              </li>
              <li>
                Ties break on: (1) higher wallet reputation, (2) earlier
                timestamp.
              </li>
            </ol>
            <Callout title="Why bid?">
              If you need $49K for a deposit <i>now</i>, bidding $3,000 gets
              you access this round. You still pay the same total nominal
              contributions — you just front-load into a bid instead of
              back-loading into 10 more months.
            </Callout>
            <p className="dim" style={{ fontSize: 11.5 }}>
              Bids are binding. If you win and default, insurance pays the
              Poolver and you are slashed per §06.
            </p>
          </DocSection>

          <DocSection id="06" n="06" title="Post-win enforcement">
            <p>
              The hardest problem in a ROSCA: once you&apos;ve received the
              pool, what stops you from walking away? Poolver stacks four
              mechanisms.
            </p>

            <h4>① Collateral lock</h4>
            <p>
              The 25% collateral you deposit at join is <b>locked until pool
              completion</b>, not until you win. If you stop paying after
              winning, your collateral is slashed in favour of the Poolver. For
              our example pool the collateral is $625 — one month&apos;s worth
              of &quot;skin in the game&quot; from every member.
            </p>

            <h4>② Tranche release</h4>
            <p>The winnings do <b>not</b> drop in one lump. They release in three parts:</p>
            <div className="docs-table-wrap">
              <table className="docs-table">
                <thead>
                  <tr>
                    <th>Tranche</th>
                    <th>Condition</th>
                    <th className="num">Share</th>
                  </tr>
                </thead>
                <tbody>
                  <tr><td>1</td><td>At draw (immediate)</td><td className="num">50%</td></tr>
                  <tr><td>2</td><td>+3 months of on-time contributions</td><td className="num">25%</td></tr>
                  <tr><td>3</td><td>+6 months of on-time contributions</td><td className="num">25%</td></tr>
                </tbody>
              </table>
            </div>
            <p>
              Miss a post-win payment and the remaining tranches are forfeited
              to the insurance reserve. You still owe monthly contributions —
              the forfeit does not cancel the debt.
            </p>

            <h4>③ Insurance waterfall</h4>
            <p>
              5% of every contribution accrues to an{" "}
              <Code>insurance_pda</Code>. If a member defaults, the waterfall
              is: (a) slashed collateral → Poolver, (b) forfeited tranches →
              Poolver, (c) insurance reserve → Poolver to make it whole. The
              Poolver is guaranteed against single-member default in virtually
              every configuration.
            </p>

            <h4>④ Reputation slashing</h4>
            <p>
              Defaults are the <i>single largest</i> negative signal in the
              wallet reputation score. A default can permanently drop a wallet
              below the threshold required to join any future Poolver.
              Because the score is on-chain and non-transferable, a new wallet
              starts from 0 — so the cost of defaulting is structural, not just
              financial.
            </p>

            <Ascii>
{`        ENFORCEMENT STACK
        ─────────────────
        ┌─ reputation ─┐   ← permanent, on-chain
        ├─ insurance ──┤   ← Poolver kept whole
        ├─ tranche ────┤   ← leverage over time
        └─ collateral ─┘   ← locked at join

           DEFAULT COST ≈ collateral + forfeited tranches + rep wipeout`}
            </Ascii>

            <Callout title="The honest caveat">
              Smart contracts cannot force a wallet to send USDC — we can only
              make it <b>economically irrational</b> not to. Past some
              combination of winnings already received + collateral forfeit, a
              determined defector can still walk. The goal is to make that
              point land at round 18 of 20 with 90%+ already settled, not at
              round 7. The tranche schedule is tuned for this.
            </Callout>
          </DocSection>

          <DocSection id="07" n="07" title="Protocol economics">
            <div className="docs-table-wrap">
              <table className="docs-table">
                <thead>
                  <tr>
                    <th>Line</th>
                    <th>Rate</th>
                    <th>Destination</th>
                  </tr>
                </thead>
                <tbody>
                  <tr><td>Protocol fee</td><td className="num">1.5%</td><td>Deducted from pool at draw → treasury</td></tr>
                  <tr><td>Insurance allocation</td><td className="num">5.0%</td><td>Deducted from each contribution → insurance_pda</td></tr>
                  <tr><td>Oracle cost (VRF)</td><td className="num">~0.002 SOL</td><td>Switchboard · paid from treasury</td></tr>
                  <tr><td>Network fee</td><td className="num">~0.00015 SOL / tx</td><td>Paid by signer</td></tr>
                </tbody>
              </table>
            </div>
            <p>
              The treasury is governed by protocol token holders (future).
              Until then it&apos;s a multisig. Surplus insurance (if a pool
              completes with no defaults) rolls forward; deficits are topped up
              from treasury.
            </p>
          </DocSection>

          <DocSection id="08" n="08" title="Wallet reputation">
            <p>
              Every Solana wallet that has ever interacted with Poolver has an
              on-chain <Code>UserReputation</Code> account — a non-transferable
              record of how that wallet has behaved across every pool. It&apos;s
              read directly from the chain (no oracle, no off-chain database)
              and surfaced everywhere a wallet is shown: the topbar of the user
              themselves, and the &quot;Rep&quot; column of every pool roster
              so you can see who you&apos;re joining a pool with.
            </p>

            <h4>What gets tracked</h4>
            <div className="docs-table-wrap">
              <table className="docs-table">
                <thead>
                  <tr>
                    <th>Field</th>
                    <th>What it counts</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td><Code>pools_joined</Code></td>
                    <td>Total pools the wallet has ever joined</td>
                  </tr>
                  <tr>
                    <td><Code>pools_completed</Code></td>
                    <td>Pools that ran the full 12 months without this wallet defaulting</td>
                  </tr>
                  <tr>
                    <td><Code>pools_defaulted</Code></td>
                    <td>Pools where this wallet was liquidated for non-payment</td>
                  </tr>
                  <tr>
                    <td><Code>total_contributed_lifetime</Code></td>
                    <td>Lifetime USDC paid in across all pools</td>
                  </tr>
                  <tr>
                    <td><Code>total_received_lifetime</Code></td>
                    <td>Lifetime USDC received from winning</td>
                  </tr>
                </tbody>
              </table>
            </div>

            <h4>The four-color tier system</h4>
            <p>
              The full numeric history is reduced to a single color so a glance
              at a roster row tells you whether the counterparty is safe. Rules:
            </p>
            <div className="docs-table-wrap">
              <table className="docs-table">
                <thead>
                  <tr>
                    <th>Color</th>
                    <th>Label</th>
                    <th>Condition</th>
                    <th>Meaning</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td>
                      <span
                        style={{
                          display: "inline-block",
                          width: 12,
                          height: 12,
                          borderRadius: 999,
                          background: "var(--fg-4)",
                          marginRight: 6,
                          verticalAlign: "middle",
                        }}
                      />
                      Gray
                    </td>
                    <td><b>New / Neutral</b></td>
                    <td>0 completed and 0 defaulted</td>
                    <td>Brand-new wallet — no track record either way. Default starting state for every Poolver user.</td>
                  </tr>
                  <tr>
                    <td>
                      <span
                        style={{
                          display: "inline-block",
                          width: 12,
                          height: 12,
                          borderRadius: 999,
                          background: "var(--ok)",
                          marginRight: 6,
                          verticalAlign: "middle",
                        }}
                      />
                      Green
                    </td>
                    <td><b>Trusted</b></td>
                    <td>≥1 completed AND 0 defaulted</td>
                    <td>Has finished at least one full pool without ever defaulting. Lower risk to pool with.</td>
                  </tr>
                  <tr>
                    <td>
                      <span
                        style={{
                          display: "inline-block",
                          width: 12,
                          height: 12,
                          borderRadius: 999,
                          background: "var(--warn)",
                          marginRight: 6,
                          verticalAlign: "middle",
                        }}
                      />
                      Yellow
                    </td>
                    <td><b>Mixed</b></td>
                    <td>≥1 completed AND ≥1 defaulted</td>
                    <td>Has both successes and failures on record. Default may have been a one-off; weigh the ratio in the tooltip.</td>
                  </tr>
                  <tr>
                    <td>
                      <span
                        style={{
                          display: "inline-block",
                          width: 12,
                          height: 12,
                          borderRadius: 999,
                          background: "var(--err)",
                          marginRight: 6,
                          verticalAlign: "middle",
                        }}
                      />
                      Red
                    </td>
                    <td><b>Risky</b></td>
                    <td>≥1 defaulted AND 0 completed</td>
                    <td>Has only ever defaulted — no successful pool to balance it. Treat as a hard stop unless you know them off-chain.</td>
                  </tr>
                </tbody>
              </table>
            </div>

            <Callout title="Hover for raw counts">
              The colored dot is the one-glance signal. Hover over any
              reputation badge — in the topbar, in the roster, anywhere — to
              see the full <Code>joined · completed · defaulted</Code>{" "}
              breakdown plus lifetime USDC volumes. The bucketing is shared
              code (<Code>repTier()</Code> in <Code>@poolver/client</Code>)
              so V2 indexers and mobile clients see the exact same colors.
            </Callout>

            <h4>How tiers change</h4>
            <ul className="docs-ul">
              <li>
                <b>Joining a pool</b> increments <Code>pools_joined</Code> only — your color does not change.
              </li>
              <li>
                <b>Completing a pool</b> (12 months elapsed without defaulting) increments <Code>pools_completed</Code>. Gray → Green if no prior defaults; Red → Yellow if there were any.
              </li>
              <li>
                <b>Defaulting</b> (liquidated by <Code>liquidate_default</Code> on day 30 of unpaid status) increments <Code>pools_defaulted</Code>. Gray → Red, Green → Yellow.
              </li>
            </ul>

            <p>
              Reputation is <b>permanent and on-chain</b>. There is no admin
              instruction to reset it — the only way out of Yellow or Red is
              to complete more pools cleanly. New wallets start at Gray; bad
              behavior cannot be hidden by switching wallets and rejoining,
              because every new wallet starts from zero.
            </p>
          </DocSection>

          <DocSection id="09" n="09" title="Risks & disclaimers">
            <ul className="docs-ul">
              <li>
                <b>Smart contract risk.</b> The program is audited-pending.
                Until post-audit, pools are capped in size and treasury
                reimbursement is at multisig discretion.
              </li>
              <li>
                <b>Oracle risk.</b> Selection depends on Switchboard
                availability. If the oracle stalls, draws are paused;
                contributions are not affected.
              </li>
              <li>
                <b>Stablecoin risk.</b> USDC is an off-chain-backed token. A
                USDC depeg affects the real value of pool balances.
              </li>
              <li>
                <b>Regulatory.</b> Poolver is a programmatic savings
                coordination tool. It is not a bank, securities offering, or
                insurance product. Availability in your jurisdiction is your
                responsibility. This is not financial advice.
              </li>
              <li>
                <b>Anonymity.</b> Wallets are pseudonymous. If you link your
                wallet to your identity off-protocol, the protocol cannot
                unlink it.
              </li>
            </ul>
            <p className="dim" style={{ marginTop: 20, fontSize: 11.5 }}>
              Last updated 2026-04-20 · v0.1.0
            </p>
          </DocSection>
        </article>
      </div>
    </div>
  );
}
