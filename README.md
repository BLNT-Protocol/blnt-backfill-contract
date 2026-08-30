# BLNT Backfill Contract

This Soroban contract custodies Blend v3's 50 million BLNT initial allocation
and exposes two immutable distribution lanes:

- address-based claims totaling at most 30 million BLNT; and
- perpetual 1:1 legacy BLND-to-BLNT conversion totaling at most 20 million
  BLNT.

The conversion transfers an authorized holder's legacy BLND into the contract,
verifies the exact receipt, burns it, and transfers the same raw seven-decimal
amount of pre-funded BLNT to the selected recipient. It never mints BLNT.

The contract has no administrator, upgrade, sweep, expiry, or BLNT-to-BLND
entry point. Any unallocated claim balance and unused conversion capacity stay
in the contract.

## Build and test

Rust 1.91.1, Soroban SDK 27.0.3, and the `wasm32v1-none` target are pinned.

```bash
make test
make build
```

See [SPECIFICATION.md](SPECIFICATION.md) for the normative behavior and public
API.
