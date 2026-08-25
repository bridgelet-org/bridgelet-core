<!--
Purpose: Postmortem documenting how AccountFactory::batch_initialize swallows initialization errors.
Owner: Kandexa (closes #328).
Status: Documentation-only. No contracts, docs, scripts, or tools changes are introduced by this file.
-->

# Postmortem: Error Swallowing in AccountFactory::batch_initialize

| Field | Value |
| :--- | :--- |
| **Related issue** | [#328](https://github.com/bridgelet-org/bridgelet-core/issues/328) |
| **Owner / reviewer** | `_operator-name_` |
| **Last reviewed** | `_ISO-8601 date_` |

## What the code does

`AccountFactory::batch_initialize()` in `contracts/account_factory/src/lib.rs:78–162`
accepts a vector of `AccountInitRequest` and deploys + initializes an `EphemeralAccount`
for each request. The function returns a `Vec<AccountInitResult>` where each result has
three fields: `account_address`, `success`, and `error: Option<Bytes>`.

The `match` on `client.try_initialize(...)` (lines 128–157) handles two branches:

```rust
Ok(_) => AccountInitResult {
    account_address: account_address.clone(),
    success: true,
    error: None,
},
Err(_) => AccountInitResult {
    account_address: account_address.clone(),
    success: false,
    error: None, // ← the problem
},
```

When `try_initialize` succeeds, the result is correct. When it fails, the `Err` variant
is caught, `success` is set to `false`, but `error` is set to `None` — the actual error
is discarded.

## The consequence

A caller who inspects `AccountInitResult` after a batch sees `success: false` but has no
error information. The possible failure modes of `EphemeralAccount::initialize()` include:

| Error | Code | Meaning |
| :--- | :--- | :--- |
| `AlreadyInitialized` | 1000 | Account was already initialized (double-init) |
| `InvalidExpiry` | 1004 | Expiry ledger is in the past or too far in the future |
| `NotInitialized` | 1001 | Factory was not initialized (from shared error) |
| `InvalidAmount` | 1012 | Reserve amount is not positive |
| `Overflow` | 1006 | Arithmetic overflow in reserve calculation |

Without the error bytes, the caller cannot distinguish "the expiry was in the past"
from "the factory was not initialized" from "the reserve amount was negative." All three
produce the same result: `success: false, error: None`.

## Why this matters

In a production deployment, `batch_initialize` may deploy dozens of accounts in a single
transaction. If one account fails and the error is swallowed, the operator has no
diagnostic information. They cannot:
- Determine which request failed (the `account_address` is present, but the error is not).
- Determine why it failed (all failure modes look identical).
- Retry the failed request with corrected parameters (they do not know what to correct).

The `error` field was designed for this purpose — it is `Option<Bytes>` specifically to
carry serialized error information. Leaving it `None` defeats the design.

## The fix

The `Err` branch should serialize the error into the `error` field. Soroban errors can
be converted to their numeric codes via the `contracterror` derive macro. A minimal fix:

```rust
Err(e) => AccountInitResult {
    account_address: account_address.clone(),
    success: false,
    error: Some(Bytes::from_array(&env, &(e as u32).to_be_bytes())),
},
```

This gives the caller a numeric error code they can inspect and act on.

## Lessons learned

When a function returns a per-item result type with an error field, every failure path
must populate that field. An `error: None` on a `success: false` result is a contradiction:
it says "this failed" without saying "why." This pattern is common in batch operations
and is easy to introduce as a placeholder during development. The placeholder must be
replaced before production.

---

## Related Issues

- [#325](https://github.com/bridgelet-org/bridgelet-core/issues/325) — AccountFactory::batch_initialize always sets error: None
- [#303](https://github.com/bridgelet-org/bridgelet-core/issues/303) — Incomplete test coverage for initialize() error paths
- [#329](https://github.com/bridgelet-org/bridgelet-core/issues/329) — Inconsistent Cargo.lock committing practice
