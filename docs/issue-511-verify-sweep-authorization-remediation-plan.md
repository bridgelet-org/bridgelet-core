# verify_sweep_authorization Remediation Plan (#511)

## Current state (re-verified against `main`, not the state described in the issue)
The issue describes `verify_sweep_authorization()` as accepting "any signature as a placeholder."
That is **not** what is currently in the codebase:

- `contracts/sweep_controller/src/authorization.rs::verify_sweep_auth` (lines 178-216) performs a
  real `env.crypto().ed25519_verify()` check against a stored `authorized_signer` public key, over
  `SHA256(account || destination || nonce_be64 || contract_id)`.
- `contracts/ephemeral_account/src/lib.rs::verify_sweep_authorization` (lines 567-607) independently
  performs the equivalent Ed25519 check against its own stored signer.

Both are real signature verification, not a stub. The doc comment on `verify_sweep_auth` (lines
145-156) already tracks the one known residual gap under `(#411)`.

## Residual gap (the actual remaining work)
`ed25519_verify()` **panics** on an invalid signature instead of returning
`Err(SignatureVerificationFailed)`. This means callers cannot distinguish "bad signature" from other
abort causes via a typed `Result`, and any wrapping/try-call logic on the SDK side must handle a
raw transaction abort rather than a contract error code.

## Remediation plan
1. Coordinate with the corresponding `bridgelet-sdk` issue so the SDK's signer stub and this
   contract's verifier are not tested against each other while either side is still fake — confirm
   SDK-side status before merging any contract-side change here.
2. Evaluate wrapping `ed25519_verify` in a way that surfaces a typed error instead of panicking,
   or explicitly document (in both repos) that signature failure is panic-only by design.
3. Confirm `contracts/sweep_controller/src/test.rs` covers both valid-signature acceptance
   (`test_execute_sweep_unauthorized_signer_fails` and neighbors) and forged/invalid-signature
   rejection at the nonce boundary — add missing cases if any are found.
4. Update issue #511 itself to reflect that the "stub" premise is outdated, so future readers
   don't re-litigate already-shipped Ed25519 verification.
