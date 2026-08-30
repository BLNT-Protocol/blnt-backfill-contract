# BLNT Backfill Contract Specification

Normative terms `MUST`, `MUST NOT`, `SHOULD`, and `MAY` are acceptance
criteria.

## Construction and funding

`__constructor(legacy_blnd_token, blnt_token, claim_list)` MUST:

- bind two distinct seven-decimal Stellar Asset Contracts;
- reject more than 100 claimants, duplicate addresses, the contract itself as
  a claimant, and non-positive claim amounts;
- reject a claim-list total above 30 million BLNT; and
- store the claim map and aggregate allocation immutably.

The Blend v3 emitter drop MUST transfer exactly 50 million BLNT to the deployed
contract. The contract receives no token-administrator or mint authority.

## Address claims

`claim(claimant, to)` MUST require `claimant` authorization, remove and return
that claimant's complete immutable allocation, and transfer exactly that raw
seven-decimal BLNT amount to `to`. It MUST reject an absent or already-consumed
claim and the contract itself as recipient. Failed transfers or balance
mismatches MUST roll back the claim removal and accounting.

The aggregate immutable claim list MUST NOT exceed 30 million BLNT. Claims do
not expire.

## BLND-to-BLNT conversion

`swap_blnd_for_blnt(from, to, amount)` MUST:

1. require `from` authorization and a positive amount;
2. reject any call whose cumulative successful amount would exceed 20 million
   tokens;
3. transfer exactly `amount` legacy BLND from `from` to the contract;
4. verify the exact receipt and burn it completely;
5. transfer exactly the same raw seven-decimal amount of pre-funded BLNT from
   the contract to `to`; and
6. increment the cumulative successful conversion total.

Conversion does not expire. Transfer, burn, balance, authorization,
reentrancy, cap, and overflow failures MUST roll back atomically. No
BLNT-to-BLND path exists.

## Authority and lifecycle

The contract MUST NOT expose an administrator, privileged upgrade, recovery,
sweep, mint, allocation mutation, deadline, or reverse-conversion entry point.
Unused conversion capacity and unallocated BLNT remain locked unless a future
contract design explicitly replaces this deployment policy.

Contract and persistent claim-map entries rely on normal Soroban state
archival and restoration. Successful calls and views renew relevant TTLs.

## Public API

- `__constructor(legacy_blnd_token, blnt_token, claim_list)`
- `claim(claimant, to) -> i128`
- `swap_blnd_for_blnt(from, to, amount) -> i128`
- `get_claimable(claimant) -> i128`
- `get_legacy_blnd_token() -> Address`
- `get_blnt_token() -> Address`
- `get_total_allocated() -> i128`
- `get_total_claimed() -> i128`
- `get_total_swapped() -> i128`
- `get_remaining_swap_capacity() -> i128`
