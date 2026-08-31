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
| `__constructor(legacy_blnd_token: Address, blnt_token: Address, claim_list: Vec<(Address, i128)>, grant_list: Vec<(Address, i128)>)` | None | Immutably binds distinct seven-decimal legacy BLND and BLNT Stellar Asset Contracts, the backfill allocation list, both vesting schedules, the independent grant list, and the 270-day conversion deadline. The backfill list accepts at most 434 unique positive allocations totaling at most 20 million BLNT. The grant list accepts at most 100 unique positive allocations totaling at most 10 million BLNT. Combined claims cannot exceed 30 million BLNT. |

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
| `claim_backfill(user: Address) -> i128` | `user` | BLNT transferred by this call | Transfers all currently vested backfill allocation not previously claimed to the same authorized address. Vesting is linear for 180 days from construction. |
| `claim_grant(user: Address) -> i128` | `user` | BLNT transferred by this call | Transfers all currently vested grant allocation not previously claimed to the same authorized address. Vesting is linear for 720 days from construction. Grant accounting is independent from backfill accounting. |
| `swap_blnd_for_blnt(user: Address, amount: i128) -> i128` | `user` | Cumulative gross BLND-to-BLNT swap total | Before the immutable 270-day deadline, escrows exactly `amount` legacy BLND and transfers exactly the same raw amount of pre-funded BLNT back to `user`. Net outstanding conversions cannot exceed 120 million BLNT. |
| `refund_blnt_for_blnd(user: Address, amount: i128) -> i128` | `user` | Cumulative BLNT-to-BLND refund total | Before the conversion deadline, returns escrowed BLND to `user` in exchange for the same raw amount of BLNT, up to that user's unrefunded swaps. |
| `burn_expired() -> ExpiredBurn` | None | `{ blnd, blnt }` burned by this call | At or after the conversion deadline, burns BLND escrow for every unrefunded swap and the unused BLNT conversion reserve while preserving all outstanding backfill and grant claims. Returns both fields as zero once finalized. |

`ExpiredBurn` contains independent raw amounts because adding BLND and BLNT
would not produce a meaningful single-token return value.

Backfill claims, grant claims, swaps, and refunds use one address as the
authorized owner and token recipient; none permits redirecting proceeds to a
different address. No token-moving entrypoint may name the backfill contract as
its user. State and token movements roll back atomically on failure. The
contract has no administrator, upgrade, sweep, or allocation-mutation
entrypoint. Claim and grant availability does not expire.

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
| `get_total_swapped() -> i128` | Cumulative gross BLND escrowed and BLNT transferred through successful swaps; refunds do not reduce it. |
| `get_total_refunded() -> i128` | Cumulative BLNT returned and legacy BLND refunded through successful refunds. |
| `get_net_swapped() -> i128` | Gross swaps less refunds; equal to outstanding escrow and aggregate refund credit before expiry. |
| `get_refundable(user: Address) -> i128` | The user's remaining refundable amount before expiry; zero at or after expiry. |
| `get_remaining_swap_capacity() -> i128` | 120 million BLNT less net outstanding conversions before expiry; zero at or after expiry. Refunds restore capacity. |
| `get_swap_deadline() -> u64` | Immutable conversion deadline, exactly 270 days after construction. |
| `get_expired_blnd_burned() -> i128` | Escrowed legacy BLND destroyed after conversion expiry. |
| `get_expired_blnt_burned() -> i128` | Unused BLNT conversion reserve destroyed after conversion expiry. |

## Events

| Event | Topics | Data | Emitted by |
| --- | --- | --- | --- |
| `claim` | `user: Address` | `amount: i128` | `claim_backfill` |
| `claim_grant` | `user: Address` | `amount: i128` | `claim_grant` |
| `swap_blnd` | `user: Address` | `amount: i128`, `total_swapped: i128`, `refundable: i128`, `net_swapped: i128` | `swap_blnd_for_blnt` |
| `refund_blnd` | `user: Address` | `amount: i128`, `total_refunded: i128`, `refundable: i128`, `net_swapped: i128` | `refund_blnt_for_blnd` |
| `burn_expired` | None | `blnd: i128`, `blnt: i128`, `net_swapped: i128` | `burn_expired` |

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
| 1206 | `InvalidRecipient` | A token-moving call names the backfill contract as its user or recipient. |
| 1207 | `InvalidSwapAmount` | The requested swap amount is non-positive. |
| 1208 | `SwapCapExceeded` | The swap would make net outstanding conversions exceed 120 million BLNT. |
| 1209 | `BalanceMismatch` | An observed token debit, credit, burn, or payout differs from the exact requested amount. |
| 1210 | `ReentrantCall` | A token-moving entrypoint is already executing. |
| 1211 | `Overflow` | Checked arithmetic or a vesting invariant failed. |
| 1212 | `NothingClaimable` | A backfill allocation exists but currently has no unclaimed vested amount. |
| 1213 | `TooManyGrantees` | The grant list contains more than 100 entries. |
| 1214 | `DuplicateGrantee` | A grant address occurs more than once. |
| 1215 | `GrantCapExceeded` | The grant allocation exceeds 10 million BLNT. |
| 1216 | `NoGrant` | No active grant allocation exists for the grantee. |
| 1217 | `NothingGrantClaimable` | A grant exists but currently has no unclaimed vested amount. |
| 1218 | `SwapExpired` | The swap/refund conversion deadline has been reached. |
| 1219 | `SwapNotExpired` | Expired-reserve burning was requested before the conversion deadline. |
| 1220 | `InvalidRefundAmount` | The requested refund amount is non-positive. |
| 1221 | `RefundExceedsCredit` | The requested refund exceeds the user's unrefunded swaps. |
