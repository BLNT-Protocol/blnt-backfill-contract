#![no_std]

#[cfg(any(test, feature = "testutils"))]
extern crate std;

mod contract;
mod errors;
mod events;
mod storage;

pub use contract::{BlntBackfillContract, BlntBackfillContractClient, ExpiredBurn};
pub use errors::BackfillError;

pub const SCALAR_7: i128 = 10_000_000;
pub const TOTAL_FUNDING: i128 = 150_000_000 * SCALAR_7;
pub const BACKFILL_CAP: i128 = 20_000_000 * SCALAR_7;
pub const GRANT_CAP: i128 = 10_000_000 * SCALAR_7;
pub const CLAIM_CAP: i128 = BACKFILL_CAP + GRANT_CAP;
pub const SWAP_CAP: i128 = 120_000_000 * SCALAR_7;
pub const MAX_CLAIMANTS: u32 = 434;
pub const MAX_GRANTEES: u32 = 100;
pub const VESTING_DURATION_SECONDS: u64 = 180 * 24 * 60 * 60;
pub const GRANT_VESTING_DURATION_SECONDS: u64 = 720 * 24 * 60 * 60;
pub const SWAP_DURATION_SECONDS: u64 = 270 * 24 * 60 * 60;
