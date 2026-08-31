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
    amount: i128,
    total_swapped: i128,
    refundable: i128,
    net_swapped: i128,
}

#[contractevent(topics = ["refund_blnd"], data_format = "vec")]
struct RefundBlndEvent {
    #[topic]
    user: Address,
    amount: i128,
    total_refunded: i128,
    refundable: i128,
    net_swapped: i128,
}

#[contractevent(topics = ["burn_expired"], data_format = "vec")]
struct BurnExpiredEvent {
    blnd: i128,
    blnt: i128,
    net_swapped: i128,
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
    amount: i128,
    total_swapped: i128,
    refundable: i128,
    net_swapped: i128,
) {
    SwapBlndEvent {
        user,
        amount,
        total_swapped,
        refundable,
        net_swapped,
    }
    .publish(e);
}

pub fn refund_blnd(
    e: &Env,
    user: Address,
    amount: i128,
    total_refunded: i128,
    refundable: i128,
    net_swapped: i128,
) {
    RefundBlndEvent {
        user,
        amount,
        total_refunded,
        refundable,
        net_swapped,
    }
    .publish(e);
}

pub fn burn_expired(e: &Env, blnd: i128, blnt: i128, net_swapped: i128) {
    BurnExpiredEvent {
        blnd,
        blnt,
        net_swapped,
    }
    .publish(e);
}
