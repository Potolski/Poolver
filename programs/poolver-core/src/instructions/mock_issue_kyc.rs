// MOCK_KYC: V1-only instruction; replaced in production by
// `issue_kyc_attestation` signed by `protocol_config.kyc_oracle`. See
// `docs/mock-to-production.md` Site 1. The whole module is gated under
// `#[cfg(feature = "mock-kyc")]` at `instructions/mod.rs` and `lib.rs`,
// so a `--no-default-features` build excludes the dispatch entry, the
// IDL entry, and the symbol from the .so binary entirely (INV-26,
// arch §10).

use anchor_lang::prelude::*;

use crate::constants::{DEFAULT_KYC_VALIDITY_SECS, KYC_SEED, PROTOCOL_CONFIG_SEED};
use crate::error::CoreError;
use crate::events::KycAttestationIssued;
use crate::state::{KycAttestation, KycLevel, ProtocolConfig};

#[derive(Accounts)]
#[instruction(user: Pubkey)]
pub struct MockIssueKyc<'info> {
    /// Pays for and signs the attestation issuance. In V1 the admin and
    /// kyc_oracle are the same key (set by `initialize_protocol`), and
    /// the constraint below pins this to `protocol_config.kyc_oracle`.
    /// SPEC_QUESTION-26: production rotates kyc_oracle to a dedicated
    /// HSM-backed key; this signer constraint stays unchanged because
    /// the verifier reads kyc_oracle from on-chain config.
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [PROTOCOL_CONFIG_SEED],
        bump = protocol_config.bump,
        // MOCK_KYC: in V1, kyc_oracle == admin (set in initialize_protocol).
        // Production keeps this constraint identical — only the value of
        // protocol_config.kyc_oracle changes (it'll be the HSM-backed
        // oracle key).
        constraint = protocol_config.kyc_oracle == admin.key()
            @ CoreError::Unauthorized,
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,

    /// CHECK: the destination user; we don't dereference, just store the
    /// pubkey on the attestation. Anchor cannot type this as a SystemAccount
    /// because the user account need not exist on chain yet.
    pub user_pubkey: UncheckedAccount<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + KycAttestation::INIT_SPACE,
        seeds = [KYC_SEED, user_pubkey.key().as_ref()],
        bump,
    )]
    pub attestation: Account<'info, KycAttestation>,

    pub system_program: Program<'info, System>,
}

pub fn handle_mock_issue_kyc(
    ctx: Context<MockIssueKyc>,
    user: Pubkey,
    level: KycLevel,
) -> Result<()> {
    require!(
        matches!(level, KycLevel::Light | KycLevel::Full),
        CoreError::KycInsufficientLevel
    );
    // Defence-in-depth: the seed binds `attestation` to `user_pubkey`,
    // and we additionally check the explicit `user` arg against it so a
    // mismatch is rejected before any state writes.
    require_keys_eq!(
        ctx.accounts.user_pubkey.key(),
        user,
        CoreError::Unauthorized
    );

    let now = Clock::get()?.unix_timestamp;
    let expires_at = now
        .checked_add(DEFAULT_KYC_VALIDITY_SECS)
        .ok_or(CoreError::MathOverflow)?;

    let att = &mut ctx.accounts.attestation;
    att.user = user;
    att.level = level.as_u8();
    att.issued_by = ctx.accounts.admin.key();
    att.issued_at = now;
    att.expires_at = expires_at;
    // MOCK_KYC: cpf_hash zeroed in V1; production KYC oracle populates
    // from off-chain Idwall data.
    att.cpf_hash = [0u8; 32];
    // MOCK_KYC: sanctions_clean always true in V1; production KYC oracle
    // sets based on real screening result.
    att.sanctions_clean = true;
    att.bump = ctx.bumps.attestation;

    emit!(KycAttestationIssued {
        user,
        level,
        issued_by: att.issued_by,
        issued_at: now,
        expires_at,
        is_mock: true,
    });

    Ok(())
}
