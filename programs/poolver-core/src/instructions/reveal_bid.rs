use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, Transfer};

// `solana-sha256-hasher` is the canonical home of the sha256 syscall
// wrapper after solana-program 2.2.0 deprecated the inline `keccak` /
// `hash` re-exports. INV-14 binds the bid commitment to sha256, so the
// concrete syscall — not blake3 / keccak — is what we need.
use solana_sha256_hasher::hashv;

use crate::constants::{
    BID_CAP_BPS, BID_SEED, BID_STAKE_VAULT_SEED, BPS_DENOMINATOR, PARTICIPANT_SEED,
    POOL_SIZE, PROTOCOL_CONFIG_SEED,
};
use crate::error::CoreError;
use crate::events::BidRevealed;
use crate::state::{Bid, Participant, Pool, ProtocolConfig, Tier};

/// `reveal_bid` — spec §5.1.
///
/// Opens the previously-submitted commit by supplying the
/// `(bid_amount, nonce)` preimage. The on-chain handler reconstructs
/// `sha256(bid_amount.to_le_bytes() || nonce || user_pubkey.to_bytes())`
/// (56 bytes) and rejects with `BidRevealMismatch` (INV-14) on any
/// difference.
///
/// Pre-conditions:
///   - protocol not paused (INV-25 — defence in depth even though
///     reveal is non-funds-moving except for the stake refund)
///   - pool active, not complete
///   - inside the reveal window: `bid_window_ends_at <= now <
///     reveal_window_ends_at`. Two distinct error codes
///     (`BidWindowOpen` vs `BidWindowClosed`) tell the client which
///     edge they're outside of.
///   - the bid is for THIS month (`bid.month == pool.current_month`).
///     A stale `Bid` PDA from a previous month would have the wrong
///     month seed and Anchor's PDA derivation already rejects it; we
///     additionally check `bid.month` against the live month so the
///     error surface is friendly.
///   - `bid.revealed == false` — `AlreadyRevealed`
///   - participant has not won (defence-in-depth — they shouldn't be
///     able to commit either, but we re-check)
///   - hash matches (INV-14)
///   - `bid_amount > 0` (Q-9: 1-microUSDC granularity, but zero is
///     never a valid bid)
///   - `bid_amount <= bid_cap` where `bid_cap = 20% of monthly_pot`
///     and `monthly_pot = 12 × (contribution_amount − protocol_fee −
///     reserve_fee)` per Q-10 (INV-15)
///
/// Stake refund: on a successful reveal, the user's 1% anti-spam stake
/// flows back from `bid_stake_vault` → `user_usdc` via the bid-stake
/// vault's self-signing PDA. `bid.stake_refunded` flips `true` so the
/// step 7 cleanup can't double-refund.
///
/// SPEC_QUESTION-15: `Pool` is `Box`'d; `protocol_config` is manually
/// deserialized to keep `try_accounts`'s frame within the 4 KB budget.
#[derive(Accounts)]
pub struct RevealBid<'info> {
    /// The bidder. The `user` field is verified against the seed-bound
    /// `bid.user` so spoofing is structurally impossible — Anchor's PDA
    /// derivation requires the bid PDA seeds to include this signer.
    pub user: Signer<'info>,

    /// Protocol config. Manually deserialized (SPEC_QUESTION-15).
    /// CHECK: PDA seed binding here, owner + discriminator validated
    /// inside the handler.
    #[account(seeds = [PROTOCOL_CONFIG_SEED], bump)]
    pub protocol_config: UncheckedAccount<'info>,

    /// Pool. Read-only; we only read tier + windows + contribution.
    pub pool: Box<Account<'info, Pool>>,

    /// Per-(pool, user) participant. Read-only — we only re-check
    /// `has_won`, `is_defaulted`, `is_suspended`.
    #[account(
        seeds = [PARTICIPANT_SEED, pool.key().as_ref(), user.key().as_ref()],
        bump = participant.bump,
        constraint = participant.pool == pool.key() @ CoreError::NotAParticipant,
        constraint = participant.user == user.key() @ CoreError::NotAParticipant,
    )]
    pub participant: Box<Account<'info, Participant>>,

    /// Sealed-bid record. The PDA seed includes `month`, so a `Bid` PDA
    /// for a stale month would not match this derivation. We also check
    /// `bid.month == pool.current_month` in the handler for a clearer
    /// error surface.
    #[account(
        mut,
        seeds = [
            BID_SEED,
            pool.key().as_ref(),
            &[bid.month],
            user.key().as_ref(),
        ],
        bump = bid.bump,
        constraint = bid.pool == pool.key() @ CoreError::NotAParticipant,
        constraint = bid.user == user.key() @ CoreError::NotAParticipant,
    )]
    pub bid: Box<Account<'info, Bid>>,

    /// User's USDC ATA — receives the stake refund.
    /// CHECK: SPL transfer enforces ownership semantics.
    #[account(mut)]
    pub user_usdc: UncheckedAccount<'info>,

    /// Per-pool bid-stake vault.
    /// CHECK: PDA seed binding ensures identity; `mut` because we
    /// withdraw the refund from it.
    #[account(
        mut,
        seeds = [BID_STAKE_VAULT_SEED, pool.key().as_ref()],
        bump,
    )]
    pub bid_stake_vault: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_reveal_bid(
    ctx: Context<RevealBid>,
    bid_amount: u64,
    nonce: [u8; 16],
) -> Result<()> {
    // ───── 1. Pause check (INV-25) ─────────────────────────────────────
    {
        let acct = &ctx.accounts.protocol_config;
        require_keys_eq!(*acct.owner, crate::ID, CoreError::Unauthorized);
        let mut data: &[u8] = &acct.try_borrow_data()?;
        let cfg = ProtocolConfig::try_deserialize(&mut data)?;
        require!(!cfg.paused, CoreError::ProtocolPaused);
    }

    let now = Clock::get()?.unix_timestamp;

    // ───── 2. Pool gates and reveal-window check ───────────────────────
    let pool_tier: Tier;
    let contribution_amount: u64;
    let pool_protocol_fee_bps: u64;
    let pool_reserve_fee_bps: u64;
    let current_month: u8;
    {
        let pool = &ctx.accounts.pool;
        require!(!pool.is_complete, CoreError::PoolComplete);
        require!(
            pool.current_month >= 1 && pool.current_month <= Pool::TOTAL_MONTHS,
            CoreError::PoolNotStarted
        );

        // Distinct error per side of the reveal window so the client UX
        // can show "wait for commit window to close" vs "reveal expired".
        require!(now >= pool.bid_window_ends_at, CoreError::BidWindowOpen);
        require!(
            now < pool.reveal_window_ends_at,
            CoreError::BidWindowClosed
        );

        pool_tier = pool.tier;
        contribution_amount = pool.contribution_amount;
        current_month = pool.current_month;

        // Re-read fee bps from the protocol config so the math is
        // identical to `contribute` / `join_pool`. We reload the cfg
        // here (cheap; we just deserialized it for the pause check —
        // re-deserializing keeps the local scope tight).
        let acct = &ctx.accounts.protocol_config;
        let mut data: &[u8] = &acct.try_borrow_data()?;
        let cfg = ProtocolConfig::try_deserialize(&mut data)?;
        pool_protocol_fee_bps = cfg.protocol_fee_bps as u64;
        pool_reserve_fee_bps = match pool_tier {
            Tier::Vault => cfg.vault_reserve_fee_bps,
            Tier::DeFi => cfg.defi_reserve_fee_bps,
        } as u64;
    }

    // ───── 3. Participant gates (defence-in-depth) ─────────────────────
    {
        let p = &ctx.accounts.participant;
        require!(!p.is_defaulted, CoreError::Defaulted);
        require!(!p.is_suspended, CoreError::Suspended);
        require!(!p.has_won, CoreError::AlreadyWon);
    }

    // ───── 4. Bid record gates ─────────────────────────────────────────
    {
        let b = &ctx.accounts.bid;
        require!(b.month == current_month, CoreError::BidWindowClosed);
        require!(!b.revealed, CoreError::AlreadyRevealed);
    }

    // ───── 5. Hash verification (INV-14) ───────────────────────────────
    // sha256(bid_amount.to_le_bytes() (8) || nonce ([u8;16]) ||
    //        user.key().to_bytes() (32)) — total 56-byte input. Spec §3
    // Bid `commit_hash` definition. Length is fixed by construction so
    // no length prefix is needed.
    let user_key_bytes = ctx.accounts.user.key().to_bytes();
    let bid_amount_bytes = bid_amount.to_le_bytes();
    // hashv concatenates the slices internally — equivalent to
    // sha256(bid_amount.to_le_bytes() || nonce || user_pubkey) but avoids
    // building a 56-byte stack buffer.
    let computed = hashv(&[&bid_amount_bytes, &nonce, &user_key_bytes]).to_bytes();
    require!(
        computed == ctx.accounts.bid.commit_hash,
        CoreError::BidRevealMismatch
    );

    // ───── 6. Bid amount validation (Q-9 + INV-15) ─────────────────────
    require!(bid_amount > 0, CoreError::InvalidAmount);

    // Q-10 monthly pot: `12 × (contrib − protocol_fee − reserve_fee)`.
    // Compute fees the SAME WAY as `contribute` / `join_pool` so the
    // pot definition stays consistent across the codebase.
    let protocol_fee = contribution_amount
        .checked_mul(pool_protocol_fee_bps)
        .and_then(|v| v.checked_div(BPS_DENOMINATOR))
        .ok_or(CoreError::MathOverflow)?;
    let reserve_fee = contribution_amount
        .checked_mul(pool_reserve_fee_bps)
        .and_then(|v| v.checked_div(BPS_DENOMINATOR))
        .ok_or(CoreError::MathOverflow)?;
    let net_contribution = contribution_amount
        .checked_sub(protocol_fee)
        .and_then(|v| v.checked_sub(reserve_fee))
        .ok_or(CoreError::MathOverflow)?;
    let monthly_pot = (POOL_SIZE as u64)
        .checked_mul(net_contribution)
        .ok_or(CoreError::MathOverflow)?;
    // `bid_cap = monthly_pot * 2000 / 10_000` (20%). Q-10 + spec §4.
    let bid_cap = monthly_pot
        .checked_mul(BID_CAP_BPS)
        .and_then(|v| v.checked_div(BPS_DENOMINATOR))
        .ok_or(CoreError::MathOverflow)?;
    require!(bid_amount <= bid_cap, CoreError::BidExceedsCap);

    // ───── 7. State updates (before token movement so partial-failure ──
    //          on the refund leaves a sane on-chain trace). The Bid PDA
    //          is owned by core so the writes are self-contained; the
    //          token move below is the only way to fail after this
    //          point. If the refund CPI errors, the entire tx reverts
    //          and `revealed = true` is rolled back.
    let bid = &mut ctx.accounts.bid;
    bid.revealed = true;
    bid.revealed_amount = bid_amount;
    bid.revealed_at = now;

    // ───── 8. Stake refund — bid_stake_vault → user_usdc ───────────────
    let pool_key = ctx.accounts.pool.key();
    let stake_amount = bid.stake_amount;
    if stake_amount > 0 {
        let bid_stake_bump = ctx.bumps.bid_stake_vault;
        let seeds: &[&[&[u8]]] = &[&[
            BID_STAKE_VAULT_SEED,
            pool_key.as_ref(),
            &[bid_stake_bump],
        ]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info().key(),
            Transfer {
                from: ctx.accounts.bid_stake_vault.to_account_info(),
                to: ctx.accounts.user_usdc.to_account_info(),
                authority: ctx.accounts.bid_stake_vault.to_account_info(),
            },
            seeds,
        );
        token::transfer(cpi_ctx, stake_amount)?;
    }
    bid.stake_refunded = true;

    emit!(BidRevealed {
        pool: pool_key,
        user: ctx.accounts.user.key(),
        month: current_month,
        bid_amount,
        timestamp: now,
    });

    Ok(())
}
