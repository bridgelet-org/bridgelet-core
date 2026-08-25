<!--
Purpose: Postmortem documenting how claim()'s nonce-bypass breaks the destination lock guarantee.
Owner: ibrahimmosouf-png (closes #312).
Status: Documentation-only. No contracts, docs, scripts, or tools changes are introduced by this file.
-->

# Postmortem: claim()'s Nonce-Bypass Breaking the Destination Lock Guarantee

| Field | Value |
| :--- | :--- |
| **Related issue** | [#312](https://github.com/bridgelet-org/bridgelet-core/issues/312) |
| **Owner / reviewer** | `_operator-name_` |
| **Last reviewed** | `_ISO-8601 date_` |

## The two sweep paths

`SweepController` exposes two independent paths to sweep funds from an `EphemeralAccount`:

**Path A: `execute_sweep(ephemeral_account, destination, auth_signature)`**
1. Verifies the Ed25519 signature against the stored `authorized_signer`.
2. **Increments the sweep nonce** (in `sweep_account`, line 206–208 of `lib.rs`).
3. Delegates to `EphemeralAccount::sweep()`.

**Path B: `claim(recipient, ephemeral_account)`**
1. Calls `recipient.require_auth()` (Soroban auth entry).
2. Validates destination against locked destination (if set).
3. Delegates to `EphemeralAccount::sweep_claim()` via `authorize_claim()`.
4. **Does not increment the sweep nonce.**

The `claim` path skips `sweep_account()` entirely — it calls `authorize_claim()` directly,
which does not touch the nonce.

## How the destination lock works

`update_authorized_destination(new_destination)` lets the admin change the locked destination
**only before the first sweep**. The guard is:

```rust
let nonce = storage::get_sweep_nonce(&env);
if nonce > 0 {
    return Err(Error::AccountAlreadySwept);
}
```

This check relies on the nonce being incremented after every sweep. If the nonce is `0`,
no sweep has occurred, and the destination can be changed. If the nonce is `> 0`, a sweep
has occurred, and the lock is permanent.

## The bypass

Because `claim` does not increment the nonce, an account can be swept via `claim` while
the nonce remains at `0`. After the sweep:

1. The funds have already been transferred to the recipient.
2. The account status is `Swept` (set by `EphemeralAccount::execute_sweep_core`).
3. But the sweep nonce is still `0`.

At this point, `update_authorized_destination` sees `nonce == 0` and allows the admin
to change the destination — even though funds have already been transferred. The destination
lock's documented guarantee ("once a sweep has been executed, the destination cannot be
changed") is broken.

## Worked example

1. Admin initializes a `SweepController` with `authorized_destination = Some(alice)`.
2. An account is created and funded.
3. `claim(bob, account)` succeeds — bob receives the funds, account is `Swept`, nonce is `0`.
4. Admin calls `update_authorized_destination(carol)` — succeeds because `nonce == 0`.
5. A second account is created and funded.
6. `claim(carol, account2)` succeeds — the destination lock was changed after the first
   sweep, even though the guarantee says it should be immutable post-sweep.

## Concrete consequence

The `update_authorized_destination` function's guard is only effective for the
`execute_sweep` path. For the `claim` path, the guard is meaningless: funds can be swept
via `claim`, and the destination can still be changed afterward. An integrator who reads
the `update_authorized_destination` docstring — "Once a sweep has been executed, the
destination cannot be changed" — and relies on it for policy enforcement would be making
a security decision based on an invariant that does not hold across both paths.

## Lessons learned

When a system has two independent paths to the same effect, every invariant needs to hold
across both paths, not just the one that was designed first. The nonce increment in
`sweep_account()` was designed for the `execute_sweep` path. The `claim` path was added
later as a gas-free alternative, but the nonce was not incremented there because the
`claim` path does not use Ed25519 signatures (and thus does not need replay protection
for the signature). However, the nonce is also used for the destination lock — a
completely separate invariant — and that invariant breaks when the nonce is not incremented.

---

## Related Issues

- [#324](https://github.com/bridgelet-org/bridgelet-core/issues/324) — docs/security.md not documenting the claim() authorization path
- [#311](https://github.com/bridgelet-org/bridgelet-core/issues/311) — AccountFactory's unprotected initialize()
- [#309](https://github.com/bridgelet-org/bridgelet-core/issues/309) — record_payment's missing access control
