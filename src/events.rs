use soroban_sdk::{contractevent, Address, Env};

#[contractevent(topics = ["claim"], data_format = "single-value")]
struct ClaimEvent {
    #[topic]
    user: Address,
    amount: i128,
}

#[contractevent(topics = ["claim_grant"], data_format = "single-value")]
struct GrantClaimEvent {
    #[topic]
    user: Address,
    amount: i128,
}

#[contractevent(topics = ["swap_blnd"], data_format = "vec")]
struct SwapBlndEvent {
    #[topic]
    user: Address,
    blnd_burned: i128,
    blnt_received: i128,
    total_blnt_swapped: i128,
}

#[contractevent(topics = ["burn_expired"], data_format = "vec")]
struct BurnExpiredEvent {
    blnt: i128,
    total_blnt_burned: i128,
}

pub fn claim(e: &Env, user: Address, amount: i128) {
    ClaimEvent { user, amount }.publish(e);
}

pub fn grant_claim(e: &Env, user: Address, amount: i128) {
    GrantClaimEvent { user, amount }.publish(e);
}

pub fn swap_blnd(
    e: &Env,
    user: Address,
    blnd_burned: i128,
    blnt_received: i128,
    total_blnt_swapped: i128,
) {
    SwapBlndEvent {
        user,
        blnd_burned,
        blnt_received,
        total_blnt_swapped,
    }
    .publish(e);
}

pub fn burn_expired(e: &Env, blnt: i128, total_blnt_burned: i128) {
    BurnExpiredEvent {
        blnt,
        total_blnt_burned,
    }
    .publish(e);
}
