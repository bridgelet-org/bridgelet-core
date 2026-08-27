# Issue #508: Integer Overflow/Underflow and Panic-Safety Audit

Scope: arithmetic on balances, reserves, and ledger sequences across
`contracts/*/src/lib.rs`.

## Explicit checked arithmetic found

- `contracts/ephemeral_account/src/lib.rs:550` — `.checked_add(payment.amount)`
  when accumulating a new payment amount, propagating an error rather than
  wrapping/panicking on overflow.
- `contracts/ephemeral_account/src/lib.rs:637,640` — `.checked_sub(reclaim_amount)`
  used twice when decrementing reserve balances during `reclaim_reserve`.
- `contracts/ephemeral_account/src/lib.rs:670` — `event_count.checked_add(1)
  .ok_or(Error::InvalidAmount)` guarding the reserve-event counter.
- `contracts/sweep_controller/src/lib.rs:422-424` — `current_ledger
  .checked_add(SIGNER_TIMELOCK_LEDGERS).ok_or(Error::Overflow)` when computing
  the signer-rotation time-lock's effective ledger, explicitly mapped to a
  dedicated `Error::Overflow` variant.

## Implicit/unchecked arithmetic found

- `contracts/sweep_controller/src/storage.rs:91` — `increment_sweep_nonce`
  computes `current_nonce + 1` on a `u64` without `checked_add`, relying on
  Rust's implicit panic-on-overflow rather than an explicit check. At one
  sweep per call, reaching `u64::MAX` is theoretical, but it is inconsistent
  with the `checked_add` pattern used elsewhere in the same file
  (`update_authorized_signer`'s ledger arithmetic).
- `contracts/sweep_controller/src/lib.rs:167,235,395` —
  `info.payments.iter().map(|p| p.amount).sum()` sums `i128` payment amounts
  with no explicit checked/saturating reducer, relying on Rust's default
  overflow behavior instead.

## Release-build panic behavior

Trusting implicit-panic arithmetic in production requires
`overflow-checks = true` at the workspace `[profile.release]` level; this
audit did not modify any `contracts/*/Cargo.toml`, so confirming that
setting is tracked as a follow-up rather than fixed here.

## Findings / recommendations

1. No unchecked arithmetic on ephemeral-account payment/reserve balances —
   these already use `checked_add`/`checked_sub` with typed errors.
2. `increment_sweep_nonce` and the `.sum()` calls over payment vectors rely
   on implicit panic-on-overflow. Soroban aborts (does not silently corrupt
   state) on such a panic, so this fails safe rather than producing a wrong
   sweep amount — but should be made explicit per the acceptance criteria.
3. Follow-up: add `checked_add`/explicit saturating logic to
   `increment_sweep_nonce` and the payment-sum reducers, and confirm
   `overflow-checks = true` at the workspace `Cargo.toml` level.
