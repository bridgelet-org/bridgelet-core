<!--
Purpose: Postmortem documenting can_sweep's redundant double cross-contract call.
Owner: kilodesodiq-arch (closes #332).
Status: Documentation-only. No contracts, docs, scripts, or tools changes are introduced by this file.
-->

# Postmortem: can_sweep Makes Two Cross-Contract Calls When One Suffices

| Field | Value |
| :--- | :--- |
| **Related issue** | [#332](https://github.com/bridgelet-org/bridgelet-core/issues/332) |
| **Owner / reviewer** | `_operator-name_` |
| **Last reviewed** | `_ISO-8601 date_` |

## What `can_sweep` does

`SweepController::can_sweep(ephemeral_account)` in `contracts/sweep_controller/src/lib.rs:283–291`
is a read-only view function that checks whether a given `EphemeralAccount` is eligible for
sweeping. It makes two cross-contract calls to the ephemeral account:

```rust
pub fn can_sweep(env: Env, ephemeral_account: Address) -> bool {
    storage::extend_instance_ttl(&env);
    let account_client = EphemeralAccountClient::new(&env, &ephemeral_account);
    let info = account_client.get_info();
    info.status as u32 == AccountStatus::PaymentReceived as u32 && !account_client.is_expired()
}
```

**Call 1:** `account_client.get_info()` — returns `AccountInfo`, which includes
`status`, `expiry_ledger`, `creator`, `recovery_address`, `payments`, and `swept_to`.

**Call 2:** `account_client.is_expired()` — returns `bool` by comparing the current
ledger against `expiry_ledger` stored in instance storage.

## The redundancy

`get_info()` already returns `expiry_ledger: u32` (line 388 of `ephemeral_account/src/lib.rs`).
The caller (`can_sweep`) has access to the current ledger via `env.ledger().sequence()`.
The expiry check — `current_ledger > expiry_ledger` — can be computed entirely locally
after the first cross-contract call:

```rust
let info = account_client.get_info();
let expired = env.ledger().sequence() > info.expiry_ledger;
info.status == AccountStatus::PaymentReceived && !expired
```

This eliminates the second cross-contract call entirely.

## Why this matters

Each cross-contract call in Soroban incurs:
1. **Compute cost:** The callee contract's WASM must be instantiated and executed.
2. **Memory cost:** The call frame, arguments, and return values must be serialized
   and deserialized across the contract boundary.
3. **Auth cost:** If the callee requires authorization, an auth entry must be created
   and verified.

`can_sweep` is a read-only view function — it does not modify state and does not require
authorization. It is likely called off-chain or by a frontend to determine sweep eligibility.
Making two cross-contract calls for a boolean check that can be computed from data already
returned by the first call is wasteful.

For a single call, the overhead is negligible. But `can_sweep` may be called in a loop
over many accounts (e.g., "which of these 50 accounts are ready for sweep?"). In that
case, the redundant call doubles the cost per account.

## The fix

Replace the second cross-contract call with a local comparison:

```rust
pub fn can_sweep(env: Env, ephemeral_account: Address) -> bool {
    storage::extend_instance_ttl(&env);
    let account_client = EphemeralAccountClient::new(&env, &ephemeral_account);
    let info = account_client.get_info();
    let expired = env.ledger().sequence() > info.expiry_ledger;
    info.status == AccountStatus::PaymentReceived && !expired
}
```

This reduces the cross-contract calls from two to one while preserving identical behavior.

## Lessons learned

When a function calls two methods on the same remote object and the second method's
result is derivable from the first, the second call is redundant. This is a standard
optimization in distributed systems — "fetch once, compute locally." The same principle
applies to cross-contract calls in Soroban, where each call has non-trivial overhead.

---

## Related Issues

- [#297](https://github.com/bridgelet-org/bridgelet-core/issues/297) — Can_sweep makes two cross-contract calls when one suffices
- [#318](https://github.com/bridgelet-org/bridgelet-core/issues/318) — SweepController does not extend instance TTL
- [#299](https://github.com/bridgelet-org/bridgelet-core/issues/299) — Panic-vs-typed-Result review checklist
