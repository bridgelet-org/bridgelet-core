# Integration Test Plan: EphemeralAccount + SweepController + ReserveContract

Tracking: #520

## Scope

New crate `contracts/integration_tests/`, workspace member alongside the
existing 16 contract crates, with a `[dev-dependencies]` path reference to
`ephemeral_account`, `sweep_controller`, and `reserve_contract`. Builds on
the existing pattern in `contracts/sweep_controller/tests/integration.rs`
(deploy via `env.register_contract`, drive both clients from one `Env`),
extended to three contracts and no dependency on bridgelet-sdk.

## Scenario 1: create -> fund -> claim-triggered sweep

1. `reserve_contract::initialize(admin)`, `set_base_reserve(amount)`.
2. `ephemeral_account::initialize(creator, expiry_ledger, recovery)`.
3. `ephemeral_account::record_payment(amount, asset)` to fund it.
4. `sweep_controller::can_sweep(ephemeral_id)` — assert `true` once funded.
5. `sweep_controller::claim(recipient, ephemeral_id)`.
6. Assert `ephemeral_account::get_status() == AccountStatus::Swept` and the
   recipient's simulated balance reflects the swept amount.

## Scenario 2: create -> fund -> expire -> recovery sweep

1. Same setup as Scenario 1, but advance past expiry:
   `env.ledger().set_sequence_number(expiry_ledger + 1)` before claiming.
2. Assert `ephemeral_account::is_expired() == true`.
3. Call `ephemeral_account::expire()`, then `recover(caller)` from the
   `recovery` address set at `initialize`.
4. Assert reserve is released via `reclaim_reserve()` and
   `get_reserve_remaining()` returns 0, cross-checked against
   `reserve_contract::get_base_reserve()` for the amount reserved upfront.

## Execution

- Run with `cargo test` from `contracts/integration_tests/`; no sandbox
  network calls required since `soroban_sdk::testutils::Address` and
  `Env::default()` already simulate ledger state in-process (same approach
  `sweep_controller`'s existing integration test uses) — "local Soroban
  sandbox" here means the SDK's in-process test environment, not a running
  `soroban container`.
- Add `cd contracts/integration_tests && cargo test --verbose` as a new
  step in `.github/workflows/test.yml`, in the same per-crate loop style as
  the other 16 entries, so failures block PRs the same way.
