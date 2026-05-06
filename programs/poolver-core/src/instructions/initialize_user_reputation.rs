use anchor_lang::prelude::*;

use crate::constants::REPUTATION_SEED;
use crate::events::UserReputationInitialized;
use crate::state::{KycLevel, UserReputation};

/// Add-on instruction (not in spec §5.1) created because of the
/// `init_if_needed` ban (spec §9.10, SPEC_QUESTION-12). The user calls
/// this once before their first `join_pool`. The `init` constraint
/// makes the second call fail loudly, which gives us idempotent-by-
/// rejection semantics (acceptable for V1).
///
/// SPEC_QUESTION-26: production may want to fold this into the
/// `issue_kyc_attestation` flow — at attestation issuance the oracle
/// would also seed the reputation account. Defer to V2.
#[derive(Accounts)]
pub struct InitializeUserReputation<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init,
        payer = user,
        space = 8 + UserReputation::INIT_SPACE,
        seeds = [REPUTATION_SEED, user.key().as_ref()],
        bump,
    )]
    pub reputation: Account<'info, UserReputation>,

    pub system_program: Program<'info, System>,
}

pub fn handle_initialize_user_reputation(
    ctx: Context<InitializeUserReputation>,
) -> Result<()> {
    let rep = &mut ctx.accounts.reputation;
    rep.user = ctx.accounts.user.key();
    rep.pools_joined = 0;
    rep.pools_completed = 0;
    rep.pools_defaulted = 0;
    rep.total_contributed_lifetime = 0;
    rep.total_received_lifetime = 0;
    rep.kyc_status = KycLevel::None.as_u8();
    rep.kyc_attestation = Pubkey::default();
    rep.last_kyc_at = 0;
    rep.bump = ctx.bumps.reputation;

    emit!(UserReputationInitialized {
        user: rep.user,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}
