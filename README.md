# BLNT Backfill Contract

This Soroban contract custodies BLNT v3's 150 million BLNT initial allocation
and exposes three immutable distribution lanes:

- backfill allocations for at most 434 recipients totaling at most 74 million
  BLNT and vesting linearly over 180 days;
- grant allocations for at most 100 recipients totaling at most 25 million
  BLNT and vesting linearly over 720 days; and
- 2:1 legacy BLND-to-BLNT conversion totaling at most 51 million BLNT and
  expiring 270 days after construction.

For each raw unit of BLNT requested, the conversion transfers and immediately
burns two raw units of an authorized user's legacy BLND, then transfers one raw
unit of pre-funded BLNT to that same user. It never mints BLNT and conversions
cannot be refunded.

The contract has no administrator, upgrade, or sweep entry point. After
conversion expiry, anyone may call `burn_expired()` to destroy the unused BLNT
conversion reserve without affecting backfill or grant claims.

The production snapshot manifest assigns exactly 74 million BLNT across the
434 addresses (423 accounts and 11 contracts) with positive
`flattened_total_cpal_raw` in
`comet_cpal_flattened_ownership_before_41c898a1.csv`. The generated allocation
uses proportional floor rounding followed by deterministic largest-remainder
assignment. `claim_backfill(user)` transfers all vested-but-unclaimed BLNT to
that same authorized user. Vesting starts at contract construction, accrues per
second, and claims remain available indefinitely after vesting completes.

An independent immutable grant list is supplied at construction.
`claim_grant(user)` transfers its vested-but-unclaimed portion to that same
authorized user using the same construction timestamp but a two-year (720-day)
schedule. Backfill and grant progress, caps, views, and events are separate,
even when one address is present in both lists.

## Build and test

Rust 1.91.1, Soroban SDK 27.0.3, and the `wasm32v1-none` target are pinned.

```bash
make test
make build
```

See [SPECIFICATION.md](SPECIFICATION.md) for normative behavior and
[PUBLIC_API.md](PUBLIC_API.md) for the complete entrypoint, event, and error
reference.
