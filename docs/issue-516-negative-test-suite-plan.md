# Negative Test Suite Plan — Malformed/Adversarial Arguments

Closes #516.

## Scope

Systematic negative-argument coverage across `contracts/*/tests/`,
complementing existing happy-path unit tests (e.g.
`contracts/reserve_contract/src/test.rs`,
`contracts/sweep_controller/src/test.rs`). Distinct from invalid
*business-logic* values (already covered by e.g. `set_base_reserve`'s
`InvalidAmount`/`AmountTooLarge` checks) — this targets invalid *argument
shape*.

## Representative cases by contract

- **reserve_contract**: `set_base_reserve` with `i128::MIN`; calling any
  state-changing method before `initialize` (already returns
  `NotInitialized` — confirm no panic path exists instead).
- **sweep_controller**: `execute_sweep`/`claim` with a `BytesN<64>`
  signature that is well-formed length but all-zero bytes; an
  `ephemeral_account` Address that has never been deployed/initialized
  (currently: `EphemeralAccountClient::new` + cross-contract call — confirm
  this fails as a typed error and not a raw host trap); `auth_signature`
  reused after nonce increment (covered by #515 but also a negative case
  here).
- **General across contracts**: empty `Vec`/`Bytes` where a non-empty
  collection is implicitly expected; an `Address` argument equal to the
  contract's own address (self-call edge case).

## Error-handling convention check

Per current code: `reserve_contract` and `sweep_controller` both use
`#[contracterror]` enums returned as `Result<_, Error>` for validation
failures, but `authorization::verify_sweep_auth` documents that a bad
Ed25519 signature **panics** via `env.crypto().ed25519_verify()` rather
than returning `Err(InvalidSignature)` (see `authorization.rs` doc comment
above `verify_sweep_auth`, referencing issue #411). This is the one
confirmed inconsistency: signature-format errors return typed `Result`s,
but signature-*validity* failures abort the transaction. The negative test
suite should assert this behavior explicitly (i.e. pin it down as
intentional, tested behavior) rather than leave it undocumented, and flag
it in the acceptance-criteria review as the finding to reconcile or
consciously accept as "abort on crypto failure by design."

## Deliverable

One test module per contract asserting each public function rejects at
least one malformed-shape input, plus a table enumerating panic-vs-typed-
error behavior per function so the convention is auditable at a glance.
