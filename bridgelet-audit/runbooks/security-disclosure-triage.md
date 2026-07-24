# Runbook: Triaging an Incoming Security Disclosure

**Audience:** on-call engineers, maintainers, and outside reporters (via the source link referenced from the future `SECURITY.md`).

**Scope:** the steps to take when someone reports a suspected vulnerability in Bridgelet Core — from the first acknowledgement through to a reproducible technical classification that the maintainers can act on.

This runbook itself is **not** the security contact page. It defines how to triage once a report has reached the team. See the future `SECURITY.md` at the repo root for how reports reach the team in the first place; that file is **not** created by the closure of this issue.

---

## 1. Acknowledge and triage intake

Within **one business day** (or sooner for a credible critical report):

1. **Acknowledge** receipt to the reporter with a unique tracking ID (e.g. `BH-2026-001`). Do not include technical details in the acknowledgement.
2. **Open a private issue / tracker entry** with:
   - Tracking ID.
   - Reporter handle (anonymized if requested).
   - Affected contract(s) — see [Which contracts are in scope](#which-contracts-are-in-scope).
   - Initial severity guess — see [Severity classification](#2-severity-classification-for-a-funds-custody-system).
   - Raw reporter notes verbatim.
3. **Confirm scope.** A report is "in scope" if it concerns any of the four deployable contracts in `contracts/`. Out-of-scope reports (e.g. third-party SDKs, Horizon misconfiguration, generic Stellar protocol bugs) should be redirected to the appropriate owner and closed within the tracker.
4. **Set a private embargo window** — default **90 days** from acknowledgement, or shorter if the reporter requests it. Critical findings may need a coordinated disclosure on a much shorter timeline (see [Severity classification](#2-severity-classification-for-a-funds-custody-system)).

---

## 2. Severity classification for a funds-custody system

Bridgelet moves and gates real funds, so the severity ladder must be calibrated to *what an attacker could do*, not to bug-hunt-friendly categories. Use the three primary classes below; pick the highest applicable one.

| Class | Definition (must hold under realistic attacker capability) | Typical Bridgelet example |
|---|---|---|
| **Critical — Fund-loss** | Attacker can move funds out of an `EphemeralAccount` to an attacker-controlled address, or prevent legitimate recovery back to `recovery_address`. | Forging an Ed25519 signature accepted by `SweepController::verify_sweep_auth`; bypassing the `require_auth` gate inside `EphemeralAccount::sweep`; nonce-replay that reuses a previously valid signature. |
| **High — State-corruption** | Attacker can mutate persistent state (status, expiry, recorded payments, admin addresses, base reserve value) without authorization, but cannot extract funds directly. | Calling `EphemeralAccount::initialize` on an account whose `initialize` was supposed to be one-shot (returning `AlreadyInitialized` when it should); manipulating `DataKey::Admin` / `DataKey::BaseReserve` in `reserve_contract`; injecting a `record_payment` that an auditor cannot reconcile. |
| **Medium — Availability / DoS** | Attacker can block legitimate flow (sweep, expire, recovery, admin update) by repeatedly invoking an operation that traps, panics, or front-runs a target transaction. | Submitting a low-fee `expire()` to grief a just-paid ephemeral account; submitting an `execute_sweep` with a signature that traps the host function (Soroban host-function failures abort the transaction — see [`docs/security.md`](../../docs/security.md) § Reentrancy Protection), preventing the legitimate signed sweep from landing. |
| **Low — Information / minor invariant** | Attacker can read internal state they should not have access to, or break an internal invariant that does not directly translate to fund loss, state corruption, or DoS. | Off-chain event content disclosing an address that the reporter believes should be redacted; a read returning `Some(…)` where the design called for `None` first. |

### Severity escalation rules

- **Any** Critical finding **must** be triaged by at least two maintainers within 4 hours of acknowledgement.
- **Any** finding that affects signed funds movement (signatures, nonces, `SweepController::claim`) defaults to **Critical** unless and until proven otherwise.
- A report that reads as Medium but enables chained exploitation to Critical is itself Critical.

### Reference: how the real auth path looks

Before classifying any auth-related report, re-read [`docs/SIGNATURE_FORMAT.md`](../../docs/SIGNATURE_FORMAT.md) and [`docs/architecture.md`](../../docs/architecture.md) § `SweepController`. Important subtlety, frequently missed by first-time reporters:

- Real Ed25519 verification happens **only** in `SweepController::verify_sweep_auth`.
- `EphemeralAccount::sweep` accepts an `auth_signature` parameter but ignores it — authorization there is delegated entirely to `authorized_controller.require_auth()`. So a report that says "calling `EphemeralAccount::sweep` directly without a valid signature succeeds" is **expected behaviour**, not a vulnerability. Re-read [`contracts/ephemeral_account/src/lib.rs`](../../contracts/ephemeral_account/src/lib.rs) `verify_sweep_authorization` and the architecture doc before concluding.

---

## 3. Reproduction — which contracts and which Error enums to check

When reproducing a reported issue, work the contracts top-down. Only the four deployable crates under `contracts/` carry their own `Error` enum.

| Contract | Error file | Variants (with discriminants) |
|---|---|---|
| `ephemeral_account` | [`contracts/ephemeral_account/src/errors.rs`](../../contracts/ephemeral_account/src/errors.rs) | `AlreadyInitialized=1`, `NotInitialized=2`, `PaymentAlreadyReceived=3`, `InvalidAmount=4`, `InvalidExpiry=5`, `NotExpired=6`, `AlreadySwept=7`, `Unauthorized=8`, `InvalidSignature=9`, `NoPaymentReceived=10`, `AccountExpired=11`, `InvalidStatus=12`, `DuplicateAsset=13`, `TooManyPayments=14`, `NotUpgradeAdmin=15`. |
| `sweep_controller` | [`contracts/sweep_controller/src/errors.rs`](../../contracts/sweep_controller/src/errors.rs) | `InvalidAccount=1`, `TransferFailed=2`, `AuthorizationFailed=3`, `InsufficientBalance=4`, `AccountNotReady=5`, `AccountExpired=6`, `AccountAlreadySwept=7`, `InvalidSignature=8`, `SignatureVerificationFailed=9`, `AuthorizedSignerNotSet=10`, `InvalidNonce=11`, `UnauthorizedDestination=13` (discriminant `12` is intentionally skipped — leave it alone in patch notes). |
| `reserve_contract` | [`contracts/reserve_contract/src/errors.rs`](../../contracts/reserve_contract/src/errors.rs) | `InvalidAmount=1`, `ReserveNotSet=2`, `Unauthorized=3`, `AlreadyInitialized=4`, `NotInitialized=5`, `AmountTooLarge=6`. |
| `account_factory` | (no `Error` enum — returns `success: bool, error: None` on per-account failure). | Only structural errors propagate from `try_initialize`; the factory itself doesn't surface them. See Known gap in [`docs/architecture.md`](../../docs/architecture.md). |

A reproducible report should name:

- The exact entry function (e.g. `EphemeralAccount::sweep`, `SweepController::claim`, `AccountFactory::batch_initialize`).
- The input parameters that trigger the bug, in the form an SDK would supply them.
- The on-chain Error variant returned, if any, **including its discriminant** (e.g. `UnauthorizedDestination (13)` is distinct from `Unauthorized (8)` even though both mention auth).
- The expected Error variant per the contract's documented behaviour.
- A reference environment: contract ID, ledger at the moment of the call, network.

If the reporter can supply a transaction hash from a public network (testnet is the most common), pin the reproduction to that exact invocation — do not re-author a transaction with a fresh signature, since nonces advance on every successful sweep.

---

## 4. Containment (for Critical and High only)

For **Critical** findings, before any patch is written:

1. **Notify the deployer / signers.** If `SweepController`'s `authorized_signer` key is implicated, follow [`ephemeral-account-incident.md`](./ephemeral-account-incident.md) / [`reserve-contract-admin-key-rotation.md`](./reserve-contract-admin-key-rotation.md) where applicable and rotate the affected authority.
2. **Pause integrators.** If a finding concerns `EphemeralAccount::sweep` or `SweepController`, ask SDK operators to:
   - Stop creating new ephemeral accounts if the finding affects initialization.
   - Hold any pending signed sweep messages (their nonces will become invalid; do **not** rebroadcast stale ones).
3. **Consider a contract upgrade** through `EphemeralAccount::upgrade(new_wasm_hash)` for the affected contract. Cross-reference [./upgrade-admin-authority.md](./upgrade-admin-authority.md).
4. **Do not** push a fix to `main` until a working reproduction exists *and* a regression test fails on `main` and passes on the proposed fix.

For **High** state-corruption findings, the same containment rules apply scaled to the affected state. Medium (DoS) and Low findings can wait for the standard patch flow.

---

## 5. Disclosure and post-mortem

1. **Coordinated disclosure.** Honour the agreed embargo with the reporter. If the embargo lapses without a fix, release a public advisory with the known impact and the recommended mitigations.
2. **CVE / GHSA.** If the finding warrants, request a CVE via the reporter's preferred channel and link it from the post-mortem.
3. **Post-mortem.** Required for any **Critical** finding, recommended for **High**. The post-mortem should:
   - Reference the canonical technical write-up (linked from the future `SECURITY.md` advisory index).
   - List each contract and error variant implicated.
   - Identify the test that would have caught it (`contracts/<crate>/src/test.rs` or `contracts/<crate>/tests/integration.rs`), and add a regression test if one does not exist.
   - Cross-link the runbook used during the incident from this runbook's [Cross-references](#8-cross-references).
4. **Patch and advisory go out together.** Do not merge a Critical patch in silence; pair the merge with the public advisory at or after the embargo.

---

## 6. Auditability and records

For every report:

- Record the tracker ID, severity, contracts implicated, and Error enum discriminants involved.
- Save the reproduction (tx hash, inputs, observed output) alongside the tracker entry.
- Record the public advisory URL once published.

The Bridgelet audit index (`bridgelet-audit/`) — once the surrounding meta issues are closed — is the durable home for these records. Do **not** store disclosure details in this runbook file; updates here are for the *process*, not for past incidents.

---

## 7. Where this runbook sits in the project

- This file lives at `bridgelet-audit/runbooks/security-disclosure-triage.md`.
- It is referenced from the (future) `bridgelet-audit/SECURITY.md` once that file is introduced in a later issue. **Creating that `SECURITY.md` is out of scope for this runbook.**
- It complements [`docs/security.md`](../../docs/security.md) (the architectural security model) and [`docs/reentrancy-analysis.md`](../../docs/reentrancy-analysis.md) (the deep dive on Soroban's reentrancy properties).

---

## 8. Cross-references

- [`reserve-contract-admin-key-rotation.md`](./reserve-contract-admin-key-rotation.md) — when triage identifies a compromised `ReserveContract` admin secret.
- [`upgrade-admin-authority.md`](./upgrade-admin-authority.md) — broader context on contract WASM upgrades.
- [`ephemeral-account-incident.md`](./ephemeral-account-incident.md) — incident playbook specific to `EphemeralAccount` (when present).
- [`docs/security.md`](../../docs/security.md) — the architectural security model and threat model.
- [`docs/SIGNATURE_FORMAT.md`](../../docs/SIGNATURE_FORMAT.md) — the canonical sweep-authorization message format.
- [`docs/architecture.md`](../../docs/architecture.md) — overall system architecture and known security-related limitations.
- [`docs/reentrancy-analysis.md`](../../docs/reentrancy-analysis.md) — Soroban execution model + reentrancy argument.
- [`docs/api-reference.md`](../../docs/api-reference.md) — full function and error-code reference for all three main contracts.
