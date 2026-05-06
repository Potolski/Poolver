use anchor_lang::prelude::*;
use std::cell::Ref;

use crate::error::ConsolError;

/// Minimal Switchboard RandomnessAccountData parser.
/// Avoids pulling in the full `switchboard-on-demand` crate which has
/// borsh conflicts with anchor-lang 1.0.
///
/// Layout derived from:
/// https://github.com/switchboard-xyz/solana-sdk (switchboard-on-demand 0.11.3)
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct RandomnessAccountData {
    /// Authority that can update this randomness account
    pub authority: [u8; 32],
    /// Queue this randomness account belongs to
    pub queue: [u8; 32],
    /// Slot hash used as randomness seed
    pub seed_slothash: [u8; 32],
    /// Slot number used as randomness seed
    pub seed_slot: u64,
    /// Oracle that provided the randomness
    pub oracle: [u8; 32],
    /// Slot at which randomness was revealed
    pub reveal_slot: u64,
    /// The random value (32 bytes)
    pub value: [u8; 32],
    _ebuf2: [u8; 96],
    _ebuf1: [u8; 128],
}

/// Switchboard Randomness account discriminator
const RANDOMNESS_DISCRIMINATOR: [u8; 8] = [10, 66, 229, 135, 220, 239, 217, 114];

impl RandomnessAccountData {
    /// Parse from raw account data bytes.
    pub fn parse<'a>(data: Ref<'a, &mut [u8]>) -> Result<Ref<'a, Self>> {
        let expected_size = std::mem::size_of::<Self>() + 8;
        require!(
            data.len() >= expected_size,
            ConsolError::InvalidRandomnessAccount
        );

        // Check discriminator
        let mut disc = [0u8; 8];
        disc.copy_from_slice(&data[..8]);
        require!(
            disc == RANDOMNESS_DISCRIMINATOR,
            ConsolError::InvalidRandomnessAccount
        );

        let slice = &data[8..expected_size];
        bytemuck::try_from_bytes::<Self>(slice)
            .map_err(|_| error!(ConsolError::InvalidRandomnessAccount))?;

        Ok(Ref::map(data, |d: &&mut [u8]| {
            bytemuck::from_bytes(&d[8..expected_size])
        }))
    }

    /// Get the 32-byte random value. Valid once the oracle has revealed (reveal_slot > 0).
    pub fn get_value(&self, _clock_slot: u64) -> Result<[u8; 32]> {
        require!(
            self.is_revealed(),
            ConsolError::VrfNotResolved
        );
        Ok(self.value)
    }

    /// Check if randomness has been revealed (reveal_slot > 0 and value is non-zero).
    pub fn is_revealed(&self) -> bool {
        self.reveal_slot > 0 && self.value != [0u8; 32]
    }
}
