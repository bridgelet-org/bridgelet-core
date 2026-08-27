# Partial Payment Detection (#542)

## Location
`contracts/ephemeral_account/src/lib.rs` (`record_payment`, `get_info`,
`simulate_sweep`).

## Problem
`record_payment(env, amount, asset)` records whatever amount the
`authorized_controller` reports for a given asset, once per asset (duplicate-asset
check via `storage::get_payment`). There is no concept of an "expected" amount per
asset, so a sender who transfers less than the integrator quoted (partial payment,
interrupted retry, split payment) is recorded identically to a full payment: status
flips to `AccountStatus::PaymentReceived` on the very first `record_payment` call
(`payment_count == 0` check), and `can_sweep`/`SweepController::can_sweep` reports the
account ready.

## Decision
1. **No contract-level "expected amount" is introduced in this release.** The
   ephemeral account contract has no way to know the sender's intended amount ahead
   of time — only `bridgelet-sdk` at account-creation time knows the quoted amount,
   and that value is not currently passed into `initialize` or `record_payment`.
2. **Distinguishing "ready to claim" vs "awaiting more" is therefore an off-chain
   responsibility**, using data the contract already exposes:
   - `get_info()` returns `AccountInfo.payments` (per-asset amount + timestamp) and
     `payment_received`.
   - `SweepController::fee_estimate` and `simulate_sweep` both surface the exact
     recorded `amount` per asset before a sweep is attempted.
   - `bridgelet-sdk` MUST compare the summed `payments` amount against its own
     locally quoted expected amount and withhold triggering `execute_sweep`/`claim`
     until the recorded amount meets or exceeds the quote.
3. **Top-up handling**: because `record_payment` rejects a second call for the same
   `asset` (`Error::DuplicateAsset`, code 1012), a genuine "partial now, rest later"
   top-up for the *same* asset is currently **not supported** at the contract level —
   this is called out explicitly as a known limitation, not a bug: attempting a
   same-asset top-up after a first partial `record_payment` fails fast with
   `Error::DuplicateAsset` rather than silently combining amounts.
4. Test coverage to add in `contracts/ephemeral_account/src/test.rs`: a
   `record_payment` call with a below-quote amount followed by an SDK-level
   assertion that `execute_sweep` is not triggered, and a same-asset second
   `record_payment` call asserting the existing `Error::DuplicateAsset` (not a
   silent top-up).
