<!--
Purpose: Postmortem documenting the gap where docs/security.md fails to describe the claim() authorization path as a distinct entry point.
Owner: Magrexy (closes #324).
Status: Documentation-only. No contracts, docs, scripts, or tools changes are introduced by this file.
-->

# Postmortem: docs/security.md Missing the claim() Authorization Path

| Field | Value |
| :--- | :--- |
| **Related issue** | [#324](https://github.com/bridgelet-org/bridgelet-core/issues/324) |
| **Owner / reviewer** | `_operator-name_` |
| **Last reviewed** | `_ISO-8601 date_` |

## What happened

`docs/security.md` was written to describe the Bridgelet Core authorization model. It documents
`execute_sweep` (path 2a) in detail: the Ed25519 signature flow, nonce increment, and Soroban
auth delegation. It then separately describes a "Claim Operations" section (numbered 2a again,
an unrelated numbering bug) that covers `claim` using Soroban auth entries.

However, the claim path is **not referenced in the Threat Model section**, is not mentioned in
the "Security Guarantees" section, and the "Known Limitations and Assumptions" section
explicitly states that `EphemeralAccount::sweep` verification is a placeholder — without
noting that `claim` bypasses the Ed25519 verification entirely by design. The net effect is
that a reader of the threat model has no visibility into the claim path as a separate
authorization surface.

## What `claim()` does

`SweepController::claim(recipient, ephemeral_account)` provides a gas-free sweep path:

1. The recipient signs a Soroban auth entry for `claim(recipient, ephemeral_account)`.
2. A relayer or SDK submits the transaction and pays the transaction fee.
3. `SweepController` authorizes itself as the invoker of `EphemeralAccount::sweep_claim`
   via `authorize_as_current_contract()`.
4. `EphemeralAccount::sweep_claim` verifies `controller.require_auth()` and delegates
   to `execute_sweep_core()`.

This is a **fully separate authorization surface** from `execute_sweep`. The two paths diverge
on multiple dimensions:

| Dimension | `execute_sweep` | `claim` |
| :--- | :--- | :--- |
| Signature type | Ed25519 (off-chain signer) | Soroban auth entry (recipient) |
| Nonce handling | Nonce increments before external calls | Nonce is **not** incremented |
| Destination lock | Checked via `validate_destination` | Checked via `validate_destination` |
| Token transfers | Handled by `SweepController` | Handled by `SweepController` |

## Why this matters

The `claim` path's omission from the threat model means the following risks are undocumented:

1. **Nonce bypass**: `claim` does not increment the sweep nonce (see [#312](https://github.com/bridgelet-org/bridgelet-core/issues/312)).
   An integrator who reads only the threat model would not know that `update_authorized_destination`'s
   lock (which relies on `nonce > 0`) can be circumvented by calling `claim` instead of `execute_sweep`.

2. **Dual entry point confusion**: Both `execute_sweep` and `claim` achieve the same end-state
   (funds transferred, account marked `Swept`). If a monitoring system only watches for
   `execute_sweep` events, it would miss sweeps performed via `claim`.

3. **Authorization surface expansion**: The claim path expands the set of parties who can trigger
   a sweep from "the off-chain signer" to "the off-chain signer OR the recipient". This is
   intentional design, but it should be explicit in the security documentation.

## What should have been documented

The `docs/security.md` threat model should have included:

- An explicit statement that `claim` is an independent authorization surface with different
  trust assumptions.
- The fact that `claim` does not increment the nonce, and the implications for
  `update_authorized_destination`'s lock guarantee.
- A note that `EphemeralAccount::sweep_claim` only verifies `controller.require_auth()`
  (Soroban auth), not an Ed25519 signature.

## Lessons learned

When a system has two independent paths to the same effect, every invariant that depends on
one path's behavior must be validated against the other. The claim path was designed as a
gas-free alternative to `execute_sweep`, but its different nonce behavior was not reflected
in the security documentation or the threat model analysis. Documentation that covers only
the "primary" path and omits the secondary path creates a false sense of coverage.

---

## Related Issues

- [#312](https://github.com/bridgelet-org/bridgelet-core/issues/312) — claim()'s nonce-bypass breaking the destination lock guarantee
- [#311](https://github.com/bridgelet-org/bridgelet-core/issues/311) — AccountFactory's unprotected initialize()
- [#309](https://github.com/bridgelet-org/bridgelet-core/issues/309) — record_payment's missing access control
