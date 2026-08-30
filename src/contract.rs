use crate::{
    errors::BackfillError, events, storage, BACKFILL_CAP, CLAIM_CAP, GRANT_CAP,
    GRANT_VESTING_DURATION_SECONDS, MAX_CLAIMANTS, MAX_GRANTEES, SWAP_CAP,
    VESTING_DURATION_SECONDS,
};
use soroban_sdk::{
    contract, contractimpl, panic_with_error, token::TokenClient, Address, Env, Executable, Map,
    Vec,
};

#[contract]
pub struct BlntBackfillContract;

#[derive(Clone, Copy)]
enum ClaimLane {
    Backfill,
    Grant,
}

fn require_unlocked(e: &Env) {
    if storage::get_lock(e) {
        panic_with_error!(e, BackfillError::ReentrantCall);
    }
}

fn require_external_recipient(e: &Env, to: &Address) {
    if *to == e.current_contract_address() {
        panic_with_error!(e, BackfillError::InvalidRecipient);
    }
}

fn transfer_blnt(e: &Env, to: &Address, amount: i128) {
    let contract = e.current_contract_address();
    let blnt = TokenClient::new(e, &storage::get_blnt(e));
    let contract_before = blnt.balance(&contract);
    let recipient_before = blnt.balance(to);

    blnt.transfer(&contract, to, &amount);

    if contract_before.checked_sub(blnt.balance(&contract)) != Some(amount)
        || blnt.balance(to).checked_sub(recipient_before) != Some(amount)
    {
        panic_with_error!(e, BackfillError::BalanceMismatch);
    }
}

fn vested_amount(e: &Env, allocation: i128, vesting_end: u64) -> i128 {
    let now = e.ledger().timestamp();
    let start = storage::get_vesting_start(e);
    if now <= start {
        return 0;
    }
    if now >= vesting_end {
        return allocation;
    }

    let duration = vesting_end
        .checked_sub(start)
        .filter(|duration| *duration > 0)
        .unwrap_or_else(|| panic_with_error!(e, BackfillError::Overflow));
    allocation
        .checked_mul(i128::from(now - start))
        .and_then(|value| value.checked_div(i128::from(duration)))
        .unwrap_or_else(|| panic_with_error!(e, BackfillError::Overflow))
}

fn claimable_amount(e: &Env, allocation: i128, claimed: i128, vesting_end: u64) -> i128 {
    vested_amount(e, allocation, vesting_end)
        .checked_sub(claimed)
        .filter(|amount| *amount >= 0)
        .unwrap_or_else(|| panic_with_error!(e, BackfillError::Overflow))
}

fn execute_claim(e: &Env, claimant: &Address, to: &Address, lane: ClaimLane) -> i128 {
    storage::extend_instance(e);
    require_unlocked(e);
    require_external_recipient(e, to);
    claimant.require_auth();

    let mut claims = match lane {
        ClaimLane::Backfill => storage::get_backfill_claims(e),
        ClaimLane::Grant => storage::get_grant_claims(e),
    };
    let allocation = claims.get(claimant.clone()).unwrap_or_else(|| match lane {
        ClaimLane::Backfill => panic_with_error!(e, BackfillError::NoClaim),
        ClaimLane::Grant => panic_with_error!(e, BackfillError::NoGrant),
    });
    let mut progress = match lane {
        ClaimLane::Backfill => storage::get_backfill_progress(e),
        ClaimLane::Grant => storage::get_grant_progress(e),
    };
    let previously_claimed = progress.get(claimant.clone()).unwrap_or(0);
    let vesting_end = match lane {
        ClaimLane::Backfill => storage::get_vesting_end(e),
        ClaimLane::Grant => storage::get_grant_vesting_end(e),
    };
    let amount = claimable_amount(e, allocation, previously_claimed, vesting_end);
    if amount == 0 {
        match lane {
            ClaimLane::Backfill => panic_with_error!(e, BackfillError::NothingClaimable),
            ClaimLane::Grant => panic_with_error!(e, BackfillError::NothingGrantClaimable),
        }
    }
    let claimant_total = previously_claimed
        .checked_add(amount)
        .filter(|claimed| *claimed <= allocation)
        .unwrap_or_else(|| panic_with_error!(e, BackfillError::Overflow));
    let claimed = storage::get_total_claimed(e)
        .checked_add(amount)
        .unwrap_or_else(|| panic_with_error!(e, BackfillError::Overflow));
    let lane_claimed = match lane {
        ClaimLane::Backfill => storage::get_backfill_claimed(e),
        ClaimLane::Grant => storage::get_grant_claimed(e),
    }
    .checked_add(amount)
    .unwrap_or_else(|| panic_with_error!(e, BackfillError::Overflow));

    storage::set_lock(e, true);
    if claimant_total == allocation {
        claims.remove(claimant.clone());
        progress.remove(claimant.clone());
        match lane {
            ClaimLane::Backfill => storage::set_backfill_claims(e, &claims),
            ClaimLane::Grant => storage::set_grant_claims(e, &claims),
        }
    } else {
        progress.set(claimant.clone(), claimant_total);
    }
    match lane {
        ClaimLane::Backfill => {
            storage::set_backfill_progress(e, &progress);
            storage::set_backfill_claimed(e, lane_claimed);
        }
        ClaimLane::Grant => {
            storage::set_grant_progress(e, &progress);
            storage::set_grant_claimed(e, lane_claimed);
        }
    }
    storage::set_total_claimed(e, claimed);
    transfer_blnt(e, to, amount);
    storage::set_lock(e, false);

    match lane {
        ClaimLane::Backfill => events::claim(e, claimant.clone(), to.clone(), amount),
        ClaimLane::Grant => events::grant_claim(e, claimant.clone(), to.clone(), amount),
    }
    amount
}

#[contractimpl]
impl BlntBackfillContract {
    pub fn __constructor(
        e: Env,
        legacy_blnd_token: Address,
        blnt_token: Address,
        claim_list: Vec<(Address, i128)>,
        grant_list: Vec<(Address, i128)>,
    ) {
        let contract = e.current_contract_address();
        if legacy_blnd_token == blnt_token
            || legacy_blnd_token.executable() != Some(Executable::StellarAsset)
            || blnt_token.executable() != Some(Executable::StellarAsset)
            || TokenClient::new(&e, &legacy_blnd_token).decimals() != 7
            || TokenClient::new(&e, &blnt_token).decimals() != 7
        {
            panic_with_error!(&e, BackfillError::InvalidToken);
        }
        if claim_list.len() > MAX_CLAIMANTS {
            panic_with_error!(&e, BackfillError::TooManyClaimants);
        }
        if grant_list.len() > MAX_GRANTEES {
            panic_with_error!(&e, BackfillError::TooManyGrantees);
        }

        let mut claims = Map::new(&e);
        let mut backfill_total = 0_i128;
        for (claimant, amount) in claim_list {
            if claimant == contract || amount <= 0 {
                panic_with_error!(&e, BackfillError::InvalidClaimAmount);
            }
            if claims.contains_key(claimant.clone()) {
                panic_with_error!(&e, BackfillError::DuplicateClaimant);
            }
            backfill_total = backfill_total
                .checked_add(amount)
                .unwrap_or_else(|| panic_with_error!(&e, BackfillError::Overflow));
            if backfill_total > BACKFILL_CAP {
                panic_with_error!(&e, BackfillError::ClaimCapExceeded);
            }
            claims.set(claimant, amount);
        }

        let mut grants = Map::new(&e);
        let mut grant_total = 0_i128;
        for (grantee, amount) in grant_list {
            if grantee == contract || amount <= 0 {
                panic_with_error!(&e, BackfillError::InvalidClaimAmount);
            }
            if grants.contains_key(grantee.clone()) {
                panic_with_error!(&e, BackfillError::DuplicateGrantee);
            }
            grant_total = grant_total
                .checked_add(amount)
                .unwrap_or_else(|| panic_with_error!(&e, BackfillError::Overflow));
            if grant_total > GRANT_CAP {
                panic_with_error!(&e, BackfillError::GrantCapExceeded);
            }
            grants.set(grantee, amount);
        }
        let total = backfill_total
            .checked_add(grant_total)
            .filter(|total| *total <= CLAIM_CAP)
            .unwrap_or_else(|| panic_with_error!(&e, BackfillError::ClaimCapExceeded));

        let vesting_start = e.ledger().timestamp();
        let vesting_end = vesting_start
            .checked_add(VESTING_DURATION_SECONDS)
            .unwrap_or_else(|| panic_with_error!(&e, BackfillError::Overflow));
        let grant_vesting_end = vesting_start
            .checked_add(GRANT_VESTING_DURATION_SECONDS)
            .unwrap_or_else(|| panic_with_error!(&e, BackfillError::Overflow));

        storage::set_legacy_blnd(&e, &legacy_blnd_token);
        storage::set_blnt(&e, &blnt_token);
        storage::set_total_allocated(&e, total);
        storage::set_backfill_allocated(&e, backfill_total);
        storage::set_grant_allocated(&e, grant_total);
        storage::set_vesting_start(&e, vesting_start);
        storage::set_vesting_end(&e, vesting_end);
        storage::set_grant_vesting_end(&e, grant_vesting_end);
        storage::set_backfill_claims(&e, &claims);
        storage::set_backfill_progress(&e, &Map::new(&e));
        storage::set_grant_claims(&e, &grants);
        storage::set_grant_progress(&e, &Map::new(&e));
        storage::extend_instance(&e);
    }
}

#[contractimpl]
impl BlntBackfillContract {
    pub fn claim_backfill(e: Env, claimant: Address, to: Address) -> i128 {
        execute_claim(&e, &claimant, &to, ClaimLane::Backfill)
    }

    pub fn claim_grant(e: Env, grantee: Address, to: Address) -> i128 {
        execute_claim(&e, &grantee, &to, ClaimLane::Grant)
    }

    pub fn swap_blnd_for_blnt(e: Env, from: Address, to: Address, amount: i128) -> i128 {
        storage::extend_instance(&e);
        require_unlocked(&e);
        require_external_recipient(&e, &to);
        if amount <= 0 {
            panic_with_error!(&e, BackfillError::InvalidSwapAmount);
        }
        from.require_auth();

        let total = storage::get_total_swapped(&e)
            .checked_add(amount)
            .unwrap_or_else(|| panic_with_error!(&e, BackfillError::Overflow));
        if total > SWAP_CAP {
            panic_with_error!(&e, BackfillError::SwapCapExceeded);
        }

        storage::set_lock(&e, true);
        storage::set_total_swapped(&e, total);

        let contract = e.current_contract_address();
        let legacy = TokenClient::new(&e, &storage::get_legacy_blnd(&e));
        let legacy_before = legacy.balance(&contract);
        legacy.transfer(&from, &contract, &amount);
        if legacy.balance(&contract).checked_sub(legacy_before) != Some(amount) {
            panic_with_error!(&e, BackfillError::BalanceMismatch);
        }
        legacy.burn(&contract, &amount);
        if legacy.balance(&contract) != legacy_before {
            panic_with_error!(&e, BackfillError::BalanceMismatch);
        }

        transfer_blnt(&e, &to, amount);
        storage::set_lock(&e, false);

        events::swap_blnd(&e, from, to, amount, total);
        total
    }

    pub fn get_backfill_claimable(e: Env, claimant: Address) -> i128 {
        storage::extend_instance(&e);
        let allocation = storage::get_backfill_claims(&e).get(claimant.clone());
        match allocation {
            Some(allocation) => claimable_amount(
                &e,
                allocation,
                storage::get_backfill_progress(&e)
                    .get(claimant)
                    .unwrap_or(0),
                storage::get_vesting_end(&e),
            ),
            None => 0,
        }
    }

    pub fn get_grant_claimable(e: Env, grantee: Address) -> i128 {
        storage::extend_instance(&e);
        let allocation = storage::get_grant_claims(&e).get(grantee.clone());
        match allocation {
            Some(allocation) => claimable_amount(
                &e,
                allocation,
                storage::get_grant_progress(&e).get(grantee).unwrap_or(0),
                storage::get_grant_vesting_end(&e),
            ),
            None => 0,
        }
    }

    pub fn get_legacy_blnd_token(e: Env) -> Address {
        storage::extend_instance(&e);
        storage::get_legacy_blnd(&e)
    }

    pub fn get_blnt_token(e: Env) -> Address {
        storage::extend_instance(&e);
        storage::get_blnt(&e)
    }

    pub fn get_total_allocated(e: Env) -> i128 {
        storage::extend_instance(&e);
        storage::get_total_allocated(&e)
    }

    pub fn get_backfill_allocated(e: Env) -> i128 {
        storage::extend_instance(&e);
        storage::get_backfill_allocated(&e)
    }

    pub fn get_grant_allocated(e: Env) -> i128 {
        storage::extend_instance(&e);
        storage::get_grant_allocated(&e)
    }

    pub fn get_total_claimed(e: Env) -> i128 {
        storage::extend_instance(&e);
        storage::get_total_claimed(&e)
    }

    pub fn get_backfill_claimed(e: Env) -> i128 {
        storage::extend_instance(&e);
        storage::get_backfill_claimed(&e)
    }

    pub fn get_grant_claimed(e: Env) -> i128 {
        storage::extend_instance(&e);
        storage::get_grant_claimed(&e)
    }

    pub fn get_vesting_start(e: Env) -> u64 {
        storage::extend_instance(&e);
        storage::get_vesting_start(&e)
    }

    pub fn get_vesting_end(e: Env) -> u64 {
        storage::extend_instance(&e);
        storage::get_vesting_end(&e)
    }

    pub fn get_grant_vesting_start(e: Env) -> u64 {
        storage::extend_instance(&e);
        storage::get_vesting_start(&e)
    }

    pub fn get_grant_vesting_end(e: Env) -> u64 {
        storage::extend_instance(&e);
        storage::get_grant_vesting_end(&e)
    }

    pub fn get_total_swapped(e: Env) -> i128 {
        storage::extend_instance(&e);
        storage::get_total_swapped(&e)
    }

    pub fn get_remaining_swap_capacity(e: Env) -> i128 {
        storage::extend_instance(&e);
        SWAP_CAP
            .checked_sub(storage::get_total_swapped(&e))
            .unwrap_or_else(|| panic_with_error!(&e, BackfillError::Overflow))
    }
}

#[cfg(test)]
mod tests {
    use super::{BlntBackfillContract, BlntBackfillContractClient};
    use crate::{
        BACKFILL_CAP, CLAIM_CAP, GRANT_CAP, GRANT_VESTING_DURATION_SECONDS, MAX_CLAIMANTS,
        MAX_GRANTEES, SCALAR_7, SWAP_CAP, VESTING_DURATION_SECONDS,
    };
    use soroban_sdk::{
        testutils::{Address as _, Events, Ledger},
        token::{StellarAssetClient, TokenClient},
        vec, Address, Env, Vec,
    };

    struct Fixture {
        e: Env,
        contract: Address,
        legacy: Address,
        blnt: Address,
        claimant: Address,
        grantee: Address,
        user: Address,
        recipient: Address,
    }

    impl Fixture {
        fn create(claim_amount: i128) -> Self {
            Self::create_with_grant(claim_amount, 0)
        }

        fn create_with_grant(claim_amount: i128, grant_amount: i128) -> Self {
            let e = Env::default();
            e.mock_all_auths();
            e.ledger().set_timestamp(1_000);
            let legacy = e
                .register_stellar_asset_contract_v2(Address::generate(&e))
                .address();
            let blnt = e
                .register_stellar_asset_contract_v2(Address::generate(&e))
                .address();
            let claimant = Address::generate(&e);
            let grantee = claimant.clone();
            let user = Address::generate(&e);
            let recipient = Address::generate(&e);
            let claims = if claim_amount > 0 {
                vec![&e, (claimant.clone(), claim_amount)]
            } else {
                Vec::new(&e)
            };
            let grants = if grant_amount > 0 {
                vec![&e, (grantee.clone(), grant_amount)]
            } else {
                Vec::new(&e)
            };
            let contract = e.register(BlntBackfillContract, (&legacy, &blnt, claims, grants));
            StellarAssetClient::new(&e, &blnt).mint(&contract, &(50_000_000 * SCALAR_7));
            StellarAssetClient::new(&e, &legacy).mint(&user, &SWAP_CAP);
            Self {
                e,
                contract,
                legacy,
                blnt,
                claimant,
                grantee,
                user,
                recipient,
            }
        }

        fn client(&self) -> BlntBackfillContractClient<'_> {
            BlntBackfillContractClient::new(&self.e, &self.contract)
        }

        fn finish_vesting(&self) {
            self.e
                .ledger()
                .set_timestamp(self.client().get_vesting_end());
        }

        fn finish_grant_vesting(&self) {
            self.e
                .ledger()
                .set_timestamp(self.client().get_grant_vesting_end());
        }
    }

    fn constructor_panics(
        e: &Env,
        legacy: &Address,
        blnt: &Address,
        claims: Vec<(Address, i128)>,
        grants: Vec<(Address, i128)>,
    ) -> bool {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            e.register(BlntBackfillContract, (legacy, blnt, claims, grants));
        }))
        .is_err()
    }

    #[test]
    fn backfill_claims_only_the_linearly_vested_portion() {
        let allocation = i128::from(VESTING_DURATION_SECONDS);
        let fixture = Fixture::create(allocation);
        let blnt = TokenClient::new(&fixture.e, &fixture.blnt);
        let client = fixture.client();
        let start = client.get_vesting_start();
        let third = VESTING_DURATION_SECONDS / 3;

        assert_eq!(client.get_vesting_end(), start + VESTING_DURATION_SECONDS);
        assert_eq!(client.get_backfill_claimable(&fixture.claimant), 0);
        assert!(client
            .try_claim_backfill(&fixture.claimant, &fixture.recipient)
            .is_err());

        fixture.e.ledger().set_timestamp(start + third);
        assert_eq!(
            client.get_backfill_claimable(&fixture.claimant),
            i128::from(third)
        );
        assert_eq!(
            client.claim_backfill(&fixture.claimant, &fixture.recipient),
            i128::from(third)
        );
        assert_eq!(client.get_backfill_claimable(&fixture.claimant), 0);
        assert!(client
            .try_claim_backfill(&fixture.claimant, &fixture.recipient)
            .is_err());

        fixture.e.ledger().set_timestamp(start + 2 * third);
        assert_eq!(
            client.claim_backfill(&fixture.claimant, &fixture.recipient),
            i128::from(third)
        );

        fixture.e.ledger().set_timestamp(client.get_vesting_end());
        assert_eq!(
            client.claim_backfill(&fixture.claimant, &fixture.recipient),
            allocation - 2 * i128::from(third)
        );
        assert_eq!(client.get_backfill_claimable(&fixture.claimant), 0);
        assert_eq!(client.get_total_claimed(), allocation);
        assert_eq!(blnt.balance(&fixture.recipient), allocation);
        assert!(client
            .try_claim_backfill(&fixture.claimant, &fixture.recipient)
            .is_err());
    }

    #[test]
    fn grants_vest_linearly_over_two_years_without_backfill_bypass() {
        let allocation = i128::from(GRANT_VESTING_DURATION_SECONDS);
        let fixture = Fixture::create_with_grant(0, allocation);
        let client = fixture.client();
        let blnt = TokenClient::new(&fixture.e, &fixture.blnt);
        let start = client.get_grant_vesting_start();

        assert_eq!(client.get_grant_vesting_start(), client.get_vesting_start());
        assert_eq!(
            client.get_grant_vesting_end(),
            start + GRANT_VESTING_DURATION_SECONDS
        );
        assert_eq!(client.get_backfill_allocated(), 0);
        assert_eq!(client.get_grant_allocated(), allocation);
        assert_eq!(client.get_total_allocated(), allocation);
        assert_eq!(client.get_grant_claimable(&fixture.grantee), 0);
        assert!(client
            .try_claim_grant(&fixture.grantee, &fixture.recipient)
            .is_err());
        assert!(client
            .try_claim_backfill(&fixture.grantee, &fixture.recipient)
            .is_err());

        fixture
            .e
            .ledger()
            .set_timestamp(start + GRANT_VESTING_DURATION_SECONDS / 2);
        assert_eq!(client.get_grant_claimable(&fixture.grantee), allocation / 2);
        assert_eq!(
            client.claim_grant(&fixture.grantee, &fixture.recipient),
            allocation / 2
        );
        assert_eq!(client.get_grant_claimable(&fixture.grantee), 0);

        fixture.finish_grant_vesting();
        assert_eq!(
            client.claim_grant(&fixture.grantee, &fixture.recipient),
            allocation / 2
        );
        assert_eq!(client.get_grant_claimed(), allocation);
        assert_eq!(client.get_backfill_claimed(), 0);
        assert_eq!(client.get_total_claimed(), allocation);
        assert_eq!(blnt.balance(&fixture.recipient), allocation);
        assert!(client
            .try_claim_grant(&fixture.grantee, &fixture.recipient)
            .is_err());
    }

    #[test]
    fn swaps_one_to_one_burns_blnd_and_never_mints_blnt() {
        let fixture = Fixture::create(0);
        let legacy = TokenClient::new(&fixture.e, &fixture.legacy);
        let blnt = TokenClient::new(&fixture.e, &fixture.blnt);
        let contract_before = blnt.balance(&fixture.contract);
        let amount = 25 * SCALAR_7;

        assert_eq!(
            fixture
                .client()
                .swap_blnd_for_blnt(&fixture.user, &fixture.recipient, &amount),
            amount
        );
        assert_eq!(legacy.balance(&fixture.contract), 0);
        assert_eq!(legacy.balance(&fixture.user), SWAP_CAP - amount);
        assert_eq!(blnt.balance(&fixture.recipient), amount);
        assert_eq!(blnt.balance(&fixture.contract), contract_before - amount);
        assert_eq!(fixture.client().get_total_swapped(), amount);
        assert_eq!(
            fixture.client().get_remaining_swap_capacity(),
            SWAP_CAP - amount
        );
    }

    #[test]
    fn conversion_is_perpetual_and_stops_at_the_aggregate_cap() {
        let fixture = Fixture::create(0);
        fixture.e.ledger().set_timestamp(u64::MAX);
        fixture
            .client()
            .swap_blnd_for_blnt(&fixture.user, &fixture.recipient, &SWAP_CAP);
        assert_eq!(fixture.client().get_remaining_swap_capacity(), 0);
        assert!(fixture
            .client()
            .try_swap_blnd_for_blnt(&fixture.user, &fixture.recipient, &1)
            .is_err());
    }

    #[test]
    fn complete_claim_and_conversion_caps_conserve_full_funding() {
        let fixture = Fixture::create_with_grant(BACKFILL_CAP, GRANT_CAP);
        let blnt = TokenClient::new(&fixture.e, &fixture.blnt);
        fixture.finish_grant_vesting();

        assert_eq!(
            fixture
                .client()
                .claim_backfill(&fixture.claimant, &fixture.recipient),
            BACKFILL_CAP
        );
        assert_eq!(
            fixture
                .client()
                .claim_grant(&fixture.grantee, &fixture.recipient),
            GRANT_CAP
        );
        assert_eq!(
            fixture
                .client()
                .swap_blnd_for_blnt(&fixture.user, &fixture.user, &SWAP_CAP),
            SWAP_CAP
        );
        assert_eq!(fixture.client().get_total_claimed(), CLAIM_CAP);
        assert_eq!(fixture.client().get_total_swapped(), SWAP_CAP);
        assert_eq!(blnt.balance(&fixture.contract), 0);
    }

    #[test]
    fn claim_and_swap_require_source_authorization() {
        let e = Env::default();
        let legacy = e
            .register_stellar_asset_contract_v2(Address::generate(&e))
            .address();
        let blnt = e
            .register_stellar_asset_contract_v2(Address::generate(&e))
            .address();
        let claimant = Address::generate(&e);
        let grantee = Address::generate(&e);
        let user = Address::generate(&e);
        let recipient = Address::generate(&e);
        let contract = e.register(
            BlntBackfillContract,
            (
                &legacy,
                &blnt,
                vec![&e, (claimant.clone(), SCALAR_7)],
                vec![&e, (grantee.clone(), SCALAR_7)],
            ),
        );
        let client = BlntBackfillContractClient::new(&e, &contract);
        e.ledger().set_timestamp(client.get_grant_vesting_end());

        assert!(client.try_claim_backfill(&claimant, &recipient).is_err());
        assert!(client.try_claim_grant(&grantee, &recipient).is_err());
        assert!(client
            .try_swap_blnd_for_blnt(&user, &recipient, &SCALAR_7)
            .is_err());
        assert_eq!(client.get_backfill_claimable(&claimant), SCALAR_7);
        assert_eq!(client.get_total_swapped(), 0);
    }

    #[test]
    fn failed_payout_rolls_back_claim_and_conversion_accounting() {
        let fixture = Fixture::create(SCALAR_7);
        let blnt = TokenClient::new(&fixture.e, &fixture.blnt);
        let legacy = TokenClient::new(&fixture.e, &fixture.legacy);
        fixture.finish_vesting();
        blnt.burn(&fixture.contract, &blnt.balance(&fixture.contract));

        assert!(fixture
            .client()
            .try_claim_backfill(&fixture.claimant, &fixture.recipient)
            .is_err());
        assert_eq!(
            fixture.client().get_backfill_claimable(&fixture.claimant),
            SCALAR_7
        );
        assert_eq!(fixture.client().get_total_claimed(), 0);

        assert!(fixture
            .client()
            .try_swap_blnd_for_blnt(&fixture.user, &fixture.recipient, &SCALAR_7)
            .is_err());
        assert_eq!(fixture.client().get_total_swapped(), 0);
        assert_eq!(legacy.balance(&fixture.user), SWAP_CAP);
        assert_eq!(legacy.balance(&fixture.contract), 0);
    }

    #[test]
    fn rejects_nonpositive_swap_and_contract_recipient() {
        let fixture = Fixture::create(0);
        assert!(fixture
            .client()
            .try_swap_blnd_for_blnt(&fixture.user, &fixture.recipient, &0)
            .is_err());
        assert!(fixture
            .client()
            .try_swap_blnd_for_blnt(&fixture.user, &fixture.recipient, &-1)
            .is_err());
        assert!(fixture
            .client()
            .try_swap_blnd_for_blnt(&fixture.user, &fixture.contract, &1)
            .is_err());
    }

    #[test]
    fn rejects_invalid_claim_lists() {
        let e = Env::default();
        let legacy = e
            .register_stellar_asset_contract_v2(Address::generate(&e))
            .address();
        let blnt = e
            .register_stellar_asset_contract_v2(Address::generate(&e))
            .address();
        let claimant = Address::generate(&e);

        assert!(constructor_panics(
            &e,
            &legacy,
            &blnt,
            vec![&e, (claimant.clone(), 0_i128)],
            Vec::new(&e),
        ));
        assert!(constructor_panics(
            &e,
            &legacy,
            &blnt,
            vec![
                &e,
                (claimant.clone(), SCALAR_7),
                (claimant.clone(), SCALAR_7),
            ],
            Vec::new(&e),
        ));
        assert!(constructor_panics(
            &e,
            &legacy,
            &blnt,
            vec![&e, (claimant.clone(), BACKFILL_CAP + 1)],
            Vec::new(&e),
        ));
        assert!(constructor_panics(
            &e,
            &legacy,
            &blnt,
            Vec::new(&e),
            vec![&e, (claimant.clone(), GRANT_CAP + 1)],
        ));
        assert!(constructor_panics(
            &e,
            &legacy,
            &blnt,
            Vec::new(&e),
            vec![&e, (claimant.clone(), SCALAR_7), (claimant, SCALAR_7),],
        ));
    }

    #[test]
    fn rejects_too_many_claimants() {
        let e = Env::default();
        let legacy = e
            .register_stellar_asset_contract_v2(Address::generate(&e))
            .address();
        let blnt = e
            .register_stellar_asset_contract_v2(Address::generate(&e))
            .address();
        let mut claims = Vec::new(&e);
        for _ in 0..=MAX_CLAIMANTS {
            claims.push_back((Address::generate(&e), 1_i128));
        }
        assert!(constructor_panics(&e, &legacy, &blnt, claims, Vec::new(&e),));

        let mut grants = Vec::new(&e);
        for _ in 0..=MAX_GRANTEES {
            grants.push_back((Address::generate(&e), 1_i128));
        }
        assert!(constructor_panics(&e, &legacy, &blnt, Vec::new(&e), grants,));
    }

    #[test]
    fn accepts_snapshot_bound_with_all_claimants_partially_vested() {
        let e = Env::default();
        e.mock_all_auths();
        let legacy = e
            .register_stellar_asset_contract_v2(Address::generate(&e))
            .address();
        let blnt = e
            .register_stellar_asset_contract_v2(Address::generate(&e))
            .address();
        let mut claims = Vec::new(&e);
        let mut claimants = Vec::new(&e);
        for _ in 0..MAX_CLAIMANTS {
            let claimant = Address::generate(&e);
            claims.push_back((claimant.clone(), 2_i128));
            claimants.push_back(claimant);
        }

        let contract = e.register(
            BlntBackfillContract,
            (&legacy, &blnt, claims, Vec::<(Address, i128)>::new(&e)),
        );
        let client = BlntBackfillContractClient::new(&e, &contract);
        assert_eq!(client.get_total_allocated(), 2 * i128::from(MAX_CLAIMANTS));

        StellarAssetClient::new(&e, &blnt).mint(&contract, &(2 * i128::from(MAX_CLAIMANTS)));
        e.ledger()
            .set_timestamp(client.get_vesting_start() + VESTING_DURATION_SECONDS / 2);
        for claimant in claimants.iter() {
            assert_eq!(client.claim_backfill(&claimant, &claimant), 1);
        }
        assert_eq!(client.get_total_claimed(), i128::from(MAX_CLAIMANTS));

        let first = claimants.first().unwrap();
        e.ledger().set_timestamp(client.get_vesting_end());
        assert_eq!(client.claim_backfill(&first, &first), 1);
        assert_eq!(client.get_backfill_claimable(&first), 0);
    }

    #[test]
    fn binds_distinct_seven_decimal_sacs() {
        let fixture = Fixture::create(0);
        assert_eq!(fixture.client().get_legacy_blnd_token(), fixture.legacy);
        assert_eq!(fixture.client().get_blnt_token(), fixture.blnt);
        assert_eq!(fixture.client().get_total_allocated(), 0);

        let e = Env::default();
        let token = e
            .register_stellar_asset_contract_v2(Address::generate(&e))
            .address();
        assert!(constructor_panics(
            &e,
            &token,
            &token,
            Vec::<(Address, i128)>::new(&e),
            Vec::<(Address, i128)>::new(&e),
        ));
    }

    #[test]
    fn publishes_claim_and_swap_events() {
        let fixture = Fixture::create_with_grant(SCALAR_7, SCALAR_7);
        fixture.finish_grant_vesting();
        fixture
            .client()
            .claim_backfill(&fixture.claimant, &fixture.recipient);
        fixture
            .client()
            .claim_grant(&fixture.grantee, &fixture.recipient);
        let claim_events = fixture
            .e
            .events()
            .all()
            .filter_by_contract(&fixture.contract);
        assert_eq!(claim_events.events().len(), 1);

        fixture
            .client()
            .swap_blnd_for_blnt(&fixture.user, &fixture.recipient, &SCALAR_7);
        let swap_events = fixture
            .e
            .events()
            .all()
            .filter_by_contract(&fixture.contract);
        assert_eq!(swap_events.events().len(), 1);
    }
}
