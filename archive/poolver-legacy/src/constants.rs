use anchor_lang::prelude::*;

// PDA Seeds
#[constant]
pub const GROUP_SEED: &[u8] = b"group";
#[constant]
pub const MEMBER_SEED: &[u8] = b"member";
#[constant]
pub const ROUND_SEED: &[u8] = b"round";
#[constant]
pub const VAULT_SEED: &[u8] = b"vault";
#[constant]
pub const INSURANCE_SEED: &[u8] = b"insurance";
#[constant]
pub const REPUTATION_SEED: &[u8] = b"reputation";
#[constant]
pub const TREASURY_SEED: &[u8] = b"treasury";

// Protocol parameters
pub const MAX_GROUP_SIZE: u8 = 50;
pub const MIN_GROUP_SIZE: u8 = 3;
pub const MIN_CONTRIBUTION: u64 = 10_000_000; // 10 USDC (6 decimals)
pub const PAYMENT_WINDOW_DAYS: i64 = 7;
pub const GRACE_PERIOD_DAYS: i64 = 3;
pub const LATE_FEE_BPS: u16 = 500; // 5%
pub const PROTOCOL_FEE_BPS: u16 = 150; // 1.5%
pub const DEFAULT_COLLATERAL_BPS: u16 = 2000; // 20%
pub const DEFAULT_INSURANCE_BPS: u16 = 300; // 3%
pub const MAX_MISSED_PAYMENTS: u8 = 3;
pub const FORMATION_TIMEOUT_DAYS: i64 = 30;
