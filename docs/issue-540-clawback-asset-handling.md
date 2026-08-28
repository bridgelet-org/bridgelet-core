# Clawback-Enabled Asset Handling During Sweep (#540)

## Location
`contracts/sweep_controller/src/lib.rs` (`sweep_account`, `execute_sweep`, `claim`) and
`contracts/sweep_controller/src/transfers.rs` (`execute_transfers`).

## Problem
A Stellar asset issuer can enable clawback on a trustline, allowing the issuer to
reclaim funds already sent to an ephemeral account at any time, including after
`record_payment` recorded the amount on the ephemeral account contract but before
`execute_sweep`/`claim` moves it via `TokenClient::transfer`. Today, `execute_transfers`
calls `token.transfer(from, destination, amount)` unconditionally and relies on the
SEP-41 token contract to panic if the balance is insufficient. A prior clawback
therefore surfaces as an opaque host panic rather than a typed `Error`.

## Decision
1. A clawback that occurs **before** `execute_sweep`/`claim` is attempted causes
   `token.transfer` to fail (the ephemeral account's on-chain balance is now short of
   the recorded `Payment.amount`). Per the atomicity guarantee already documented in
   `transfers.rs`, the whole sweep transaction reverts — no partial transfer, no state
   change on `sweep_controller` or `ephemeral_account`.
2. This case is distinguished from ordinary transfer failures by introducing
   `Error::ClawbackDetected` (new sweep_controller error, namespace 2000-2999, next
   free code `2019`). `execute_transfers` pre-checks each payment's asset balance via
   `TokenClient::balance(from)` against the recorded `Payment.amount` before invoking
   `transfer`; a shortfall returns `Error::ClawbackDetected` instead of letting the
   opaque token-level panic propagate.
3. This is recorded as a known risk category in `docs/security.md` alongside existing
   MVP-scope caveats: bridgelet does not block clawback-enabled assets at
   `record_payment` time (the ephemeral account cannot introspect trustline flags), so
   integrators funding with clawback-enabled assets accept the risk that a clawback
   after funding, but before sweep, causes `Error::ClawbackDetected` and requires the
   integrator to treat the ephemeral account as a lost-funds case rather than retry.

## Follow-up
`bridgelet-sdk` should surface `Error::ClawbackDetected` (2019) distinctly from
`Error::TransferFailed` (2001) in its error mapping so integrators can alert on it
separately from generic transfer failures.
