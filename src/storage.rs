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
    BackfillAllocated,
    GrantAllocated,
    TotalClaimed,
    BackfillClaimed,
    GrantClaimed,
    TotalSwapped,
    SwapDeadline,
    ExpiredBlntBurned,
    VestingStart,
    VestingEnd,
    GrantVestingEnd,
    Lock,
}

#[derive(Clone)]
#[contracttype]
enum PersistentKey {
    BackfillClaims,
    BackfillProgress,
    GrantClaims,
    GrantProgress,
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

pub fn get_backfill_allocated(e: &Env) -> i128 {
    e.storage()
        .instance()
        .get(&InstanceKey::BackfillAllocated)
        .unwrap_or(0)
}

pub fn set_backfill_allocated(e: &Env, amount: i128) {
    e.storage()
        .instance()
        .set(&InstanceKey::BackfillAllocated, &amount);
}

pub fn get_grant_allocated(e: &Env) -> i128 {
    e.storage()
        .instance()
        .get(&InstanceKey::GrantAllocated)
        .unwrap_or(0)
}

pub fn set_grant_allocated(e: &Env, amount: i128) {
    e.storage()
        .instance()
        .set(&InstanceKey::GrantAllocated, &amount);
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

pub fn get_backfill_claimed(e: &Env) -> i128 {
    e.storage()
        .instance()
        .get(&InstanceKey::BackfillClaimed)
        .unwrap_or(0)
}

pub fn set_backfill_claimed(e: &Env, amount: i128) {
    e.storage()
        .instance()
        .set(&InstanceKey::BackfillClaimed, &amount);
}

pub fn get_grant_claimed(e: &Env) -> i128 {
    e.storage()
        .instance()
        .get(&InstanceKey::GrantClaimed)
        .unwrap_or(0)
}

pub fn set_grant_claimed(e: &Env, amount: i128) {
    e.storage()
        .instance()
        .set(&InstanceKey::GrantClaimed, &amount);
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

pub fn get_swap_deadline(e: &Env) -> u64 {
    e.storage()
        .instance()
        .get(&InstanceKey::SwapDeadline)
        .unwrap_optimized()
}

pub fn set_swap_deadline(e: &Env, timestamp: u64) {
    e.storage()
        .instance()
        .set(&InstanceKey::SwapDeadline, &timestamp);
}

pub fn get_expired_blnt_burned(e: &Env) -> i128 {
    e.storage()
        .instance()
        .get(&InstanceKey::ExpiredBlntBurned)
        .unwrap_or(0)
}

pub fn set_expired_blnt_burned(e: &Env, amount: i128) {
    e.storage()
        .instance()
        .set(&InstanceKey::ExpiredBlntBurned, &amount);
}

pub fn get_vesting_start(e: &Env) -> u64 {
    e.storage()
        .instance()
        .get(&InstanceKey::VestingStart)
        .unwrap_optimized()
}

pub fn set_vesting_start(e: &Env, timestamp: u64) {
    e.storage()
        .instance()
        .set(&InstanceKey::VestingStart, &timestamp);
}

pub fn get_vesting_end(e: &Env) -> u64 {
    e.storage()
        .instance()
        .get(&InstanceKey::VestingEnd)
        .unwrap_optimized()
}

pub fn set_vesting_end(e: &Env, timestamp: u64) {
    e.storage()
        .instance()
        .set(&InstanceKey::VestingEnd, &timestamp);
}

pub fn get_grant_vesting_end(e: &Env) -> u64 {
    e.storage()
        .instance()
        .get(&InstanceKey::GrantVestingEnd)
        .unwrap_optimized()
}

pub fn set_grant_vesting_end(e: &Env, timestamp: u64) {
    e.storage()
        .instance()
        .set(&InstanceKey::GrantVestingEnd, &timestamp);
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

pub fn set_backfill_claims(e: &Env, claims: &Map<Address, i128>) {
    let key = PersistentKey::BackfillClaims;
    e.storage().persistent().set(&key, claims);
    e.storage()
        .persistent()
        .extend_ttl(&key, TTL_THRESHOLD, TTL_BUMP);
}

pub fn get_backfill_claims(e: &Env) -> Map<Address, i128> {
    let key = PersistentKey::BackfillClaims;
    let claims = e.storage().persistent().get(&key).unwrap_optimized();
    e.storage()
        .persistent()
        .extend_ttl(&key, TTL_THRESHOLD, TTL_BUMP);
    claims
}

pub fn set_backfill_progress(e: &Env, progress: &Map<Address, i128>) {
    let key = PersistentKey::BackfillProgress;
    e.storage().persistent().set(&key, progress);
    e.storage()
        .persistent()
        .extend_ttl(&key, TTL_THRESHOLD, TTL_BUMP);
}

pub fn get_backfill_progress(e: &Env) -> Map<Address, i128> {
    let key = PersistentKey::BackfillProgress;
    let progress = e.storage().persistent().get(&key).unwrap_optimized();
    e.storage()
        .persistent()
        .extend_ttl(&key, TTL_THRESHOLD, TTL_BUMP);
    progress
}

pub fn set_grant_claims(e: &Env, claims: &Map<Address, i128>) {
    let key = PersistentKey::GrantClaims;
    e.storage().persistent().set(&key, claims);
    e.storage()
        .persistent()
        .extend_ttl(&key, TTL_THRESHOLD, TTL_BUMP);
}

pub fn get_grant_claims(e: &Env) -> Map<Address, i128> {
    let key = PersistentKey::GrantClaims;
    let claims = e.storage().persistent().get(&key).unwrap_optimized();
    e.storage()
        .persistent()
        .extend_ttl(&key, TTL_THRESHOLD, TTL_BUMP);
    claims
}

pub fn set_grant_progress(e: &Env, progress: &Map<Address, i128>) {
    let key = PersistentKey::GrantProgress;
    e.storage().persistent().set(&key, progress);
    e.storage()
        .persistent()
        .extend_ttl(&key, TTL_THRESHOLD, TTL_BUMP);
}

pub fn get_grant_progress(e: &Env) -> Map<Address, i128> {
    let key = PersistentKey::GrantProgress;
    let progress = e.storage().persistent().get(&key).unwrap_optimized();
    e.storage()
        .persistent()
        .extend_ttl(&key, TTL_THRESHOLD, TTL_BUMP);
    progress
}
