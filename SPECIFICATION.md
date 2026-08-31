# BLNT Backfill Contract Specification

Normative terms `MUST`, `MUST NOT`, `SHOULD`, and `MAY` are acceptance
criteria.

## Construction and funding

`__constructor(legacy_blnd_token, blnt_token, claim_list, grant_list)` MUST:

- bind two distinct seven-decimal Stellar Asset Contracts;
- reject more than 434 backfill claimants or 100 grant recipients;
- reject duplicate addresses within either list, the contract itself as a
  recipient, and non-positive amounts;
- reject a backfill total above 20 million BLNT, a grant total above 10 million
  BLNT, or a combined total above 30 million BLNT;
- store both allocation maps and their separate and combined totals
  immutably;
- bind a vesting start to the construction ledger timestamp and a vesting end
  exactly 180 days later for backfill and 720 days later for grants; and
- bind the BLND-to-BLNT conversion deadline to exactly 270 days after the
  construction ledger timestamp.

The same address MAY occur once in each list because the two allocations,
progress records, caps, and entry points are independent.

The BLNT v3 emitter drop MUST transfer exactly 150 million BLNT to the deployed
contract. The contract receives no token-administrator or mint authority.

## Address claims

For an immutable allocation `A`, construction timestamp `S`, 180-day duration
`D`, and current timestamp `T`, cumulative vested BLNT MUST be:

- zero when `T <= S`;
- `floor(A * (T - S) / D)` when `S < T < S + D`; and
- exactly `A` when `T >= S + D`.

`claim_backfill(claimant, to)` MUST require `claimant` authorization and
transfer exactly the cumulative vested amount less that claimant's prior
successful claims. A claimant MAY call repeatedly as additional BLNT vests.
The call MUST reject an absent or fully consumed allocation, a zero currently
claimable amount, and the contract itself as recipient. Failed transfers or
balance mismatches MUST roll back both claimant and aggregate progress.

The production snapshot allocation MUST assign exactly 20 million BLNT in
proportion to each of the 423 account and 11 contract addresses with a positive
`flattened_total_cpal_raw` value from
`comet_cpal_flattened_ownership_before_41c898a1.csv`. It MUST floor each
pro-rata result, then distribute the remaining base units to the greatest
fractional remainders, breaking equal remainders by ascending address. Rows
with zero weight MUST receive no claim. The generated manifest MUST record the
source hash, weight total, claimant count, and exact aggregate allocation.

The aggregate immutable backfill and grant lists MUST NOT exceed 30 million
BLNT. Vesting starts at construction and claims do not expire after their
respective vesting ends.

## Grant claims

For an immutable grant allocation `G`, the cumulative vested calculation MUST
use the address-claim formula above with a 720-day duration. Grant vesting MUST
begin at the same construction timestamp as backfill vesting.

`claim_grant(grantee, to)` MUST require `grantee` authorization and transfer
exactly the cumulative grant amount vested less that grantee's prior successful
grant claims. It MUST use independent grant progress and MUST NOT read, consume,
or accelerate a backfill allocation held by the same address. The zero-amount,
recipient, rollback, rounding, final-allocation, and no-expiry requirements for
backfill claims apply equally to grant claims.

The immutable grant list MUST NOT exceed 100 recipients or 10 million BLNT.

## BLND-to-BLNT conversion

`swap_blnd_for_blnt(from, to, amount)` MUST:

1. require `from` authorization and a positive amount;
2. reject at or after the immutable conversion deadline;
3. reject any call whose cumulative successful amount would exceed 120 million
   tokens;
4. transfer exactly `amount` legacy BLND from `from` to the contract;
5. verify the exact receipt and burn it completely;
6. transfer exactly the same raw seven-decimal amount of pre-funded BLNT from
   the contract to `to`; and
7. increment the cumulative successful conversion total.

Conversion remains open before the deadline and expires exactly 270 days after
construction. Transfer, burn, balance, authorization,
reentrancy, cap, and overflow failures MUST roll back atomically. No
BLNT-to-BLND path exists.

After the conversion deadline, permissionless `burn_expired()` MUST burn
exactly the unused conversion reserve, defined as 120 million BLNT less the
cumulative successful conversion total and any amount already burned by that
entry point. It MUST reject before the deadline, MUST preserve enough BLNT to
cover every unclaimed backfill and grant allocation, and MUST verify the exact
token debit. The first successful call MUST record and emit the burned amount.
Calls after the reserve has been fully converted or burned MUST return zero so
permissionless callers may race safely.

## Authority and lifecycle

The contract MUST NOT expose an administrator, privileged upgrade, recovery,
sweep, mint, allocation mutation, or reverse-conversion entry point. Unused
conversion capacity is destroyed only through `burn_expired`; backfill and
grant claims do not expire and their outstanding reserves MUST remain intact.

Contract and persistent allocation/progress-map entries rely on normal Soroban
state archival and restoration. Successful calls and views renew relevant
TTLs.

## Public API

See [PUBLIC_API.md](PUBLIC_API.md) for authorization, return-value, event, and
error details.

- `__constructor(legacy_blnd_token, blnt_token, claim_list, grant_list)`
- `claim_backfill(claimant, to) -> i128`
- `claim_grant(grantee, to) -> i128`
- `swap_blnd_for_blnt(from, to, amount) -> i128`
- `burn_expired() -> i128`
- `get_backfill_claimable(claimant) -> i128`
- `get_grant_claimable(grantee) -> i128`
- `get_legacy_blnd_token() -> Address`
- `get_blnt_token() -> Address`
- `get_total_allocated() -> i128`
- `get_backfill_allocated() -> i128`
- `get_grant_allocated() -> i128`
- `get_total_claimed() -> i128`
- `get_backfill_claimed() -> i128`
- `get_grant_claimed() -> i128`
- `get_vesting_start() -> u64`
- `get_vesting_end() -> u64`
- `get_grant_vesting_start() -> u64`
- `get_grant_vesting_end() -> u64`
- `get_total_swapped() -> i128`
- `get_remaining_swap_capacity() -> i128`
- `get_swap_deadline() -> u64`
- `get_expired_swap_burned() -> i128`
