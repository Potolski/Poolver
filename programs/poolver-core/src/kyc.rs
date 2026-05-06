//! KYC verification helpers.
//!
//! These helpers run identically against any `KycAttestation` PDA,
//! regardless of whether it was minted by `mock_issue_kyc` (V1 dev path)
//! or by the real `issue_kyc_attestation` (production). This is the
//! whole point of the mock pattern: zero production drift in the
//! verification side. Per `docs/mock-to-production.md` Site 6, this
//! file is intentionally NOT marked with `// MOCK_KYC:` — it does not
//! change between V1 and production.

use anchor_lang::prelude::*;

use crate::error::CoreError;
use crate::state::{KycAttestation, KycLevel};

/// Require that `attestation` is a Light-or-better, unexpired,
/// sanctions-clean attestation for `user` at time `now`.
pub fn require_light_kyc(
    attestation: &KycAttestation,
    user: &Pubkey,
    now: i64,
) -> Result<()> {
    require_keys_eq!(attestation.user, *user, CoreError::Unauthorized);
    require!(
        attestation.level >= KycLevel::Light.as_u8(),
        CoreError::KycInsufficientLevel
    );
    require!(attestation.expires_at > now, CoreError::KycExpired);
    require!(attestation.sanctions_clean, CoreError::KycSanctionsHit);
    Ok(())
}

/// Require that `attestation` is a Full-level, unexpired, sanctions-
/// clean attestation for `user` at time `now`. Reserved for instructions
/// that need it in later steps; included now so the surface is stable.
#[allow(dead_code)]
pub fn require_full_kyc(
    attestation: &KycAttestation,
    user: &Pubkey,
    now: i64,
) -> Result<()> {
    require_keys_eq!(attestation.user, *user, CoreError::Unauthorized);
    require!(
        attestation.level >= KycLevel::Full.as_u8(),
        CoreError::KycInsufficientLevel
    );
    require!(attestation.expires_at > now, CoreError::KycExpired);
    require!(attestation.sanctions_clean, CoreError::KycSanctionsHit);
    Ok(())
}
