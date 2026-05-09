# Backlog

Post-V1 work items that aren't blocking the hackathon demo but should
land before any real-money launch. Each entry is a one-paragraph
description plus the rough scope so we can pick them up out of order.

---

## Admin withdraw paths for protocol fees + tier reserves

**Why:** Today there's no way for the protocol to move USDC out of the
two long-lived treasury accounts:

- `protocol_fee_vault` (`EBMyGaPTCscvCXgtUsTzZFMeHxVzBxNEhQGjDe17oVao` on devnet)
  — accumulates 1.5% of every contribution + 5% of every winning bid.
  Currently sits at ~$32K on devnet. Real protocol revenue.
- `reserve_usdc_vault` per tier
  (vault: `4Jcvvh8QMy4TkDrsuFzByLYPsVZTFTuERZS3hjaQxrpM`,
   defi: `2QPRpvpAsKkiYueAXzUTeb8ixz4e67BmE2RDNpjCwveW`)
  — insurance backstop drawn during `liquidate_default`. Tier-shared
  across all pools of the same tier (INV-4).

`admin_close_reserve` only closes EMPTY token accounts (SPL `CloseAccount`
fails non-empty), so the funds are locked until we ship a withdraw ix. If
the project ever winds down, money is stranded.

**Scope:** two new admin instructions in `poolver-core` (or `poolver-reserve`
for the reserve one):

1. **`admin_withdraw_protocol_fees(amount)`**
   - Auth: `protocol_config.admin == caller.key()`
   - Source: `protocol_fee_vault` (PDA-signed)
   - Dest: any USDC ATA the caller passes
   - No additional gate — these are clean revenue, withdraw any time.

2. **`admin_withdraw_reserve(tier, amount)`** (CPI'd from core into reserve)
   - Auth: `protocol_config.admin == caller.key()`
   - Source: `reserve_usdc_vault` (per tier)
   - Dest: any USDC ATA the caller passes
   - Gate options to pick from at design time (not a hard call yet):
     - **all-pools-complete**: require iterating active pools and
       confirming `is_complete=true` for all of the same tier. Heavy.
     - **paused-flag**: require `protocol_config.paused == true`. Cheap,
       but doesn't actually verify no active pools depend on the reserve.
     - **timelock**: 30-day delay between proposal and execution. Safest
       for production, overkill for a hackathon retreat.
   - V2 wraps both with a Squads multisig; V1 single-key on devnet.

**Effort:** ~80 LOC Rust + IDL refresh + small SDK + admin-only UI button.

---

(add new items below this line)
