use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum BackfillError {
    InvalidToken = 1200,
    TooManyClaimants = 1201,
    InvalidClaimAmount = 1202,
    DuplicateClaimant = 1203,
    ClaimCapExceeded = 1204,
    NoClaim = 1205,
    InvalidRecipient = 1206,
    InvalidSwapAmount = 1207,
    SwapCapExceeded = 1208,
    BalanceMismatch = 1209,
    ReentrantCall = 1210,
    Overflow = 1211,
    NothingClaimable = 1212,
    TooManyGrantees = 1213,
    DuplicateGrantee = 1214,
    GrantCapExceeded = 1215,
    NoGrant = 1216,
    NothingGrantClaimable = 1217,
    SwapExpired = 1218,
    SwapNotExpired = 1219,
}
