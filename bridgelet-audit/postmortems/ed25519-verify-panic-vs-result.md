<!--
Purpose: Postmortem documenting that ed25519_verify traps rather than returning a Result, causing verify_sweep_auth's declared return type to be misleading.
Owner: DeEvelyn (closes #316).
Status: Documentation-only. No contracts, docs, scripts, or tools changes are introduced by this file.
-->

# Postmortem: ed25519_verify Panics Instead of Returning a Result

| Field | Value |
| :--- | :--- |
| **Related issue** | [#316](https://github.com/bridgelet-org/bridgelet-core/issues/316) |
| **Owner / reviewer** | `_operator-name_` |
| **Last reviewed** | `_ISO-8601 date_` |

## What `ed25519_verify` does

Soroban's `env.crypto().ed25519_verify(public_key, hash, signature)` performs constant-time
Ed25519 signature verification. Its return type is `()` — it returns nothing on success.
On failure, it **panics** (aborts the entire Soroban transaction). It does not return a
boolean, a `Result`, or an error code.

This behavior is correct from a security standpoint: a failed signature check indicates
tampering or a bug, and allowing partial state progression would be dangerous. Panicking
is the right thing to do.

## The problem in this codebase

`verify_sweep_auth()` in `contracts/sweep_controller/src/authorization.rs` is declared as:

```rust
pub fn verify_sweep_auth(
    env: &Env,
    account: &Address,
    destination: &Address,
    signature: &BytesN<64>,
) -> Result<(), Error> {
```

The `Result<(), Error>` signature implies the function can return a recoverable error.
In practice:

1. If the signature format is wrong, `validate_signature_length()` returns
   `Err(Error::InvalidSignature)` — a recoverable error. ✓
2. If the authorized signer is not configured, `get_authorized_signer()` returns
   `Err(Error::AuthorizedSignerNotSet)` — a recoverable error. ✓
3. If the signer key format is wrong, `validate_signer_key()` returns
   `Err(Error::InvalidSignature)` — a recoverable error. ✓
4. If `ed25519_verify` fails (wrong signature), it **panics**. ✗

The function's declared return type is `Result<(), Error>`, but the actual failure behavior
is a panic. Callers who pattern-match on `Err(Error::...)` variants after calling this
function will never see the Ed25519 failure — the transaction will abort before the
`Err` is constructed.

## Which Error variants become unreachable

The `sweep_controller::Error` enum includes `SignatureVerificationFailed` (2008), defined
as "the cryptographic signature verification primitive returned a failure." This variant
can never be returned because `ed25519_verify` panics instead of producing a value.
Similarly, `InvalidNonce` (2010) is defined but never explicitly returned anywhere in
the current code.

These are dead-code variants: they exist in the enum but no code path constructs them.

## Why this happened

When wrapping a host function that panics in a `Result`-returning Rust function, the author
must decide: should the wrapper catch the panic and return `Err`, or let it propagate?
In this codebase the decision was intentional (panicking aborts the transaction cleanly),
but the declared return type was not updated to reflect it. The function looks like it
returns `Result`, which misleads readers into thinking the failure is recoverable.

## Lessons learned

When wrapping a host function that has panic-only failure behavior, the wrapper's return
type should reflect the actual failure modes. Two valid options:

1. **Keep the panic** and change the return type to `()` (or document clearly that the
   `Result` is only for pre-verification errors, and the actual verification panics).
2. **Catch the panic** (via `std::panic::catch_unwind` or Soroban's equivalent) and
   convert it to `Err(Error::SignatureVerificationFailed)`.

Either approach is defensible; the issue is that the current code mixes both: it declares
a `Result` but panics on the most important failure path. A reader who trusts the return
type will write error-handling code that never executes.

---

## Related Issues

- [#314](https://github.com/bridgelet-org/bridgelet-core/issues/314) — sweep()'s unused auth_signature parameter
- [#315](https://github.com/bridgelet-org/bridgelet-core/issues/315) — Error::TransferFailed being unreachable dead code
- [#299](https://github.com/bridgelet-org/bridgelet-core/issues/299) — Panic-vs-typed-Result review checklist
