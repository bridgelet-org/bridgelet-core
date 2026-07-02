# Reentrancy Analysis: Soroban Execution Model

This document fulfills the requirement in issue #108: document which Soroban properties prevent reentrancy in the `sweep` and `record_payment` flows, and describe the tests that verify these invariants.

---

## What Is Reentrancy?

Reentrancy occurs when a contract's state can be observed and mutated by an external callback *before* the original call has finished updating that state. The classic EVM pattern is:

1. Contract A checks a balance, then calls Contract B (token transfer).
2. Contract B's fallback re-enters Contract A before the balance is decremented.
3. The attacker drains funds by repeating step 1 with the stale pre-update balance.

---

## Soroban Properties That Prevent Reentrancy

### 1. Single-threaded, sandboxed WASM execution

Each Soroban contract invocation runs inside a sandboxed WASM instance. There is no preemption, no concurrency, and no interrupt mechanism within a single transaction. A contract call **cannot** be interrupted mid-execution by another call from the same transaction. The entire call stack is strictly sequential.

### 2. Snapshot-consistent storage reads across cross-contract calls

Storage changes made within a call frame are committed atomically to the host's ledger state when that frame returns. Other contracts that are called *during* the execution see the state as it was when the outermost transaction began, not partial intermediate state. This eliminates the "read-before-write" window that EVM reentrancy exploits.

### 3. Explicit, pull-based authorization model

Soroban's authorization model is pull-based: authorization entries are attached to the transaction before it executes. There is no push-based "receive hook" or fallback function that fires on token receipt. An attacker cannot inject a callback into a sweep transaction.

---

## How `EphemeralAccount::sweep()` Protects Itself

Even if Soroban's runtime model were somehow bypassed, the contract-level logic provides an independent layer of defense:

```rust
// Update status BEFORE any external work
storage::set_status(&env, AccountStatus::Swept);
storage::set_swept_to(&env, &destination);

// ... emit events, reclaim reserve ...
```

The `status = Swept` write happens **before** any downstream operations. A reentrant call to `sweep()` would immediately fail the guard:

```rust
if storage::get_status(&env) == AccountStatus::Swept {
    return Err(Error::AlreadySwept);
}
```

This is the same check-effects-interactions (CEI) pattern recommended for EVM contracts, applied here as an additional safety layer.

---

## How `record_payment` Is Protected

`record_payment` does not check account status before recording. A caller could invoke `record_payment` after a sweep has occurred (injecting a new asset) and then attempt a second sweep. This is blocked because:

- `sweep()` checks `status == Swept` first, returning `AlreadySwept` immediately.
- The second call never reaches the payment-reading code.

---

## Test Coverage

The following tests in `contracts/ephemeral_account/src/test.rs` verify these invariants:

### `test_reentrancy_sweep_blocked_by_already_swept_guard`

Simulates what a reentrant attacker would attempt: calls `sweep()` twice on the same account. Asserts that the second call returns `Error::AlreadySwept`, confirming the state-write-first guard works.

### `test_reentrancy_record_payment_then_sweep_replay_blocked`

Simulates a more sophisticated attack: after a successful sweep, injects a new payment via `record_payment` for a different asset, then attempts a second sweep. Asserts that the second sweep is blocked by `AlreadySwept`.

### `test_replay_sweep_call_does_not_reclaim_twice` (existing)

Verifies that the reserve reclaim lifecycle does not allow double-claiming even when `sweep()` is called twice. The second call panics (contract error), and reserve event counts remain unchanged.

### `test_reserve_double_claim_prevention` (existing)

Verifies that `reclaim_reserve()` returns `0` and emits a no-op event when called after the reserve is fully reclaimed.

---

## Summary

| Protection Layer | Mechanism | Covers |
| :--- | :--- | :--- |
| Soroban runtime | Single-threaded WASM, no preemption | All cross-contract callback vectors |
| Soroban storage model | Atomic snapshot-consistent reads | Read-before-write window |
| Contract logic (CEI) | `status = Swept` written before external work | Any hypothetical intra-transaction reentry |
| `AlreadySwept` guard | Status check at entry of `sweep()` | Replay and reentrant sweep calls |
| Reserve idempotency | `reclaim_reserve()` returns 0 when fully reclaimed | Reserve double-claim |

The combination of Soroban's execution model and the contract's CEI pattern means reentrancy is not a viable attack vector against `sweep()` or `record_payment()`.
