# BLNT Backfill Contract Public API

This non-normative reference describes the public ABI exported by the
optimized BLNT backfill contract. [SPECIFICATION.md](SPECIFICATION.md) remains
authoritative for contract behavior and invariants.

Signatures omit the implicit Soroban `Env` argument. All token amounts are
signed `i128` values in raw seven-decimal base units. Timestamps are `u64`
Unix seconds. Examples use readable JSON-like notation rather than Stellar
CLI's encoded JSON representation.

## Constructor

| Entry point | Authorization | Behavior |
| --- | --- | --- |
| `__constructor(legacy_blnd_token: Address, blnt_token: Address, claim_list: Vec<(Address, i128)>, grant_list: Vec<(Address, i128)>)` | None | Immutably binds distinct seven-decimal legacy BLND and BLNT Stellar Asset Contracts, the backfill allocation list, both vesting schedules, and the independent grant list. The backfill list accepts at most 434 unique positive allocations totaling at most 20 million BLNT. The grant list accepts at most 100 unique positive allocations totaling at most 10 million BLNT. Combined claims cannot exceed 30 million BLNT. |

An address may occur once in each list. Duplicate addresses within one list
are rejected. Neither list can be changed after construction.

Example list shape:

```text
[
  ["G_RECIPIENT_1", "125000000"],
  ["C_RECIPIENT_2", "75000000"]
]
```

## State-changing entrypoints

| Entry point | Required authorization | Return value | Behavior |
| --- | --- | --- | --- |
| `claim_backfill(claimant: Address, to: Address) -> i128` | `claimant` | BLNT transferred by this call | Transfers all currently vested backfill allocation not previously claimed. Vesting is linear for 180 days from construction. |
| `claim_grant(grantee: Address, to: Address) -> i128` | `grantee` | BLNT transferred by this call | Transfers all currently vested grant allocation not previously claimed. Vesting is linear for 720 days from construction. Grant accounting is independent from backfill accounting. |
| `swap_blnd_for_blnt(from: Address, to: Address, amount: i128) -> i128` | `from` | Cumulative successful BLND-to-BLNT swap total | Transfers and burns exactly `amount` legacy BLND, then transfers exactly the same raw amount of pre-funded BLNT to `to`. The aggregate conversion cap is 20 million BLNT and does not expire. |

`to` may differ from the authorized source but cannot be the backfill contract
itself. State and token movements roll back atomically on failure. The contract
has no administrator, upgrade, sweep, reverse-swap, or allocation-mutation
entrypoint.

## Views

Views require no contract-level authorization. Clients may obtain their return
values through RPC simulation without submitting a transaction. The
implementation requests TTL renewal while reading contract state; those TTL
changes persist only if the invocation is signed, paid for, and submitted.

### Claim state

| Entry point | Return value |
| --- | --- |
| `get_backfill_claimable(claimant: Address) -> i128` | Backfill BLNT currently vested and not yet claimed; zero for an absent, fully claimed, or not-yet-vested allocation. |
| `get_grant_claimable(grantee: Address) -> i128` | Grant BLNT currently vested and not yet claimed; zero for an absent, fully claimed, or not-yet-vested allocation. |
| `get_total_allocated() -> i128` | Immutable combined backfill and grant allocation. |
| `get_backfill_allocated() -> i128` | Immutable aggregate backfill allocation. |
| `get_grant_allocated() -> i128` | Immutable aggregate grant allocation. |
| `get_total_claimed() -> i128` | Cumulative successful backfill and grant claims. |
| `get_backfill_claimed() -> i128` | Cumulative successful backfill claims. |
| `get_grant_claimed() -> i128` | Cumulative successful grant claims. |

### Vesting state

| Entry point | Return value |
| --- | --- |
| `get_vesting_start() -> u64` | Shared construction timestamp at which backfill and grant vesting begin. |
| `get_vesting_end() -> u64` | Backfill vesting end, exactly 180 days after construction. |
| `get_grant_vesting_start() -> u64` | Grant vesting start; equal to `get_vesting_start()`. |
| `get_grant_vesting_end() -> u64` | Grant vesting end, exactly 720 days after construction. |

### Token and conversion state

| Entry point | Return value |
| --- | --- |
| `get_legacy_blnd_token() -> Address` | Immutable legacy BLND Stellar Asset Contract address. |
| `get_blnt_token() -> Address` | Immutable BLNT Stellar Asset Contract address. |
| `get_total_swapped() -> i128` | Cumulative legacy BLND burned and BLNT transferred through successful swaps. |
| `get_remaining_swap_capacity() -> i128` | Remaining amount under the 20-million-BLNT aggregate conversion cap. |

## Events

| Event | Topics | Data | Emitted by |
| --- | --- | --- | --- |
| `claim` | `claimant: Address`, `to: Address` | `amount: i128` | `claim_backfill` |
| `claim_grant` | `grantee: Address`, `to: Address` | `amount: i128` | `claim_grant` |
| `swap_blnd` | `from: Address`, `to: Address` | `amount: i128`, `total_swapped: i128` | `swap_blnd_for_blnt` |

## Contract errors

Soroban authorization and token-contract failures may also propagate from host
or nested calls. The backfill contract defines these errors:

| Code | Name | Meaning |
| ---: | --- | --- |
| 1200 | `InvalidToken` | Token bindings are equal, are not Stellar Asset Contracts, or do not use seven decimals. |
| 1201 | `TooManyClaimants` | The backfill list contains more than 434 entries. |
| 1202 | `InvalidClaimAmount` | A claim or grant amount is non-positive, or its recipient is the backfill contract. |
| 1203 | `DuplicateClaimant` | A backfill address occurs more than once. |
| 1204 | `ClaimCapExceeded` | The backfill or combined claim allocation exceeds its cap. |
| 1205 | `NoClaim` | No active backfill allocation exists for the claimant. |
| 1206 | `InvalidRecipient` | A claim or swap names the backfill contract as `to`. |
| 1207 | `InvalidSwapAmount` | The requested swap amount is non-positive. |
| 1208 | `SwapCapExceeded` | The swap would exceed the 20-million-BLNT aggregate cap. |
| 1209 | `BalanceMismatch` | An observed token debit, credit, burn, or payout differs from the exact requested amount. |
| 1210 | `ReentrantCall` | A token-moving entrypoint is already executing. |
| 1211 | `Overflow` | Checked arithmetic or a vesting invariant failed. |
| 1212 | `NothingClaimable` | A backfill allocation exists but currently has no unclaimed vested amount. |
| 1213 | `TooManyGrantees` | The grant list contains more than 100 entries. |
| 1214 | `DuplicateGrantee` | A grant address occurs more than once. |
| 1215 | `GrantCapExceeded` | The grant allocation exceeds 10 million BLNT. |
| 1216 | `NoGrant` | No active grant allocation exists for the grantee. |
| 1217 | `NothingGrantClaimable` | A grant exists but currently has no unclaimed vested amount. |
