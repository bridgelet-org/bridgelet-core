# Concurrent Sweep Test Harness Plan

Closes #515.

## What "concurrent" means in a Soroban test

Soroban contract tests execute against `soroban_sdk::testutils::Env`, which
has no real threads or ledger concurrency. "Concurrent/interleaved" here
means: submit two `execute_sweep` (or one `execute_sweep` + one `claim`)
invocations for the *same* `ephemeral_account`, back to back, before either
has had a chance to observe the other's effect — i.e. simulate two relayers
racing to submit the same authorized sweep within the same or adjacent
ledger.

## Relevant mechanism

`contracts/sweep_controller/src/lib.rs::sweep_account` and
`authorization::increment_nonce` (`contracts/sweep_controller/src/
authorization.rs`): the sweep nonce is incremented *after* signature
verification but *before* the cross-contract call to
`EphemeralAccountClient::sweep`, specifically to make a re-entrant/replayed
call within the same transaction fail nonce verification.

## Test harness design

`contracts/sweep_controller/tests/integration.rs` (existing file) gets two
new scenarios (not added here — doc only):

1. **Same-signature replay race**: sign one sweep message under nonce N,
   submit it twice against the same ephemeral account. Assert the first
   call succeeds and the second fails signature verification (nonce has
   advanced to N+1, so the signature no longer matches).
2. **Two distinct destinations race**: two off-chain signers race to submit
   sweeps for the same account to *different* destinations. Assert exactly
   one call transitions the account out of `PaymentReceived` state and the
   loser observes `AccountNotReady` (via `can_sweep` returning `false`) or
   a signature/nonce mismatch.

## What to document explicitly

Per the acceptance criteria, the test's findings must state plainly:
double-sweep protection here is enforced **at the contract layer**, via
the monotonic `sweep_nonce` bound into the signed message
(`construct_sweep_message` in `authorization.rs`), not merely at the
application/relayer layer. Cross-reference with the equivalent
bridgelet-sdk load-test issue to confirm the SDK-level test doesn't
silently rely on relayer-side locking to pass.
