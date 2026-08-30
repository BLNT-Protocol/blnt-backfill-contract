use soroban_sdk::{contracttype, unwrap::UnwrapOptimized, Address, Env, Map};

const ONE_DAY_LEDGERS: u32 = 17_280;
const TTL_THRESHOLD: u32 = 90 * ONE_DAY_LEDGERS;
const TTL_BUMP: u32 = 180 * ONE_DAY_LEDGERS;

#[derive(Clone)]
#[contracttype]
enum InstanceKey {
    LegacyBlnd,
    Blnt,
    TotalAllocated,
    TotalClaimed,
    TotalSwapped,
    Lock,
}

#[derive(Clone)]
#[contracttype]
enum PersistentKey {
    Claims,
}

pub fn extend_instance(e: &Env) {
    e.storage().instance().extend_ttl(TTL_THRESHOLD, TTL_BUMP);
}

pub fn get_legacy_blnd(e: &Env) -> Address {
    e.storage()
        .instance()
        .get(&InstanceKey::LegacyBlnd)
        .unwrap_optimized()
}

pub fn set_legacy_blnd(e: &Env, token: &Address) {
    e.storage().instance().set(&InstanceKey::LegacyBlnd, token);
}

pub fn get_blnt(e: &Env) -> Address {
    e.storage()
        .instance()
        .get(&InstanceKey::Blnt)
        .unwrap_optimized()
}

pub fn set_blnt(e: &Env, token: &Address) {
    e.storage().instance().set(&InstanceKey::Blnt, token);
}

pub fn get_total_allocated(e: &Env) -> i128 {
    e.storage()
        .instance()
        .get(&InstanceKey::TotalAllocated)
        .unwrap_or(0)
}

pub fn set_total_allocated(e: &Env, amount: i128) {
    e.storage()
        .instance()
        .set(&InstanceKey::TotalAllocated, &amount);
}

pub fn get_total_claimed(e: &Env) -> i128 {
    e.storage()
        .instance()
        .get(&InstanceKey::TotalClaimed)
        .unwrap_or(0)
}

pub fn set_total_claimed(e: &Env, amount: i128) {
    e.storage()
        .instance()
        .set(&InstanceKey::TotalClaimed, &amount);
}

pub fn get_total_swapped(e: &Env) -> i128 {
    e.storage()
        .instance()
        .get(&InstanceKey::TotalSwapped)
        .unwrap_or(0)
}

pub fn set_total_swapped(e: &Env, amount: i128) {
    e.storage()
        .instance()
        .set(&InstanceKey::TotalSwapped, &amount);
}

pub fn get_lock(e: &Env) -> bool {
    e.storage()
        .instance()
        .get(&InstanceKey::Lock)
        .unwrap_or(false)
}

pub fn set_lock(e: &Env, locked: bool) {
    e.storage().instance().set(&InstanceKey::Lock, &locked);
}

pub fn set_claims(e: &Env, claims: &Map<Address, i128>) {
    let key = PersistentKey::Claims;
    e.storage().persistent().set(&key, claims);
    e.storage()
        .persistent()
        .extend_ttl(&key, TTL_THRESHOLD, TTL_BUMP);
}

pub fn get_claims(e: &Env) -> Map<Address, i128> {
    let key = PersistentKey::Claims;
    let claims = e
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| Map::new(e));
    e.storage()
        .persistent()
        .extend_ttl(&key, TTL_THRESHOLD, TTL_BUMP);
    claims
}
