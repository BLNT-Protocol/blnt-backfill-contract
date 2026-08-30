#![no_std]

#[cfg(any(test, feature = "testutils"))]
extern crate std;

mod contract;
mod errors;
mod events;
mod storage;

pub use contract::{BlntBackfillContract, BlntBackfillContractClient};
pub use errors::BackfillError;

pub const SCALAR_7: i128 = 10_000_000;
pub const TOTAL_FUNDING: i128 = 50_000_000 * SCALAR_7;
pub const CLAIM_CAP: i128 = 30_000_000 * SCALAR_7;
pub const SWAP_CAP: i128 = 20_000_000 * SCALAR_7;
pub const MAX_CLAIMANTS: u32 = 100;
