<!--
Purpose: Checklist covering every replay-relevant mechanism across the system.
Owner: dzekojohn4 (closes #307).
Status: Documentation-only. No contracts, docs, scripts, or tools changes are introduced by this file.
-->

# Replay-Protection Checklist

> **Purpose.** Confirm that every state-changing operation in the Bridgelet Core system
> is protected against replay — signature replay, transaction replay, and state-replay
> via stale nonces or status.

| Field | Value |
| :--- | :--- |
| **Related issue** | [#307](https://github.com/bridgelet-org/bridgelet-core/issues/307) |
| **Owner / reviewer** | `_operator-name_` |
| **Last reviewed** | `_ISO-8601 date_` |

## Table of Contents

1. [SweepController Nonce](#sweepcontroller-nonce)
2. [EphemeralAccount Status Guard](#ephemeralaccount-status-guard)
3. [Cross-Contract Replay](#cross-contract-replay)
4. [Cross-Account Replay](#cross-account-replay)
5. [Destination Lock and Nonce Interaction](#destination-lock-and-nonce-interaction)
6. [Signer Rotation Timelock](#signer-rotation-timelock)
7. [Factory Salt Uniqueness](#factory-salt-uniqueness)
8. [Sign-Off](#sign-off)

---

## SweepController Nonce

The `SweepController` maintains a monotonically increasing nonce (`DataKey::SweepNonce`)
that starts at 0 at `initialize()` and increments by 1 after every successful
`execute_sweep()`.

- [ ] **Nonce increments on `execute_sweep`**: Confirm that `authorization::increment_nonce(env)`
      is called inside `sweep_account()` after successful verification and before any
      external contract calls. (`sweep_controller/src/lib.rs:205–208`)
- [ ] **Nonce is included in the signed message**: Confirm that `construct_sweep_message()`
      reads the current nonce and includes it as 8-byte big-endian in the signed payload.
      (`authorization.rs:49–79`)
- [ ] **Nonce is NOT incremented on `claim`**: Confirm that `claim()` does not call
      `increment_nonce()` — this is by design (claim uses Soroban auth, not Ed25519),
      but the implication for `update_authorized_destination` must be understood.
      See [`postmortems/claim-nonce-bypass.md`](postmortems/claim-nonce-bypass.md).
- [ ] **Off-chain signer uses current nonce**: Confirm that the SDK/off-chain pipeline
      reads `get_nonce()` before constructing the signed payload. A stale nonce
      produces a valid-looking signature that will fail on-chain.

---

## EphemeralAccount Status Guard

The `EphemeralAccount` enforces a strict state machine:
`Active → PaymentReceived → Swept | Expired`.

- [ ] **`AlreadySwept` guard exists on `sweep()`**: Confirm that `sweep()` checks
      `storage::get_status(&env) == AccountStatus::Swept` and returns
      `Error::AlreadySwept`. (`ephemeral_account/src/lib.rs:185–187`)
- [ ] **`AlreadySwept` guard exists on `sweep_claim()`**: Confirm that `sweep_claim()`
      delegates to `execute_sweep_core()`, which includes the same guard.
      (`ephemeral_account/src/lib.rs:213–221`)
- [ ] **`InvalidStatus` guard exists on `expire()`**: Confirm that `expire()` checks
      `status == Swept || status == Expired` and returns `Error::InvalidStatus`.
      (`ephemeral_account/src/lib.rs:289–292`)
- [ ] **`InvalidStatus` guard exists on `recover()`**: Confirm that `recover()` checks
      the same condition. (`ephemeral_account/src/lib.rs:417–419`)
- [ ] **Status set before external calls (CEI)**: Confirm that `execute_sweep_core()`
      calls `storage::set_status(env, AccountStatus::Swept)` before any downstream
      operations (reserve reclaim, event emission). (`ephemeral_account/src/lib.rs:235`)
- [ ] **No test exercises replay of `sweep` after `claim`**: Check test coverage for
      the case where an account is swept via `claim` and then `sweep` is attempted.
      (`sweep_controller/tests/integration.rs:421–435`)

---

## Cross-Contract Replay

A signature produced for one `SweepController` instance must not be valid on another.

- [ ] **Contract ID in signed message**: Confirm that `construct_sweep_message()` includes
      the sweep controller's own contract address (`env.current_contract_address()`)
      in the signed payload. (`authorization.rs:83–84`)
- [ ] **Contract ID in ephemeral account's verification**: Confirm that
      `verify_sweep_authorization()` in `EphemeralAccount` includes the ephemeral
      account's own contract address (not the controller's) in its message.
      (`ephemeral_account/src/lib.rs:559–560`)

> **Note:** The two contracts use different message formats. The sweep controller signs
> `SHA256(account || destination || nonce || controller_id)`. The ephemeral account signs
> `SHA256(destination || nonce || account_id)`. These are not interchangeable.

---

## Cross-Account Replay

A signature produced for one ephemeral account must not be valid on another.

- [ ] **Account address in sweep controller's message**: Confirm that
      `construct_sweep_message()` includes the ephemeral account address as the first
      component. (`authorization.rs:62–63`)
- [ ] **No hardcoded account addresses in signed messages**: Confirm that all addresses
      in the signed payload are derived from caller arguments or on-chain state, not
      from constants.

---

## Destination Lock and Nonce Interaction

The `update_authorized_destination` function uses the nonce to determine whether a
sweep has occurred. This creates a coupling between nonce handling and destination lock
enforcement.

- [ ] **Nonce > 0 blocks destination update**: Confirm that `update_authorized_destination`
      checks `nonce > 0` and returns `Error::AccountAlreadySwept`.
      (`sweep_controller/src/lib.rs:342–344`)
- [ ] **Understand `claim` path implication**: The `claim` path does not increment the
      nonce. Confirm that operators understand this means the destination lock can be
      changed after a `claim` sweep. See [`postmortems/claim-nonce-bypass.md`](postmortems/claim-nonce-bypass.md).

---

## Signer Rotation Timelock

`update_authorized_signer` initiates a time-locked rotation; `apply_signer_update`
applies it after the timelock elapses.

- [ ] **Timelock is 48 hours (34,560 ledgers)**: Confirm `SIGNER_TIMELOCK_LEDGERS`
      matches the documented value. (`sweep_controller/src/lib.rs:27`)
- [ ] **Pending signer is cleared after application**: Confirm that `apply_signer_update`
      calls `storage::clear_pending_signer()`. (`sweep_controller/src/lib.rs:442`)
- [ ] **Stale signatures rejected after rotation**: After `apply_signer_update`, the
      old signer's signatures should fail because the authorized signer has changed.
      Confirm this is the case.

---

## Factory Salt Uniqueness

`AccountFactory::batch_initialize` uses a monotonic nonce mixed into the deployment
salt to ensure unique addresses across separate invocations.

- [ ] **Nonce incremented once per call**: Confirm that `BatchNonce` is incremented
      exactly once at the start of `batch_initialize`, not inside the loop.
      (`account_factory/src/lib.rs:95–105`)
- [ ] **Salt layout prevents index collisions**: Confirm that the 32-byte salt is
      `[nonce(8 bytes) || zeros(20 bytes) || index(4 bytes)]` and that the nonce
      is in the high bytes. (`account_factory/src/lib.rs:116–119`)
- [ ] **Cross-invocation uniqueness verified by test**: Confirm that
      `test_batch_initialize_call_nonce_produces_unique_salts_across_calls` exists
      and passes. (`account_factory/src/test.rs`)

---

## Sign-Off

| Role | Name | Signature / commit SHA | Date (UTC) |
| :--- | :--- | :--- | :--- |
| Operator |  |  |  |
| Reviewer  |  |  |  |

---

## Related Issues

- [#312](https://github.com/bridgelet-org/bridgelet-core/issues/312) — claim()'s nonce-bypass breaking the destination lock guarantee
- [#329](https://github.com/bridgelet-org/bridgelet-core/issues/329) — Inconsistent Cargo.lock committing practice
- [`postmortems/claim-nonce-bypass.md`](postmortems/claim-nonce-bypass.md) — Claim nonce-bypass postmortem
- [`postmortems/account-factory-salt-collision.md`](postmortems/account-factory-salt-collision.md) — Factory salt collision postmortem
