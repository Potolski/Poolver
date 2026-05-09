# Poolver Deck v1 — Review Notes

Reviewed `Poolver — Solana Colosseum 2026FINAL.pdf` (12 slides). Listed
below are the must-fix items + a slide-by-slide critique.

---

## Must-fix (from the user)

### 1. Slide 8 stats — replace placeholders with real on-chain numbers

The deck currently shows fictional numbers (`14 pools`, `$48K
contributions`, `$720 protocol fees`). Real devnet snapshot pulled
from chain via `scripts/deck-stats.ts`:

| Field                  | Value (live)                          |
| ---------------------- | ------------------------------------- |
| Pools deployed         | **12** (4 completed · 6 active · 2 forming) |
| Total contributions    | **$2.7M USD**                         |
| Total distributed      | **$10.5K USD**                        |
| Collateral locked      | **$3.4M USD**                         |
| Protocol fees collected| **$32.8K USD**                        |
| Vault-tier reserve     | **$18.8K USD**                        |
| DeFi-tier reserve      | **$23.6K USD**                        |

Suggested wording for the Block C bullets:

> **12 pools deployed** · 4 completed · 6 active · 2 forming
> **$2.7M** in lifetime contributions
> **$32.8K** in protocol fees collected
> **$42.4K** in tier reserves (insurance backstop)

Re-run `npx tsx scripts/deck-stats.ts` immediately before sending the
deck — devnet keeps moving as we test.

### 2. Brenno's contact info — slides 11 + 12

The deck has him listed as `@poolverfi` on slide 11 and
`brenno@poolver.com` on slide 12. Neither exists.

**Fix:**

- **Slide 11 (Team):** replace `@poolverfi` line under Brenno with
  `brennoaraujoqueiroz@gmail.com`.
- **Slide 12 (Ask):** replace `brenno · brenno@poolver.com` with
  `brenno · brennoaraujoqueiroz@gmail.com`.
- **Keep** `@poolverfi` only at the project / company level
  (footer of slide 1, slide 12's project line). It's the X handle
  for the project, not Brenno's personal handle.

While there, also drop `x · @poolverfi` from the per-person contact
block on slide 12 — it conflates personal contact with the project
handle. Move it to a single "Project · @poolverfi · poolver.com" line.

### 3. Crypto experience years — both founders are off

Currently shown as "8+ years" (David) and "7+ years" (Brenno). Real:

- **David**: in crypto since **2019** — **7 years** at end of 2026.
- **Brenno**: in crypto since **2021** — **5 years** at end of 2026.

Suggested copy: replace the "X+ years working in crypto" line with
"in crypto since 2019" / "in crypto since 2021" — explicit start
year is more honest than a rounded count and ages well as time
passes.

### 4. QR code on slide 12 — currently broken, replace with the real one

Generated and committed:

- `docs/assets/poolver-qr.svg` (vector, infinite scaling)
- `docs/assets/poolver-qr.png` (1200px, error-correction level H so
  even a partially obscured print still scans)

Both encode `https://poolver.com` and tested-scannable. Use the SVG
on the slide so the print doesn't pixelate.

### 5. Orda founder quote — generated three options

Founder gave us creative license. Three drafts in descending order
of how-much-juice. Pick one — A is the strongest:

> **A.** *"Poolver is what consórcios should have been from the
> start. We're betting our on-ramp on it because Brazilian families
> deserve this product."*
> — Orda Founder
>
> **B.** *"In ten years of fintech I haven't seen a Brazilian crypto
> team this aligned with how real households actually save. Orda is
> wiring the fiat rails so Poolver can ship."*
> — Orda Founder
>
> **C.** *"Brazil invented consórcios. Poolver makes them honest.
> That's the kind of product Orda exists to power."*
> — Orda Founder

Plug the real founder name in once you confirm the final wording.
Recommend **A** — direct, opinionated, lands a clear endorsement
without sounding like marketing copy.

---

## General review (slide-by-slide)

### Slide 1 · Title
- Strong. Three-pill anchor (`USDC · GLOBAL`, `LIVE ON DEVNET`,
  `1.5% PROTOCOL FEE`) is doing real work — keep them.
- Footer: drop the `@poolverfi` next to David's email (his email is
  enough). Move the X handle to its own footer line so neither
  founder is wrongly attributed.

### Slide 2 · Problem
- Hero stat lands. Good.
- The "Multi-year waits" bullet says "5–7 year cycles" — verify that
  number against ABAC. Real data shows median car consórcio is
  60–80 months and real-estate up to 200 months, so "5–7 years" is
  the car/appliance segment, not the whole market. Edit to
  "5+ year cycles, sometimes 15+ for real estate" or pull the
  median from ABAC.

### Slide 3 · Market
- "~70% of households · essential goods" — that figure isn't
  ABAC-sourced and looks weak. Either replace with a hard number
  ("9M active participants" — the same as slide 2 but pivoted to
  segment), or drop the sub-stat and let the headline carry.
- Latin-American expansion callout (MX · CO · AR) is cleanly done.

### Slide 4 · Solution
- Comparison table works. Consider adding ONE row: "Settlement"
  with "weeks" vs **"400ms"** to lean into the Solana speed pitch
  without adding a slide.

### Slide 5 · Product
- Two screenshots side-by-side is the right move. The mockups in
  the deck show an older version of the UI without the new Kamino
  panel or the per-month slashed/paid bar — re-screenshot once the
  push lands so the deck matches what investors will actually see
  if they click through.

### Slide 6 · How it works
- "Five moves. Twelve months." is a great headline. Keep.
- The Kamino-tier callout at the bottom is buried. Consider
  promoting it to a colored chip on step 04 ("RECEIVE — yield
  earned during wait, on Kamino tier") so the yield mention isn't
  an afterthought.

### Slide 7 · Why now
- Three-column structure is solid. The BCB / CVM source line at
  the bottom of column 3 is the only one with a date — add similar
  data points to the other two columns ("Pix-USDC volume YTD",
  "Solana TPS / median fee") for symmetry and credibility.

### Slide 8 · Partners + traction
- Real numbers: see #1 above.
- Orda quote: see #5 above.
- Porto Seguro framing is correctly cautious. Keep "in active
  discussions" until you have public sign-off — investors respect
  hedged language more than they respect overclaiming.

### Slide 9 · Business model
- Tier comparison is clear. The "1.5% protocol fee" line is
  identical for both tiers — call out that the **reserve fee
  differs** (1.5% Vault vs 2.5% DeFi) because that's the structural
  insight that distinguishes the tiers and matches the higher-risk
  Kamino exposure. Right now both reserve lines are equal-weight
  visually — color the 2.5% on Tier 1 in accent so the eye catches
  it.

### Slide 10 · Go-to-market
- "1,000 pools" target on Phase 1 is concrete, good. Add a Phase 2
  number (e.g. "10K pools across 3 LatAm countries") so the ramp
  feels modeled, not aspirational.

### Slide 11 · Team
- Years fix: see #3.
- Brenno's `@poolverfi` line: see #2.
- Both founders need a one-line "previous lifetime" quip — the deck
  has nothing pre-Poolver. A short line ("ex-[company]" or "shipped
  X before") makes the team page read like resumes, not LinkedIn
  headlines. Even a generic "shipped Solana programs since 2022" /
  "frontend at [stack]" is enough.

### Slide 12 · Ask
- Copy is the right tone — kept the dollar figure off the page.
- "Where the money goes" lists buckets without weights. That's
  fine, but a one-sentence priority order ("engineering &
  audit first, biz dev once a real pool's live with Pix")
  makes it less generic.
- Replace the broken QR with `docs/assets/poolver-qr.svg`.
- Drop the per-founder X handle, leave one line for project
  contact (`@poolverfi · poolver.com`).

---

## Cross-deck cosmetic notes

- **Footer line** on every slide: `2 TIERS · VAULT & KAMINO ·
  DEVNET DEPLOYED` style is consistent and on-brand. Keep.
- **Slide numbers** ("02 / 12 · THE PROBLEM" pattern) work great —
  helps the audience track where they are.
- **Color use**: the dollar accent on stats (e.g. `$60B+`) is
  doing the right work. Apply the same accent to the headline
  numbers we add on slides 3, 9, 10 so the visual rhythm carries.
- **Typography**: monospace for stats / sans for narrative is
  consistent. Don't break it.

---

## Suggested next pass

1. Fix the four hard items (#1–4 above) — these are non-negotiable.
2. Pick Orda quote variant; confirm with founder; plug their name.
3. Re-screenshot product slides after current push is live.
4. Backfill founder one-line "previous lifetime" credentials.
5. Tighten slide 7 source citations for symmetry.
6. Consider the Kamino-on-step-4 chip on slide 6.

After all that, this is a solid v2 ready to send.
