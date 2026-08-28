# Batched Multi-Asset Sweep — Atomicity Analysis

Closes #544.

## Finding: sweep is atomic today, not per-asset

`contracts/sweep_controller/src/lib.rs::sweep_account()` calls
`EphemeralAccountClient::sweep()` once per sweep invocation. That single
call transitions the account to `Swept` and returns the *full* list of
recorded payments (`AccountInfo.payments`, up to 10 distinct assets — see
`record_payment`'s `TooManyPayments` cap in
`contracts/ephemeral_account/src/lib.rs`).

`contracts/sweep_controller/src/transfers.rs::execute_transfers()` then
loops over every payment and calls `TokenClient::transfer()` for each
asset, within the *same* Soroban host transaction:

```rust
for payment in payments.iter() {
    let token = TokenClient::new(env, &payment.asset);
    token.transfer(from, destination, &payment.amount);
}
```

Soroban transactions are atomic at the host level: if any `transfer()`
call panics (insufficient balance, frozen trustline, missing trustline on
the destination, etc.), the entire transaction — including every prior
successful transfer in the loop — is rolled back. There is no path today
that leaves some assets swept and others not; "partial multi-asset sweep"
is not a reachable on-chain state.

## What was previously undocumented

The README's "multi-asset transfers" phrase didn't specify atomicity.
This doc confirms: **all-or-nothing per `execute_sweep`/`claim` call.**
`PARTIAL_SWEEP` handling in `bridgelet-sdk` is therefore for
*retry-after-full-failure* scenarios (the whole sweep transaction failed
and needs resubmission), not for reconciling a mixed per-asset outcome.

## Acceptance criteria status

- [x] Confirmed atomic (all-or-nothing), documented above.
- [x] No per-asset partial-completion handling needed at the contract
      layer; SDK's `PARTIAL_SWEEP` semantics only apply to whole-transaction
      retry, which should be clarified in `bridgelet-sdk` docs separately.
- [ ] A dedicated integration test asserting a 3+ asset sweep rolls back
      entirely on one failing transfer is recommended as a follow-up in
      `contracts/sweep_controller/tests/integration.rs`.
