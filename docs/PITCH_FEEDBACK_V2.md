# Poolver Pitch Deck V2 — Feedback

Source reviewed: `Poolver — Solana Colosseum 2026V2.pdf` (14 slides)
Compared against: V1 feedback in `docs/PITCH_FEEDBACK.md`
Date: 2026-05-01

---

## Top-line verdict

**Major lift.** V2 closes nearly every critical issue from V1 feedback. The deck is now internally consistent, the value prop vs. informal ROSCA is sharp, the SBT/reputation thesis is the spine of the narrative, and the ask is properly milestone-gated. **This is a submittable deck.**

What remains is **polish-level**: a few number inconsistencies, a typo, two slides that could land harder. None are blockers.

---

## What V2 fixed (acknowledge the work)

| V1 issue | V2 status |
|---|---|
| Collateral inconsistency (20% vs ~90%) | ✅ Fixed — slide 8 now shows 100% bond with the $0-defection-profit math |
| Missing "vs. informal ROSCA" story | ✅ Fixed — slide 10 is now a 3-column tanda/consórcio/Poolver table with the *"we don't replace, we scale"* tagline |
| SBT/reputation thesis buried | ✅ Fixed — promoted to slide 5 (5th step) and slide 8 sidebar; threaded through roadmap |
| Ask oversized for traction | ✅ Fixed — two-tranche, milestone-gated, cap dropped to $4M |
| Cover lacked hook | ✅ Fixed — "$500B/yr in rotating savings. 0% on-chain. We change that." |
| Closer tagline weak | ✅ Fixed — "For the two billion the banks won't bank" + TRY IT NOW CTA |
| TAM source citations vague | ✅ Mostly fixed — ABAC 2024, World Bank Findex 2021, BACEN, GSMA SOTIR 2024 |
| Demo break risk | ✅ Mitigated — backup recording mentioned |
| Team slide thin | ✅ Improved — advisor box added |
| EVM competitor pre-empt | ✅ Added — WeTrust + ROSCA.network with "by physics, not accident" framing |
| Architecture stat density | ✅ Improved — LOC stat added |

---

## Remaining issues (fix before submission)

### 1. Typo on slide 2 — `"consorção"` → `"consórcio"`
Slide 2 reads *"Brazilian consorção admins keep 15% off the top"*. The correct word is **consórcio**. This is the *problem slide* — a typo on the term that anchors the Brazil thesis is high-visibility. **Fix immediately.**

### 2. Number inconsistency — pool size across slides
- Slide 5 (mechanic): "10–20 members"
- Slide 10 (competition): "$100–50K+ pool size"

A 20-member pool at $100/mo for 12 months = $24K max pot. To hit $50K+ you need either ~40 members or ~$200/mo contributions. **Pick one canonical range and apply consistently.** Recommend: "10–20 members, $100–500/mo contributions" → max pot ~$120K. Update slide 10 accordingly.

### 3. Tranche 1 math doesn't reconcile with use-of-funds
Slide 13 says:
- Tranche 1 = **$300K** (audit + 6mo runway)
- Use of funds (full round): audit $120K + team $300K + BD $120K + infra $60K + runway $400K = **$1M**

If tranche 1 is $300K and tranche 2 is $500K, full round is $800K, not $1M. Or if full round is $1M, the tranche split is $300K + $700K — not $500K. **The math doesn't close.** Either:
- (a) Make tranches sum to $800K and revise use-of-funds to $800K total, OR
- (b) Set tranche 1 = $300K, tranche 2 = $700K, total $1M

Be explicit on slide 13 what tranche 1 specifically funds vs the full round. A judge with a finger on the calculator will catch this in the Q&A.

### 4. Slide 8 — pot value should net protocol fee
The "Pot if you win round 1: +$1,000" doesn't account for the 1.5% protocol fee taken at `claim_payout` (per slide 9). Technically the round-1 winner receives **$985**, not $1,000. Bond forfeiture is still $900, so net defection profit becomes **−$15** (not zero) — which is even *stronger* for your argument.

**Fix:** update to `+$985 / −$900 / Net = −$15`. Or footnote: *"Net of 1.5% protocol fee. Bond loss + opportunity cost = strictly negative defection."*

### 5. Advisor "in conversation" is a softness
Slide 12 mentions a Brazilian consórcio advisor "in conversation, closing pre-mainnet." Two failure modes:
- **(a)** Judges ask "who?" — you don't have a name to share → slide weakens
- **(b)** It doesn't close → you've mentioned them in the deck

**Fix options:**
- If you have a verbal yes: name them (with permission) — this is meaningfully stronger
- If still soft: change to *"Recruiting BR consórcio advisor — 2 candidates engaged"*
- If you'd rather not commit: remove the advisor box; keep just the "Post-raise hires" callout

### 6. Pix partnership in tranche 2 is outside your control
Slide 13 lists *"Pix on-ramp partner signed"* as a tranche-2 unlock condition. Pix integration depends on a third party (Bitso, Notus, AmFi, etc.) signing a deal. If they delay or decline, your tranche 2 unlocks slip — even though your *team* delivered everything else.

**Fix:** replace with a milestone in your control: *"Pix integration spec'd + 1 partner in due diligence"* or *"Off-ramp solution validated with 5 pilot users."* Investors prefer milestones the team controls.

---

## Polish-level (nice-to-have)

### Slide 4 — TAM
The "10M+ active consórcio participants moving $250B annually" implies $25K/yr per participant ($2,083/mo). That's plausible for vehicle/property consórcios but disconnects from the "$100/mo demo pool" framing elsewhere. Consider adding a unit-economics anchor: *"Brazil avg consórcio contribution: ~$200–500/mo over 60–80mo cycle"* — closes the gap between TAM and the example pool.

### Slide 11 — Roadmap ordering
- Q3 2026: KYC tier
- Q4 2026: Reverse-auction draw (Lance)
- Q1 2027: Tiered collateral curve

The KYC tier and tiered curve are the **strategic spine** (collateral reduction → credit primitive). The Lance bid is a **product feature**. Consider:
- Q3 2026: KYC tier — 60–80% bond
- Q4 2026: Tiered curve — 15–35% for repeat users
- Q1 2027: Reverse-auction draw (Lance) + governance experiments

This keeps the credit-primitive thesis as a continuous arc through 4 quarters instead of breaking it with a feature in the middle.

### Slide 14 — Spacing
Text extraction shows `"PoolverPool. Verify. Receive."` (no space). Probably a layout artifact, but verify in the actual slide that brand wordmark and tagline have proper separation.

### Cover slide — repeat the velocity stat
"5d zero to devnet" is the single strongest signal in the deck. Consider a small footer line on the cover: *"Built in 5 days. Live on devnet."* Same energy as the "$500B" hook above the fold.

---

## What's missing — consider for V2.1 or speaker notes

### Regulatory positioning (1-line)
Brazilian fintech pitches almost always get a regulatory question. Even one line in speaker notes (or as a footer) helps:
> *"Poolver is on-chain coordination infrastructure, not a financial institution. v3 credit features will require SCD registration with BACEN — pathway scoped, audit-conditional."*

This pre-empts the "are you regulated?" question without sounding defensive.

### CAC defense
$8 CAC via "community-led" is plausible but probe-worthy. Be ready in Q&A with the channel logic:
- Telegram tanda communities → community leader incentive
- BR consórcio cooperatives → revenue share with cooperative admin
- Word-of-mouth K-factor from completed cycles (each successful pool = 9 social proofs)

### Why no token
Slide 9 says *"$0 token dependency — optionality, not crutch"* — strong stance. Worth an extra beat in speaker notes on **why** (regulatory clarity, no token-dump risk, real revenue from day 1, focus on UX).

---

## Visual / design notes (inferred from text)

- Slide 8 trust & safety math callout (`+$1,000 / −$900 / −$900 / Net $0`) is your single most persuasive visual moment. **Make sure it's the dominant element on the slide.**
- Slide 10 competition table is text-heavy — ensure the Poolver column is visually weighted (color, halo) to read as the answer.
- Slide 11 roadmap with checkmarks (shipped) vs circles (planned) is a strong device — keep.
- Cover slide: the new "$500B / 0% / we change that" hook deserves typography hierarchy that earns the claim.

---

## Numbers to nail down before pitching

- [ ] Reconcile tranche 1 + tranche 2 with use-of-funds total
- [ ] Confirm pool size range (members × monthly contribution → max pot)
- [ ] Net pot after 1.5% protocol fee on slide 8
- [ ] Decide on advisor: name, "recruiting", or remove
- [ ] Replace Pix milestone with team-controlled equivalent
- [ ] Fix `consorção → consórcio` typo

---

## Verdict

If you fix #1 (typo), #3 (tranche math), and #5 (advisor wording) before submission, this deck is **ready**. The other items are polish that you can iterate on after Colosseum.

The narrative arc — *broken system → on-chain replacement → 100% bond as the reputation primitive → KYC + tiered curve as the credit primitive → SBT consumed by other protocols* — is now coherent and compelling. That was the hard part. You've done it.
