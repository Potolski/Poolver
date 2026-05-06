use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, Transfer};

use crate::constants::{
    BID_SEED, BID_STAKE_BPS, BID_STAKE_VAULT_SEED, BPS_DENOMINATOR, KYC_SEED, PARTICIPANT_SEED,
    PROTOCOL_CONFIG_SEED,
};
use crate::error::CoreError;
use crate::events::BidCommitted;
use crate::kyc::require_full_kyc;
use crate::state::{Bid, KycAttestation, Participant, Pool, ProtocolConfig};

/// `commit_bid` — spec §5.1.
///
/// Submits a sealed bid for the current month's pot. The user precomputes
/// `commit_hash = sha256(bid_amount.to_le_bytes() || nonce ([u8;16]) ||
/// user_pubkey.to_bytes())` off-chain and submits only the hash here. The
/// matching `(bid_amount, nonce)` is opened in `reveal_bid` once the
/// commit window closes. Front-running is structurally prevented because
/// no other party (or MEV bot) can reconstruct the bid amount from the
/// hash alone — INV-14 / spec §9.8.
///
/// Pre-conditions (spec §5.1 + invariants):
///   - protocol not paused (INV-25)
///   - pool active, in months 1..=12, not complete
///   - inside commit window: `current_month_started_at <= now <
///     bid_window_ends_at`
///   - caller is a participant of this pool, not defaulted, not
///     suspended, has NOT won a previous month (INV-30 — `AlreadyWon`)
///   - caller has Full KYC (spec §5.1: "user has Full KYC (required for
///     win, so check now to avoid late surprise)") — INV-27 / §5
///   - `commit_hash != [0u8; 32]` (cheap sanity; an all-zero hash would
///     be the all-zero preimage's hash, which is statistically near
///     impossible and signals client-side error)
///   - INV-16: structurally enforced by the `init` constraint on `bid`.
///     A second commit for the same (pool, month, user) triple fails
///     with `AccountAlreadyInitialized`.
///
/// Token movement: 1% of `pool.contribution_amount` (Q-3, basis-points
/// constant `BID_STAKE_BPS`) is pulled from the user's USDC ATA into the
/// per-pool `bid_stake_vault` PDA. On successful `reveal_bid` the stake
/// is refunded; if the user fails to reveal in the 24h window
/// (SPEC_QUESTION-4), step 7's cleanup forfeits it to the tier reserve.
///
/// SPEC_QUESTION-15: `Pool` is `Box`'d. `protocol_config` and `user_kyc`
/// are manually deserialized inside the handler so the
/// `try_accounts`-time stack frame stays under the 4 KB BPF budget,
/// matching `join_pool` and `contribute`.
#[derive(Accounts)]
pub struct CommitBid<'info> {
    /// The bidder. Pays the `bid` PDA rent and signs the stake transfer.
    #[account(mut)]
    pub user: Signer<'info>,

    /// Protocol config — read solely for the pause flag (INV-25).
    /// Manually deserialized in the handler (SPEC_QUESTION-15).
    /// CHECK: PDA seed binding here, owner + discriminator validated
    /// inside the handler.
    #[account(seeds = [PROTOCOL_CONFIG_SEED], bump)]
    pub protocol_config: UncheckedAccount<'info>,

    /// Pool. Read-only here; we don't mutate any pool field.
    /// SPEC_QUESTION-15: `Box` to keep the stack frame lean.
    pub pool: Box<Account<'info, Pool>>,

    /// Per-(pool, user) participant. Verified via PDA seed binding.
    /// We rely on the seed match to enforce participation; a non-
    /// participant cannot construct a valid `Participant` PDA for this
    /// pool. The `pool` / `user` constraints below are belt-and-braces.
    #[account(
        seeds = [PARTICIPANT_SEED, pool.key().as_ref(), user.key().as_ref()],
        bump = participant.bump,
        constraint = participant.pool == pool.key() @ CoreError::NotAParticipant,
        constraint = participant.user == user.key() @ CoreError::NotAParticipant,
    )]
    pub participant: Box<Account<'info, Participant>>,

    /// User's KYC attestation. // MOCK_KYC: V1 attestations come from
    /// `mock_issue_kyc`; production attestations come from
    /// `issue_kyc_attestation`. Verification is identical (handled by
    /// `require_full_kyc` after manual deserialization — SPEC_QUESTION-15).
    /// CHECK: validated in the handler.
    #[account(seeds = [KYC_SEED, user.key().as_ref()], bump)]
    pub user_kyc: UncheckedAccount<'info>,

    /// Sealed-bid record for (pool, current_month, user). `init` makes
    /// double-commits impossible (INV-16).
    #[account(
        init,
        payer = user,
        space = 8 + Bid::INIT_SPACE,
        seeds = [
            BID_SEED,
            pool.key().as_ref(),
            &[pool.current_month],
            user.key().as_ref(),
        ],
        bump,
    )]
    pub bid: Box<Account<'info, Bid>>,

    /// User's USDC source for the 1% anti-spam stake.
    /// CHECK: SPL transfer enforces ownership and balance.
    #[account(mut)]
    pub user_usdc: UncheckedAccount<'info>,

    /// Per-pool bid-stake vault. PDA-owned token account; authority is
    /// the token account itself (its seeds sign for refunds in
    /// `reveal_bid`).
    /// CHECK: PDA seed binding ensures identity. `mut` because we deposit.
    #[account(
        mut,
        seeds = [BID_STAKE_VAULT_SEED, pool.key().as_ref()],
        bump,
    )]
    pub bid_stake_vault: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_commit_bid(
    ctx: Context<CommitBid>,
    commit_hash: [u8; 32],
) -> Result<()> {
    // ───── 0. Cheap sanity on the hash itself ──────────────────────────
    // An all-zero commit hash would mean the user effectively committed
    // nothing — likely a client serialization bug. Reject up-front.
    require!(commit_hash != [0u8; 32], CoreError::InvalidAmount);

    // ───── 1. Pause check (INV-25) — manual deser per SPEC_QUESTION-15 ─
    {
        let acct = &ctx.accounts.protocol_config;
        require_keys_eq!(*acct.owner, crate::ID, CoreError::Unauthorized);
        let mut data: &[u8] = &acct.try_borrow_data()?;
        let cfg = ProtocolConfig::try_deserialize(&mut data)?;
        require!(!cfg.paused, CoreError::ProtocolPaused);
    }

    let now = Clock::get()?.unix_timestamp;

    // ───── 2. Pool gates ───────────────────────────────────────────────
    let current_month: u8;
    let stake_amount: u64;
    {
        let pool = &ctx.accounts.pool;
        require!(!pool.is_complete, CoreError::PoolComplete);
        require!(
            pool.current_month >= 1 && pool.current_month <= Pool::TOTAL_MONTHS,
            CoreError::PoolNotStarted
        );
        // Commit window: [current_month_started_at, bid_window_ends_at).
        // After the bid window closes, only `reveal_bid` is allowed.
        require!(
            now >= pool.current_month_started_at && now < pool.bid_window_ends_at,
            CoreError::BidWindowClosed
        );
        current_month = pool.current_month;

        // Q-3: 1% of contribution_amount, basis-points math via the
        // standard checked u128-free path. `BID_STAKE_BPS = 100`.
        stake_amount = pool
            .contribution_amount
            .checked_mul(BID_STAKE_BPS)
            .and_then(|v| v.checked_div(BPS_DENOMINATOR))
            .ok_or(CoreError::MathOverflow)?;
    }

    // ───── 3. Participant gates (INV-30 / Q-11) ────────────────────────
    {
        let p = &ctx.accounts.participant;
        require!(!p.is_defaulted, CoreError::Defaulted);
        require!(!p.is_suspended, CoreError::Suspended);
        // INV-30: a prior winner cannot win twice. Spec §5.1: "user has
        // not won yet". Surface as `AlreadyWon` so the UX can present
        // the right message.
        require!(!p.has_won, CoreError::AlreadyWon);
    }

    // ───── 4. Full-KYC gate (spec §5.1, INV-27) ────────────────────────
    // MOCK_KYC: same verification helper for V1 mock + future real KYC.
    {
        let acct = &ctx.accounts.user_kyc;
        require_keys_eq!(*acct.owner, crate::ID, CoreError::Unauthorized);
        let mut data: &[u8] = &acct.try_borrow_data()?;
        let kyc: KycAttestation = KycAttestation::try_deserialize(&mut data)?;
        require_full_kyc(&kyc, &ctx.accounts.user.key(), now)?;
    }

    // ───── 5. Lock the 1% stake — user → bid_stake_vault ───────────────
    if stake_amount > 0 {
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info().key(),
            Transfer {
                from: ctx.accounts.user_usdc.to_account_info(),
                to: ctx.accounts.bid_stake_vault.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        token::transfer(cpi_ctx, stake_amount)?;
    }

    // ───── 6. Initialize Bid PDA ───────────────────────────────────────
    let pool_key = ctx.accounts.pool.key();
    let user_key = ctx.accounts.user.key();
    let bid = &mut ctx.accounts.bid;
    bid.pool = pool_key;
    bid.user = user_key;
    bid.month = current_month;
    bid.commit_hash = commit_hash;
    bid.committed_at = now;
    bid.stake_amount = stake_amount;
    bid.revealed = false;
    bid.revealed_amount = 0;
    bid.revealed_at = 0;
    bid.is_winner = false;
    bid.stake_refunded = false;
    bid.bump = ctx.bumps.bid;

    emit!(BidCommitted {
        pool: pool_key,
        user: user_key,
        month: current_month,
        commit_hash,
        stake_amount,
        timestamp: now,
    });

    Ok(())
}
