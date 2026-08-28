# EphemeralAccount Expiry Boundary Test Plan (#512)

## Logic under test
`contracts/ephemeral_account/src/lib.rs::is_expired` (lines 259-270):

```rust
let expiry_ledger = storage::get_expiry_ledger(&env);
let current_ledger = env.ledger().sequence();
current_ledger >= expiry_ledger
```

and the `initialize` guard (lines 68-71):

```rust
let current_ledger = env.ledger().sequence();
if expiry_ledger <= current_ledger {
    return Err(Error::InvalidExpiry);
}
```

## Boundary semantics confirmed by reading
- Expiry is **inclusive**: `current_ledger == expiry_ledger` already counts as expired
  (`>=`, not `>`).
- At creation time, `expiry_ledger` must be strictly greater than `current_ledger`
  (`<=` is rejected), so an account can never be created already-expired or
  expiring-this-ledger.

## Property-based test plan (proptest)
Add `proptest` as a dev-dependency of `contracts/ephemeral_account` and generate
`(current_ledger: u32, expiry_ledger: u32)` pairs across the full `u32` range, asserting:
1. `current_ledger < expiry_ledger` implies `is_expired == false`.
2. `current_ledger >= expiry_ledger` implies `is_expired == true`.
3. No panic/overflow for any generated pair, including `expiry_ledger = 0` and
   `expiry_ledger = u32::MAX`.

## Required named regression tests (not just incidental property coverage)
- `test_is_expired_false_at_expiry_minus_one` — `current_ledger == expiry_ledger - 1`.
- `test_is_expired_true_at_exact_expiry_ledger` — `current_ledger == expiry_ledger`
  (the off-by-one-prone case named explicitly per the acceptance criteria).
- `test_is_expired_true_one_past_expiry` — `current_ledger == expiry_ledger + 1`.
- `test_initialize_rejects_expiry_equal_to_current_ledger` — covers the `<=` boundary in
  `initialize`, mirroring the same edge on the write side.

## Status
No bug was found while reading the current comparison logic (`>=` at check time,
`<=` rejection at creation time are consistent with each other). The property suite above is
therefore expected to pass as-is; it exists to make that guarantee permanent and to catch any
future refactor that flips one of the two operators without the other.
