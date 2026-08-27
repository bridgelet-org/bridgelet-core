# Trustline Lifecycle for Ephemeral Accounts — Non-Native Assets

Closes #545.

## Current state

`contracts/ephemeral_account/src/lib.rs::record_payment()` accepts a
payment for any `asset: Address` (up to 10 distinct assets, the
`TooManyPayments` guard). Nothing in `initialize()`, `record_payment()`,
or `execute_sweep_core()` creates, checks, or removes a Stellar classic
trustline — the contract only tracks SEP-41 balances via `TokenClient`
(`contracts/sweep_controller/src/transfers.rs`). This is expected: a
trustline is a classic-ledger construct, established on the underlying
Stellar account, and consumes one `baseReserve` increment (mirrored here
via `contracts/reserve_contract`'s configurable reserve, default
`BASE_RESERVE_STROOPS = 1_000_000_000`, `ephemeral_account/src/lib.rs:26`).

## Gap identified

Trustline creation/removal is **not handled by any contract in this
repo** — it is an implicit external/off-chain step. The account-creation
flow must submit `ChangeTrust` before a non-native payment can land, and
again (limit 0, to remove) after `reclaim_reserve_to()` fully zeroes the
reserve (`storage::is_reserve_reclaimed`).

## Recommendation

1. Treat trustline setup as an explicit external step, performed *before*
   `EphemeralAccountContract::initialize()`, for every non-native asset
   the account expects to receive.
2. Trigger trustline removal off-chain immediately after observing
   `ReserveReclaimed{ fully_reclaimed: true }` (emitted from
   `reclaim_reserve_to`).
3. `ReserveContract::get_base_reserve()` currently returns one flat
   figure and does not scale with expected trustline count. If accounts
   hold N non-native assets, `resolve_base_reserve()` in
   `ephemeral_account/src/lib.rs` should account for N reserve
   increments, not just the single-asset default — tracked as a
   follow-up.

## Acceptance criteria status

- [x] Trustline creation confirmed as an explicit external step.
- [x] Trustline removal timing documented (post `fully_reclaimed` event).
- [ ] `ReserveContract` scaling by trustline count — follow-up, out of
      scope here.
