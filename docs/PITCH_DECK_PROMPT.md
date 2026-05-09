# Poolver Pitch Deck — Prompt for Claude Design

Paste this whole document into Claude Design (or whichever tool generates
the deck). Update the bracketed placeholders before sending.

---

## Project context — read first

Poolver is a Solana-based **consórcio** protocol — Brazilian rotating
savings groups, but verifiable, smart-contract enforced, and 10× cheaper
than the traditional kind. We've shipped V1 on devnet (4 Anchor programs,
two tiers: Vault and DeFi/Kamino). Brand is dark/terminal aesthetic —
match the visual language of the existing deck and `docs/BRAND_GUIDE.md`.

**Audience:** seed-stage investors. They glance at slides for ~30 seconds.
Words die. Visuals win.

**Length:** 11-12 slides.

**Hard rules:**

- ≤ 15 words of headline per slide, 1–3 supporting bullets max.
- One chart, photo, or diagram per slide.
- 30% text / 70% visual. If a slide reads like a paragraph, cut it.
- Numbers beat adjectives.
- Match Poolver's brand (dark bg, monospace for stats, accent on key
  numbers). No stock photos. No emoji-as-illustration.
- Leave clean placeholder boxes for: `[ORDA LOGO]`, `[PORTO SEGURO LOGO]`,
  `[DAVID PHOTO]`, `[BRENNO PHOTO]` — assets will be pasted in after.

---

## Slide-by-slide

### 1 · Title

Just the brand. Wordmark + tagline:

> Poolver
> **Verifiable consórcios on Solana**
> Solana Colosseum 2026 · poolver.com
> David Potolski Lafetá · Brenno [LASTNAME]

Footer line in mono: founders' emails / X handles.

### 2 · Problem — "Brazil runs on consórcios. They're broken."

Three pain icons + one hero stat.

- **Hero stat (large):** **~$60B+ USD** in active consórcios · ~9M Brazilian participants. (`Source: ABAC 2023, converted from R$ at ~5.0 BRL/USD`)
- **Three pains** (icon + 2–3-word label):
  - Opaque draws — you trust the administrator
  - Admin fees up to ~20% of contributions
  - Multi-year waits with no guaranteed contemplation date

### 3 · Market — "Bigger than crypto natives realize"

One chart (consórcio AUM trend last 5y) **OR** a 2×2 grid of use cases.

- One-line headline: *"Consórcios are how Brazilians buy major assets
  without paying interest."*
- Two market segments highlighted:
  - **Lower-income**: only credit-free path to a car / appliance
  - **Upper-income**: planning apartment & home purchases interest-free
- Mini callout: "Same playbook works in Mexico, Colombia, Argentina."

### 4 · Solution — "Move the trust into a smart contract."

Side-by-side comparison, no paragraphs:

|                       | Traditional consórcio | **Poolver**             |
| --------------------- | --------------------- | ----------------------- |
| Admin fee             | ~20% of contributions | **1.5% protocol fee**   |
| Trust model           | The administrator     | **Open-source code**    |
| Draws                 | Closed-room           | **Verifiable on-chain** |
| Currency              | BRL only              | **USDC · global**       |
| Onboarding            | Years of paperwork    | **One click**           |

Bottom: *"Same product. Lower fees. Provable fairness."*

### 5 · Product — "Live on devnet today."

Two screenshots side-by-side from poolver.com:

- The `/pools` grid showing live pools across both tiers
- A pool detail page with the month timeline + roster

Single caption line: *"12 people. 12 months. 12 winners. One smart contract."*

### 6 · How it works — minimal diagram

One flow diagram, no text-heavy explainer:

> 12 wallets contribute → smart contract → sealed-bid auction (or VRF
> lottery if no bidders) → winner receives pot → repeat 12 months

Below: *"Yield-bearing tier deposits idle USDC into Kamino while
participants wait."*

### 7 · Why now

Three short bullets, icon each:

- **Stablecoin adoption in Brazil** is at an all-time high (BRL/USDC + Pix integration)
- **Solana** finally delivers consumer-grade speed and cost for retail
- **Regulators** (BCB, CVM) are warming to tokenized financial products

### 8 · Partners + traction — "We're not just code."

Three visual blocks across the slide:

**Block A — Orda (on/off-ramp partner)** [LEFT]

- `[ORDA LOGO]` placeholder
- Quote (use the actual sentence after [Orda founder name] confirms wording — currently authorized to attribute):
  > *"[Founder name]: 'Poolver is the first Brazilian crypto product
  > that makes consórcios actually work. We're proud to power their
  > fiat on/off-ramp.'"*
- Caption: BRL ↔ USDC, Pix-native, in production.

**Block B — Porto Seguro (regulatory bridge)** [CENTER]

- `[PORTO SEGURO LOGO]` placeholder
- *"In active discussions with a Porto Seguro representative to bring
  Poolver closer to the reality of Brazilian consumers."*
- Subline: regulatory & distribution working group.

**Block C — Devnet today** [RIGHT]

- "Live on Solana devnet"
- Three stats (auto-pull from chain or update by hand before deck):
  - X pools deployed
  - Y total contributions
  - $Z in protocol fees collected

### 9 · Business model — clean two-tier

Single graphic, two columns:

**Tier 0 · Vault**

- 1.5% protocol fee
- 1.5% reserve (insurance)
- No yield
- For "just want it simple" users

**Tier 1 · Kamino (DeFi)**

- 1.5% protocol fee
- 2.5% reserve
- Auto-compounding USDC via Kamino
- Yield while participants wait

Bottom line: *"Protocol revenue scales linearly with TVL. We don't
extract — we earn 1.5%."*

### 10 · Go-to-market — Brazil first, then up the LatAm ladder

3-column phase plan:

| Phase 1 · Year 1                        | Phase 2 · Year 2          | Phase 3 · Year 3+                  |
| --------------------------------------- | ------------------------- | ---------------------------------- |
| **Brazil only**                         | **Mexico · Colombia · Argentina** | **Global emerging markets** |
| Orda + Porto Seguro pilot               | Localize playbook         | Asia · Africa expansion            |
| Build trust, hit first 1 000 pools      | Same product, scaled fees | Cross-border consórcio liquidity   |

One-line below: *"Brazil first: largest existing consórcio market,
deepest cultural fit, Orda + Porto Seguro already engaged."*

### 11 · Team — two founders, both 100% on Poolver

Two cards, side by side:

**[DAVID PHOTO]   David Potolski Lafetá**
- Lead Protocol Engineer
- **Quit his full-time job to work on Poolver full-time**
- Built all four Anchor programs + deploy infra + the audit-ready core
- Brazilian, [previous company / experience line — fill in]

**[BRENNO PHOTO]   Brenno [LASTNAME]**
- Frontend & Partnerships Lead
- **Closed the Orda integration. Driving Porto Seguro discussions.**
- Builds and ships the user-facing app
- Brazilian, [previous company / experience line — fill in]

Caption: *"Two-person team. Full focus. Brazilian-native go-to-market."*

### 12 · Ask

Three concise points + a QR. Keep the dollar figure flexible — we'll
size the raise per the conversation, not the deck.

- **Now raising — let's talk.** Building 12 months of runway.
- **Where the money goes**: engineering · biz dev (regulatory + partnerships) · audit · mainnet launch
- **Contact**: davidpotolskilafeta@gmail.com · [Brenno's email] · @poolverfi
- Big QR code → poolver.com

---

## Visual checklist for the designer

- [ ] Dark background, monospace stats, accent on the key number per slide
- [ ] No paragraph-heavy slides — anything > 15 words on a slide gets cut
- [ ] Logos placed in clean rectangles with breathing room (≥16px padding)
- [ ] Each chart has one takeaway, one source line in muted mono
- [ ] Founder photos same crop, same lighting, same background tone
- [ ] One CTA per slide max
- [ ] Slide numbers bottom-right in muted mono

---

## Iteration plan

Generate a first complete pass. After review we'll iterate on:

1. Exact USD-converted market numbers (verify against ABAC + Banco
   Central sources, use a conservative BRL→USD rate)
2. Quote wording from Orda founder once cleared
3. Funding-ask wording on slide 12 — keep figures out of the deck;
   surface the number in the meeting once the conversation has shape
4. Porto Seguro phrasing — keep it as "in active discussions" until
   anything is publicly announceable
5. Tightening any slide that creeps over the 15-word headline budget
