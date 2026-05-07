# Poolver Pitch Deck — Feedback

Source reviewed: `Poolver — Solana Colosseum 2026.pdf` (14 slides)
Date: 2026-05-01
Audience: design pass for v2 of the deck

---

## Top-line verdict

The deck is **structurally solid** — clear arc, real metrics, no fluff. The narrative bones are right. But there are **four substantive issues** that undermine credibility on close reading, and a handful of slide-level fixes. Nothing requires a full rewrite. A focused v2 pass should target:

1. Internal consistency between deck and protocol (collateral math)
2. The missing "vs. informal ROSCA" story
3. Surfacing the SBT / reputation primitive as the long-term thesis
4. Tightening the ask + roadmap to match what's actually been built

---

## Critical issues (fix before submission)

### 1. Collateral inconsistency — slide 8 vs. actual protocol
Slide 8 says **"20% collateral lock"**. The protocol (`docs/PAYMENT_COMMITMENT.md`) enforces a bond ≈ `(N-1) × monthly`, which for a 10-member $100/mo pool is **$900 (90%)**, not $200 (20%).

A judge who reads the docs will catch this immediately. Worse, **20% collateral mathematically does not prevent post-win abandonment** — a round-1 winner would walk away with ~$800 of pure profit ($1,000 pot − $200 lost bond).

**Fix:** align the deck to the actual protocol. Per the Brazilian-dev's hackathon decision, v1 ships with **100% collateral** (= bond covering full remaining obligation). Reframe this on slide 8 as a *deliberate v1 design*, not a flaw:

> **Bond covers 100% of post-win obligation.** v1 is the reputation primitive — every honest cycle mints verifiable trust. v2 lowers collateral for KYC-verified users with proven history.

### 2. Missing "vs. informal ROSCA" argument
The competition slide (10) addresses regulated consórcios (Rodobens, Porto, Embracon) but **never addresses informal family/community tandas** — which is what the underbanked actually use today and the comparison every judge from LatAm/Africa/SE Asia will think of.

Informal ROSCAs have zero fees, zero KYC, zero protocol risk. Poolver is *more expensive* than a tanda among siblings. The pitch must answer: **why would someone leave their cousin's tanda for Poolver?**

The honest answer (and a strong one): **informal ROSCAs are capped by your social graph.** Trust = the people you already know = ~5–15 person pools = small lump sums. Poolver lets you build a **portable reputation** that unlocks pools with strangers, larger amounts, and cross-border members. You're not replacing the family tanda — you're giving it a path to scale beyond the family.

**Fix:** add a dedicated slide (or expand slide 10) with a 3-column comparison:

| | Informal tanda | Regulated consórcio | Poolver |
|---|---|---|---|
| Fees | 0% | 10–20% | 1.5% |
| Trust source | Social ties | Regulator | Cryptography + collateral |
| Pool size | $500–5k | $5k–$200k | $100–$50k+ (scales with reputation) |
| Reach | People you know | Your country | Anyone with a wallet |
| Reputation | Lost when group ends | Bank-only | Portable on-chain SBT |

### 3. SBT / reputation primitive is buried
The single strongest forward-looking story — that Poolver builds a **portable on-chain credit primitive** other protocols can consume — is not in the deck at all. This is the answer to "why is this a $1B company, not a $10M product."

**Fix:** add reputation as an explicit narrative thread:
- Slide 5 (mechanic): add a fifth step — `COMPLETE → reputation SBT minted`. The user's good behavior becomes a portable, verifiable asset.
- Slide 11 (roadmap): show the SBT thesis explicitly — v1 reputation primitive → v2 KYC tier with reduced collateral → v3 SBT consumed by other Solana lending protocols.

### 4. Ask is large for current traction
$500K–$1M pre-seed at $5–8M cap with **5 days of building, 0 users, 0 LOIs** is aggressive. Judges will probe this.

**Fix options (pick one):**
- **(a)** Lower the cap to $3–5M and ask $300–500K — a more defensible pre-seed for first-time founders pre-traction.
- **(b)** Keep the ask but add concrete pilot signal: "LOI with [Brazilian fintech / consórcio cooperative] for X-member pilot in Q3."
- **(c)** Reframe as a **round in two tranches** — $250K immediate (audit + 6mo runway) + $500K on mainnet milestone.

---

## Slide-by-slide notes

### Slide 1 — Cover
- Tagline inconsistency: "Pool. Verify. Receive." vs. slide 14's "…— for everyone." Pick one.
- "DEVNET" badge is honest but could read as "not ready." Consider "DEVNET LIVE — POOLVER.COM" with the implication of imminence.
- Add a single-line problem hook below the tagline so the cover earns its keep (e.g., *"$500B/yr in rotating savings, 0% on-chain."*)

### Slide 2 — The Problem
- Three problems are well-chosen but **"10–20% in fees"** only applies to formal consórcios; **informal ROSCAs charge 0%.** This conflates two different markets. Consider splitting: "formal: 10–20% fees" + "informal: capped by social graph."
- The "two billion people" framing is strong. Keep.

### Slide 3 — The Solution
- Direct 1:1 mapping to slide 2 is excellent. Keep the structure.
- "1.5% protocol fee" — if you're competing with informal ROSCAs at 0%, this needs the value-add justification (provable randomness + cross-graph trust + portable reputation). Add one line.

### Slide 4 — TAM
- $500B figure needs a tighter source citation. "World Bank · ABAC · BACEN" is hand-wavy. Cite specific reports with year. Brazilian consórcio at $250B is verifiable via ABAC; the $250B for Africa+Asia is harder — be ready to defend or lower.
- "ZERO credible on-chain competitors on Solana" is good — but expand to "on-chain ROSCA attempts on EVM (WeTrust, ROSCA.io) all failed for lack of fee+latency economics — Solana fixes this." Pre-empt the "why no one else has done it" question.

### Slide 5 — Mechanic
- Strong, clear. Add a fifth column: **COMPLETE → SBT minted**.
- The "5–50 members" range conflicts with slide 9's "10 members" assumption — make sure economics modeling matches typical group size.

### Slide 6 — Live Demo
- A 2-min live demo break in a pitch is high-risk. Have a 30s pre-recorded loom/video as fallback if devnet/wallet/network has any hiccup. Mention you have it: "Live first — recording as backup."

### Slide 7 — Architecture
- Good slide. The "5d zero-to-devnet" stat is great velocity signal.
- Consider adding one more stat: **lines of code shipped** or **% test coverage** — concrete engineering signals.

### Slide 8 — Trust & Safety
- **Major rewrite needed** (see Critical Issue #1). The current numbers don't enforce what they claim.
- Replace with: 100% bond, transparent slashing schedule, **insurance pool**, and explicit roadmap to KYC-tiered collateral in v2.
- The line "the cost of defection always exceeds the benefit" must be **mathematically true** with the numbers shown. Currently it isn't.

### Slide 9 — Economics
- Revenue projections (100 → 5K → 50K → 500K groups) are aspirational. Anchor the seed-stage 100 groups in a **concrete acquisition plan** (Brazilian consórcio cooperative pilot, specific Telegram tanda communities, etc.).
- "$0 token dependency" framing is strong — keep.
- Add unit economics: CAC assumption, LTV, payback period. Even rough numbers signal sophistication.

### Slide 10 — Competition
- See Critical Issue #2 — must include informal ROSCA column.
- Add 1 row for failed EVM attempts (WeTrust, ROSCA.network, etc.) to show category awareness — and explain why Solana's cost/finality unlocks what EVM couldn't.
- "—empty—" in the on-chain high-fees quadrant is fine, but consider also adding Goldfinch / Aave / Centrifuge as adjacent (not direct) competitors so the matrix isn't suspiciously empty.

### Slide 11 — Roadmap
- Add the **reputation/credit thesis** explicitly:
  - Q3 2026: KYC tier — 60–80% collateral for verified users (Brazil first via CPF + Serasa)
  - Q1 2027: Tiered collateral curve — 15–35% for users with 3+ completed cycles
  - Q3 2027: SBT consumed by external Solana lending protocols
- "Bid mechanism (Lance)" — most non-Brazilians won't know this term. Translate as "Reverse-auction draw — members can discount their pot to win earlier (mirrors traditional consórcio lance)."

### Slide 12 — Team
- Two-person teams are common at this stage but **add an advisor box** if you have anyone (consórcio industry, Brazilian fintech, Solana ecosystem). Even one named advisor materially strengthens the slide.
- "Owns the protocol surface" / "Owns the product surface" — current language is fine but could be sharper. Show **one previous shipped product** per founder if relevant.

### Slide 13 — The Ask
- See Critical Issue #4.
- Use of funds: $50K for security audit is **light** for a financial protocol. OtterSec/Neodyme audits typically run $50–150K. Be ready to defend or revise.
- "Runway (12 months)" — show monthly burn so the 12-month figure is checkable.

### Slide 14 — Closer
- Tagline ("for everyone") good. Make consistent with slide 1.
- Add a single CTA above the contact info: **"Try it now: poolver.com"** — sends judges to the demo.

---

## Visual / design suggestions (for design pass)

I cannot see visuals from the PDF text extraction, so these are inferred from text density and layout cues:

- **Numerical hierarchy**: stats like "500B", "20%", "1.5%" should be the largest single elements on their slides — currently appear well-handled but verify they breathe.
- **Slide 4 TAM chart**: ensure the $500B number visually dominates, with the breakdown as a secondary callout.
- **Slide 8 Trust & Safety**: the slashing schedule is a good visualization candidate — consider a horizontal bar showing bond depletion over missed payments.
- **Slide 10 competition 2x2**: classic format but make sure Poolver is visually weighted (color, size, halo) to dominate the "on-chain · low fees" quadrant.
- **Brand consistency**: deck appears to use a dark/financial aesthetic. Ensure the SBT / reputation slide additions don't break the visual language.

---

## What's working — don't change

- The 5-day-to-devnet velocity is your single best signal. **Lead with it where possible.**
- The "no admin, no custodian, no permission" framing is crisp.
- Direct problem→solution mapping (slides 2→3) is elegant.
- "ZERO credible on-chain competitors on Solana" is a real moat statement.
- Switchboard VRF + commit-reveal is a strong technical credibility marker.

---

## Suggested v2 deck order (minor reordering)

Current order works. One optional change: **swap slide 6 (live demo) with slide 8 (trust & safety)** so trust/safety comes before the demo. If the demo wobbles, the trust story has already landed. If demo nails it, trust slide reinforces.

---

## Numbers to nail down before next investor meeting

- [ ] Final collateral bps (decision: 100% for v1)
- [ ] Verified TAM source citations (specific reports, year)
- [ ] Audit budget (revise $50K up if OtterSec/Neodyme)
- [ ] Monthly burn assumption for runway claim
- [ ] At least one LOI or community partnership (Brazil)
- [ ] Slashing schedule final numbers (replace "−10% / −25% / −100%")
- [ ] CAC / LTV rough estimates
