use crate::{errors::BackfillError, events, storage, CLAIM_CAP, MAX_CLAIMANTS, SWAP_CAP};
use soroban_sdk::{
    contract, contractimpl, panic_with_error, token::TokenClient, Address, Env, Executable, Map,
    Vec,
};

#[contract]
pub struct BlntBackfillContract;

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

#[contractimpl]
impl BlntBackfillContract {
    pub fn __constructor(
        e: Env,
        legacy_blnd_token: Address,
        blnt_token: Address,
        claim_list: Vec<(Address, i128)>,
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

        let mut claims = Map::new(&e);
        let mut total = 0_i128;
        for (claimant, amount) in claim_list {
            if claimant == contract || amount <= 0 {
                panic_with_error!(&e, BackfillError::InvalidClaimAmount);
            }
            if claims.contains_key(claimant.clone()) {
                panic_with_error!(&e, BackfillError::DuplicateClaimant);
            }
            total = total
                .checked_add(amount)
                .unwrap_or_else(|| panic_with_error!(&e, BackfillError::Overflow));
            if total > CLAIM_CAP {
                panic_with_error!(&e, BackfillError::ClaimCapExceeded);
            }
            claims.set(claimant, amount);
        }

        storage::set_legacy_blnd(&e, &legacy_blnd_token);
        storage::set_blnt(&e, &blnt_token);
        storage::set_total_allocated(&e, total);
        storage::set_claims(&e, &claims);
        storage::extend_instance(&e);
    }
}

#[contractimpl]
impl BlntBackfillContract {
    pub fn claim(e: Env, claimant: Address, to: Address) -> i128 {
        storage::extend_instance(&e);
        require_unlocked(&e);
        require_external_recipient(&e, &to);
        claimant.require_auth();

        let mut claims = storage::get_claims(&e);
        let amount = claims
            .get(claimant.clone())
            .unwrap_or_else(|| panic_with_error!(&e, BackfillError::NoClaim));
        let claimed = storage::get_total_claimed(&e)
            .checked_add(amount)
            .unwrap_or_else(|| panic_with_error!(&e, BackfillError::Overflow));

        storage::set_lock(&e, true);
        claims.remove(claimant.clone());
        storage::set_claims(&e, &claims);
        storage::set_total_claimed(&e, claimed);
        transfer_blnt(&e, &to, amount);
        storage::set_lock(&e, false);

        events::claim(&e, claimant, to, amount);
        amount
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

    pub fn get_claimable(e: Env, claimant: Address) -> i128 {
        storage::extend_instance(&e);
        storage::get_claims(&e).get(claimant).unwrap_or(0)
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

    pub fn get_total_claimed(e: Env) -> i128 {
        storage::extend_instance(&e);
        storage::get_total_claimed(&e)
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
    use crate::{CLAIM_CAP, MAX_CLAIMANTS, SCALAR_7, SWAP_CAP};
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
        user: Address,
        recipient: Address,
    }

    impl Fixture {
        fn create(claim_amount: i128) -> Self {
            let e = Env::default();
            e.mock_all_auths();
            let legacy = e
                .register_stellar_asset_contract_v2(Address::generate(&e))
                .address();
            let blnt = e
                .register_stellar_asset_contract_v2(Address::generate(&e))
                .address();
            let claimant = Address::generate(&e);
            let user = Address::generate(&e);
            let recipient = Address::generate(&e);
            let claims = if claim_amount > 0 {
                vec![&e, (claimant.clone(), claim_amount)]
            } else {
                Vec::new(&e)
            };
            let contract = e.register(BlntBackfillContract, (&legacy, &blnt, claims));
            StellarAssetClient::new(&e, &blnt).mint(&contract, &(50_000_000 * SCALAR_7));
            StellarAssetClient::new(&e, &legacy).mint(&user, &SWAP_CAP);
            Self {
                e,
                contract,
                legacy,
                blnt,
                claimant,
                user,
                recipient,
            }
        }

        fn client(&self) -> BlntBackfillContractClient<'_> {
            BlntBackfillContractClient::new(&self.e, &self.contract)
        }
    }

    fn constructor_panics(
        e: &Env,
        legacy: &Address,
        blnt: &Address,
        claims: Vec<(Address, i128)>,
    ) -> bool {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            e.register(BlntBackfillContract, (legacy, blnt, claims));
        }))
        .is_err()
    }

    #[test]
    fn claims_complete_allocation_once_to_selected_recipient() {
        let fixture = Fixture::create(12 * SCALAR_7);
        let blnt = TokenClient::new(&fixture.e, &fixture.blnt);

        assert_eq!(
            fixture.client().get_claimable(&fixture.claimant),
            12 * SCALAR_7
        );
        assert_eq!(
            fixture
                .client()
                .claim(&fixture.claimant, &fixture.recipient),
            12 * SCALAR_7
        );
        assert_eq!(fixture.client().get_claimable(&fixture.claimant), 0);
        assert_eq!(fixture.client().get_total_claimed(), 12 * SCALAR_7);
        assert_eq!(blnt.balance(&fixture.recipient), 12 * SCALAR_7);
        assert!(fixture
            .client()
            .try_claim(&fixture.claimant, &fixture.recipient)
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
        let fixture = Fixture::create(CLAIM_CAP);
        let blnt = TokenClient::new(&fixture.e, &fixture.blnt);

        assert_eq!(
            fixture
                .client()
                .claim(&fixture.claimant, &fixture.recipient),
            CLAIM_CAP
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
        let user = Address::generate(&e);
        let recipient = Address::generate(&e);
        let contract = e.register(
            BlntBackfillContract,
            (&legacy, &blnt, vec![&e, (claimant.clone(), SCALAR_7)]),
        );
        let client = BlntBackfillContractClient::new(&e, &contract);

        assert!(client.try_claim(&claimant, &recipient).is_err());
        assert!(client
            .try_swap_blnd_for_blnt(&user, &recipient, &SCALAR_7)
            .is_err());
        assert_eq!(client.get_claimable(&claimant), SCALAR_7);
        assert_eq!(client.get_total_swapped(), 0);
    }

    #[test]
    fn failed_payout_rolls_back_claim_and_conversion_accounting() {
        let fixture = Fixture::create(SCALAR_7);
        let blnt = TokenClient::new(&fixture.e, &fixture.blnt);
        let legacy = TokenClient::new(&fixture.e, &fixture.legacy);
        blnt.burn(&fixture.contract, &blnt.balance(&fixture.contract));

        assert!(fixture
            .client()
            .try_claim(&fixture.claimant, &fixture.recipient)
            .is_err());
        assert_eq!(fixture.client().get_claimable(&fixture.claimant), SCALAR_7);
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
        ));
        assert!(constructor_panics(
            &e,
            &legacy,
            &blnt,
            vec![&e, (claimant, CLAIM_CAP + 1)],
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
        assert!(constructor_panics(&e, &legacy, &blnt, claims));
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
        ));
    }

    #[test]
    fn publishes_claim_and_swap_events() {
        let fixture = Fixture::create(SCALAR_7);
        fixture
            .client()
            .claim(&fixture.claimant, &fixture.recipient);
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
