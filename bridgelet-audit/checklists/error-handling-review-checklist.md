<!--
Purpose: Checklist for reviewing error handling patterns across Bridgelet Core contracts.
Owner: YaronZaki (closes #299).
Status: Documentation-only. No contracts, docs, scripts, or tools changes are introduced by this file.
-->

# Error-Handling Review Checklist

> **Purpose.** Catch the most common error-handling foot-guns in Soroban smart contracts
> before they reach production. Each item below was motivated by a real issue found during
> the Bridgelet Core audit.

| Field | Value |
| :--- | :--- |
| **Related issue** | [#299](https://github.com/bridgelet-org/bridgelet-core/issues/299) |
| **Owner / reviewer** | `_operator-name_` |
| **Last reviewed** | `_ISO-8601 date_` |

## Table of Contents

1. [Panic vs Typed Result](#1-panic-vs-typed-result)
2. [expect() in Production Code](#2-expect-in-production-code)
3. [Host-Function Failure Semantics](#3-host-function-failure-semantics)
4. [Partial Failure in Batch Operations](#4-partial-failure-in-batch-operations)
5. [Silent Success on Invalid Input](#5-silent-success-on-invalid-input)
6. [Error-Code Collisions](#6-error-code-collisions)
7. [Propagation Chain Integrity](#7-propagation-chain-integrity)
8. [CEI Ordering with Errors](#8-cei-ordering-with-errors)
9. [Error Variant Exhaustiveness](#9-error-variant-exhaustiveness)
10. [Sign-Off](#sign-off)

---

## 1. Panic vs Typed Result

**Rule:** A function declared `-> Result<T, Error>` must return `Err(Error::...)` on every
failure path. It must never panic, trap, or abort silently.

**Why:** Callers who pattern-match on `Err(Error::...)` variants assume those variants are
reachable. If the function panics instead, the error-handling code is dead code, and the
caller cannot distinguish "bad input" from "host crash."

**Checklist:**

- [ ] Every function declared `-> Result<T, Error>` returns `Err(...)` on failure.
- [ ] No `unwrap()`, `expect()`, `panic!()`, or host-function traps in `Result`-returning
      functions.
- [ ] If a host function panics (e.g., `ed25519_verify`), the wrapper either catches the
      panic or changes its return type to `()`.

**Known violation:** `verify_sweep_auth()` in `sweep_controller/src/authorization.rs`
declares `Result<(), Error>` but `ed25519_verify` panics on failure.
See [`postmortems/ed25519-verify-panic-vs-result.md`](../postmortems/ed25519-verify-panic-vs-result.md).

---

## 2. expect() in Production Code

**Rule:** `expect()` in production code paths is acceptable only when the condition it
guards is structurally impossible (e.g., a value that was just written and immediately
read back in the same function). In all other cases, use `ok_or(Error::...)?.` or
`if let Some(...) = ... else { return Err(...) }`.

**Why:** `expect()` produces an opaque panic message. A contract that panics with
"factory not initialized; call initialize() first" gives the caller no typed error to
handle. The same contract that returns `Err(Error::NotInitialized)` gives the caller
a clear, actionable error code.

**Checklist:**

- [ ] No `expect()` on storage reads that depend on external state.
- [ ] No `expect()` on cross-contract call returns.
- [ ] `expect()` used only for structurally impossible conditions (e.g., values just
      written in the same transaction).

**Known violation:** `AccountFactory::batch_initialize()` calls
`expect("factory not initialized; call initialize() first")` on a storage read.
See [`postmortems/account-factory-expect-panic.md`](../postmortems/account-factory-expect-panic.md).

---

## 3. Host-Function Failure Semantics

**Rule:** Before wrapping a host function, confirm whether it returns a `Result`, panics,
or traps. Match the wrapper's return type to the actual failure behavior.

**Why:** A host function that panics cannot be caught by `?` or `match`. A host function
that returns `Result` can. Mixing the two in a single function creates unreachable code
paths and misleading signatures.

**Checklist:**

- [ ] Identify all host functions called in the codebase and their failure semantics.
- [ ] Verify that the wrapper's return type matches the host function's behavior.
- [ ] If the host function panics and the wrapper declares `Result`, the panic path
      must be caught or the return type must be changed.

**Host-function reference for Soroban:**

| Host function | Failure behavior |
| :--- | :--- |
| `env.crypto().ed25519_verify(...)` | Panics on invalid signature |
| `env.crypto().ed25519_verify(...)` returns `()` | Returns nothing on success |
| `env.storage().instance().get(...)` | Returns `Option`; no panic |
| `env.storage().persistent().get(...)` | Returns `Option`; no panic |
| `env.storage().temporary().get(...)` | Returns `Option`; no panic |
| `env.invoke_contract(...)` | Propagates the callee's error; no panic in caller |

---

## 4. Partial Failure in Batch Operations

**Rule:** When a batch operation (e.g., `batch_initialize`) processes multiple items, the
failure semantics must be documented and consistent. Either:
- All items succeed or all fail (transactional), or
- Each item's result is reported independently, with no side effects from failed items.

**Why:** A batch that silently swallows errors leaves the caller with a partial state
and no indication of which items failed. Soroban does not support rollback within a
single transaction, so the contract must explicitly track and report partial failures.

**Checklist:**

- [ ] Batch operations return per-item results (e.g., `Vec<AccountInitResult>`).
- [ ] Each per-item result includes an error field for failure reporting.
- [ ] No failed item has side effects (storage writes, external calls) that persist.
- [ ] The caller can inspect each item's result to determine success/failure.

**Known issue:** `AccountFactory::batch_initialize()` always sets
`error: None` in `AccountInitResult`, even when `create_wrapped_account` fails.
See [`postmortems/account-factory-expect-panic.md`](../postmortems/account-factory-expect-panic.md).

---

## 5. Silent Success on Invalid Input

**Rule:** A function that accepts user input must validate that input and return a typed
error on invalid values. It must not silently accept invalid input and return `Ok(())`.

**Why:** Silent success on invalid input is a foot-gun for integrators. If a function
accepts a negative value and returns `Ok(())`, the caller has no way to know the value
was invalid. The invalid state persists until something else breaks.

**Checklist:**

- [ ] All numeric inputs are bounds-checked (positive, non-zero, within range).
- [ ] All address inputs are validated (non-zero, correct length).
- [ ] All enum/status inputs are validated against allowed transitions.
- [ ] Invalid inputs return typed errors, not silent success.

---

## 6. Error-Code Collisions

**Rule:** Every error variant across all contracts must have a unique numeric code, even
across contract boundaries. A transaction that calls two contracts must not produce
ambiguous error codes.

**Why:** Soroban surfaces errors as `u32` codes. If two contracts use the same code
for different errors, the SDK cannot distinguish them. The `SharedError` enum in
`contracts/shared/src/errors.rs` defines shared codes 1–7; contract-specific codes
must not overlap.

**Checklist:**

- [ ] No contract-specific error code overlaps with `SharedError` codes (1–7).
- [ ] No two contracts use the same numeric code for different errors.
- [ ] The collision test in `shared/src/errors.rs` passes.
- [ ] New error variants are added at the end of the enum (highest code) to avoid
      shifting existing codes.

---

## 7. Propagation Chain Integrity

**Rule:** When function A calls function B, and B returns `Err(...)`, the error must
propagate to A's caller without being swallowed, transformed, or lost.

**Why:** An error that is swallowed at an intermediate call site becomes invisible to the
caller. The caller expects to see the error and cannot handle it if it is lost.

**Checklist:**

- [ ] Every `?` operator propagates the correct error type.
- [ ] No `match` arms that silently discard errors (e.g., `Err(_) => ()`).
- [ ] No `.ok()` or `.unwrap_or(...)` that converts errors to default values without
      the caller's knowledge.
- [ ] Cross-contract call errors are propagated, not caught and replaced.

---

## 8. CEI Ordering with Errors

**Rule:** Checks, effects, and interactions must be ordered correctly even when errors
are involved. Specifically:
1. **Checks** (validation, guards) come first.
2. **Effects** (storage writes) come next.
3. **Interactions** (cross-contract calls) come last.

**Why:** If an effect (storage write) happens before a check fails, the storage is
mutated even though the operation should have been rejected. If an interaction happens
before an effect, a re-entrant call could see stale state.

**Checklist:**

- [ ] All validation checks precede storage writes.
- [ ] All storage writes precede cross-contract calls.
- [ ] If a function does checks → effects → interactions, the error path does not
      skip the effects (i.e., the effects are conditional on the checks passing, not
      the interactions succeeding).

---

## 9. Error Variant Exhaustiveness

**Rule:** Every error variant in the enum must be constructed by at least one code path.
If a variant is never constructed, it is dead code and should be removed or documented
as reserved.

**Why:** Unused error variants mislead readers into thinking a failure mode is handled
when it is not. They also inflate the enum size and make error-code audits harder.

**Checklist:**

- [ ] Every error variant is constructed by at least one `return Err(Error::...)` or
      `Err(SharedError::...)` expression.
- [ ] No error variants exist solely for "future use" without documentation.
- [ ] Grep for each variant name to confirm it is used in a return position.

**Known untested variants:**
- `sweep_controller::Error::SignatureVerificationFailed` (2008) — never constructed
  (see [`postmortems/ed25519-verify-panic-vs-result.md`](../postmortems/ed25519-verify-panic-vs-result.md))
- `sweep_controller::Error::InvalidNonce` (2010) — never constructed in current code
- `shared::Error::InvalidAmount` (7) — only constructed in `reserve_contract`, never
  in `ephemeral_account` or `sweep_controller`

---

## Sign-Off

| Role | Name | Signature / commit SHA | Date (UTC) |
| :--- | :--- | :--- | :--- |
| Operator |  |  |  |
| Reviewer  |  |  |  |

---

## Related Issues

- [#316](https://github.com/bridgelet-org/bridgelet-core/issues/316) — verify_sweep_auth's Result signature not matching its actual failure behavior
- [#315](https://github.com/bridgelet-org/bridgelet-core/issues/315) — Error::TransferFailed being unreachable dead code
- [#313](https://github.com/bridgelet-org/bridgelet-core/issues/313) — Missing test coverage for EphemeralAccount::sweep with Ed25519 signatures
- [`postmortems/ed25519-verify-panic-vs-result.md`](../postmortems/ed25519-verify-panic-vs-result.md) — ed25519_verify panic-vs-result mismatch
- [`postmortems/account-factory-expect-panic.md`](../postmortems/account-factory-expect-panic.md) — AccountFactory expect() panic on storage read
