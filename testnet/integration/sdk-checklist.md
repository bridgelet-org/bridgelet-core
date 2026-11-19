# SDK Wrapper Checklist for Testnet

## Overview

Use this checklist when building or reviewing an SDK wrapper around Bridgelet contracts on Stellar Testnet. It applies to consuming applications and their SDKs; it does not change the contracts, `scripts/`, or `docs/api-reference.md`.

> **Deployment status.** `testnet/registry/contract-status.md` is the canonical record: all four contracts are undeployed, IDs are `TBD`, and `Last Verified` is `Never`. A wrapper must be able to report "no verified deployment" instead of silently using a historical ID.

---

## Configuration and endpoints

- [ ] Set the network passphrase exactly to `Test SDF Network ; September 2015`, including the spaces around `;`.
- [ ] Use `https://soroban-testnet.stellar.org` for contract calls, simulation, and events; `https://horizon-testnet.stellar.org` for classic account and payment data.
- [ ] Resolve `EphemeralAccount`, `SweepController`, `ReserveContract`, and `AccountFactory` IDs from configuration, not source code or copied examples.
- [ ] Check `testnet/registry/contract-status.md` before a run and fail clearly if the configured ID is `TBD`, unverified, or absent.
- [ ] Verify a configured ID with a read-only invocation per `testnet/registry/verify-contract-live.md` before any state-changing call.
- [ ] Never reuse a mainnet passphrase, endpoint, or ID on testnet.

The repository's older deployment artifacts disagree with the canonical registry; `testnet/policies/reset-policy.md` documents that drift. Treat a file containing an ID as a claim, not a fact.

---

## Pre-flight and submission

- [ ] Call `simulateTransaction` (or CLI `--send no`) before every state-changing submission and surface its error to the caller.
- [ ] Treat auth failures, resource-limit failures, state errors, wrong IDs, and wrong interfaces as distinct outcomes; see `testnet/integration/simulate-before-submit.md`.
- [ ] Do not claim that a clean simulation reserves a nonce, commits state, guarantees inclusion, or proves a balance moved.
- [ ] Build transaction envelopes with the SDK rather than hand-writing XDR; decode results with the same SDK version.
- [ ] Retain the transaction hash for every submission and resolve uncertain results by hash rather than resubmitting.

---

## Nonce and retry discipline

- [ ] Read `SweepController::get_nonce()` immediately before signing; never cache it across a test step or retry.
- [ ] Serialize read → sign → simulate → submit → resolve per controller with a lock or queue.
- [ ] Bound retries to 2–3 attempts with backoff; a tight loop against a shared controller is an abuse pattern.
- [ ] Never resubmit the same signature: it is bound to a nonce that may already be consumed.
- [ ] Never re-sign with a cached nonce; re-read the chain first.
- [ ] Distinguish the signature-bearing `execute_sweep` path from signature-free `claim`; see `testnet/examples/idempotency-considerations.md`.
- [ ] Treat `ed25519_verify` traps as a possible nonce race, not as proof of a signing bug; see `testnet/runbooks/failed-sweep-signature.md`.

---

## Sweep call paths

| Path | Signature | Who signs / authorizes | What the wrapper must expose |
|---|---|---|---|
| `EphemeralAccount::sweep()` | `BytesN<64>` accepted but **not verified** | `authorized_controller.require_auth()` only | A prominent warning; not the default sweep path |
| `EphemeralAccount::sweep_claim()` | None | `authorized_controller.require_auth()` via the controller's claim flow | Internal path reached through `claim()`; do not expose as a separate user flow |
| `SweepController::execute_sweep()` | Ed25519 over `SHA256(destination_xdr ‖ nonce_be_u64 ‖ controller_id_xdr)` | Off-chain `authorized_signer` key; relayer submits | The primary signed sweep path with a fresh nonce |
| `SweepController::claim()` | No `auth_signature` parameter | `recipient.require_auth()` as a Soroban auth entry; relayer pays fees | The gas-free recipient path, including the locked-destination check |

- [ ] Route users to `execute_sweep()` or `claim()` by default; do not present direct `sweep()` as an equivalent.
- [ ] Keep destination, controller ID, nonce, and signer key consistent with the digest; a signature is not bound to a particular ephemeral account.
- [ ] Do not assume `claim` increments the nonce: the contract doc comment and `testnet/config/nonce-tracking.md` say it does, while the code path traced in `testnet/examples/idempotency-considerations.md` says it does not. Reconcile against a live deployment before relying on it.
- [ ] Do not assert token movement from the call alone. `docs/api-reference.md` describes a classic SDK payout after the contract call, while `testnet/examples/integration-test-harness-notes.md` expects SEP-41 transfers through the controller; verify against the deployed WASM before a balance-change claim.
- [ ] For a completed sweep, check transaction status, `get_status == 2`, `get_info.swept_to`, `can_sweep == false`, nonce movement, and the destination balance as separate facts.

---

## Stub warnings and unsafe paths

- [ ] Surface the warning from `testnet/security/unverified-signature-path-warning.md` wherever `sweep()` is discoverable: API docs, CLI help, type docs, and error messages.
- [ ] State plainly that a `sweep()` rejection is `require_auth()` (typically `Unauthorized`, code `8`), not signature verification, and that a success proves nothing about the signature.
- [ ] Never treat `InvalidSignature` (code `9`) as evidence of real verification; neither `EphemeralAccount::InvalidSignature` nor `SweepController::SignatureVerificationFailed` is constructed by the current source.
- [ ] Flag the forward-looking risk: if token transfers are added to `EphemeralAccount::sweep()` before real verification, the same call becomes a signature-bypass drain.

---

## Types and marshalling

- [ ] Apply `testnet/integration/type-mapping.md` consistently: `Address` as a string, `BytesN<32>` / `BytesN<64>` as raw bytes, `i128` / `u64` as `bigint` in JS/TS.
- [ ] Encode `Option<Address>` as `null` / `None` / `nil`; never substitute an empty string or zero address.
- [ ] Use exact ABI field names for `contracttype` structs and preserve unknown fields from a deployed WASM.
- [ ] Decode enum discriminants per contract with an explicit match; leave unknown codes unmapped and preserve the `SweepController` gap at `12`.
- [ ] Convert stroops and asset base units explicitly; do not mix XLM amounts with raw stroops.

---

## Events and webhooks

- [ ] Generate event fixtures from `testnet/registry/event-topics.md` and test handlers locally per `testnet/integration/webhook-simulation.md`.
- [ ] Filter on `topic[0]` and treat the payload as a `contracttype`-decoded value; do not invent symbols.
- [ ] Deduplicate on `(resetEpoch, ledger, txHash, contractId, eventIndex)` and make handlers idempotent.
- [ ] Test out-of-order delivery, duplicates, missing events, and a lower-ledger reset epoch.
- [ ] Do not infer value movement or ownership from an event; correlate with transaction results and balances.
- [ ] On a real stream, validate field names and symbols against decoded XDR when a deployment exists.

---

## Errors, retries, and resets

- [ ] Map error discriminants contract-by-contract; carry the invoking contract through the error path.
- [ ] Keep traps and auth failures in a distinct non-enum error state; never decode an integer out of them.
- [ ] Classify before retrying: reconcile benign duplicates, re-fetch where a retry can help, and stop or escalate otherwise. See `testnet/examples/error-handling.md`.
- [ ] Handle `NotExpired` by polling `is_expired()` rather than retrying on a timer.
- [ ] Detect a reset by a decreasing ledger or IDs that stop resolving, per `testnet/policies/reset-policy.md` and `testnet/runbooks/rpc-outage.md`.
- [ ] After a reset, discard cached IDs, nonces, cursors, and balances; require re-verification instead of replaying pre-reset state.
- [ ] Make test runs re-creatable: fresh `EphemeralAccount` instances per run and no shared configuration changes. See `testnet/examples/integration-test-harness-notes.md`.

---

## Secret hygiene

- [ ] Never log secret keys, signing seeds, or full signature material; signatures are single-nonce bearer artefacts.
- [ ] Load key material from environment or a secure store, never from a source file, URL, fixture, or commit.
- [ ] Redact credentials from traces, crash dumps, and issue reports. See `testnet/config/README.md` and `testnet/security/admin-key-hygiene.md`.

---

## Acceptance criteria

| Criterion | Verification |
|---|---|
| Network configuration is exact | A test asserts the passphrase string and both endpoint URLs |
| IDs are configurable and verified | Removing a configured ID produces a clear error; an unverified ID blocks state-changing calls |
| Pre-flight is mandatory | A test simulates a malformed call and confirms no submission occurs |
| Nonce handling is safe | Concurrent attempts against one controller are serialized; no signature is reused |
| Sweep paths are distinguishable | API surface separates `execute_sweep`, `claim`, and direct `sweep()`, with the stub warning attached to the last |
| Type mapping is correct | `bigint`, bytes, `Option`, and enum fixtures round-trip through the SDK layer |
| Event handling is testable offline | Local fixtures drive the webhook handler without a network call |
| Idempotency holds | Duplicate delivery and duplicate sweep attempts produce one side effect |
| Errors are scoped | A code `2` from each contract maps to that contract's variant; unknown codes stay unmapped |
| Reset behaviour is defined | A decreasing ledger or unresolved ID fails closed and requires re-verification |
| No secrets are logged | Log inspection of a full sweep run finds no key, seed, or signature material |

---

## Related Documentation

- `testnet/integration/simulate-before-submit.md` — pre-flight simulation and bounded retries
- `testnet/integration/type-mapping.md` — language-specific type and marshalling reference
- `testnet/integration/webhook-simulation.md` — local event fixtures and idempotency tests
- `testnet/security/unverified-signature-path-warning.md` — the warning wrappers must surface
- `testnet/config/nonce-tracking.md` — nonce reads and locked destinations
- `testnet/examples/idempotency-considerations.md` — duplicates, traps, and the safe retry
- `testnet/examples/error-handling.md` — contract-scoped error mapping
- `testnet/examples/integration-test-harness-notes.md` — live-testnet suite etiquette
- `testnet/policies/reset-policy.md` — reset detection and integrator posture
- `testnet/registry/contract-status.md` — canonical deployment status
- `testnet/registry/event-topics.md` — event source of truth
- `testnet/docs/network-config.md` — passphrase and endpoints
- `docs/api-reference.md` — authoritative contract API

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-11-18 | Initial SDK wrapper checklist and acceptance criteria for testnet |
