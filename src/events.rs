use soroban_sdk::{contractevent, Address, Env};

#[contractevent(topics = ["claim"], data_format = "single-value")]
struct ClaimEvent {
    #[topic]
    claimant: Address,
    #[topic]
    to: Address,
    amount: i128,
}

#[contractevent(topics = ["claim_grant"], data_format = "single-value")]
struct GrantClaimEvent {
    #[topic]
    grantee: Address,
    #[topic]
    to: Address,
    amount: i128,
}

#[contractevent(topics = ["swap_blnd"], data_format = "vec")]
struct SwapBlndEvent {
    #[topic]
    from: Address,
    #[topic]
    to: Address,
    amount: i128,
    total_swapped: i128,
}

#[contractevent(topics = ["burn_expired"], data_format = "vec")]
struct BurnExpiredEvent {
    amount: i128,
    total_swapped: i128,
}

pub fn claim(e: &Env, claimant: Address, to: Address, amount: i128) {
    ClaimEvent {
        claimant,
        to,
        amount,
    }
    .publish(e);
}

pub fn grant_claim(e: &Env, grantee: Address, to: Address, amount: i128) {
    GrantClaimEvent {
        grantee,
        to,
        amount,
    }
    .publish(e);
}

pub fn swap_blnd(e: &Env, from: Address, to: Address, amount: i128, total_swapped: i128) {
    SwapBlndEvent {
        from,
        to,
        amount,
        total_swapped,
    }
    .publish(e);
}

pub fn burn_expired(e: &Env, amount: i128, total_swapped: i128) {
    BurnExpiredEvent {
        amount,
        total_swapped,
    }
    .publish(e);
}
