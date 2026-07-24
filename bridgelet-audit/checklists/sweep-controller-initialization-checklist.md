<!--
Purpose: Review checklist for correctly initializing a new SweepController deployment.
Owner: @JudeDaniel6 (closes #295 — bridgelet-audit/ knowledge-base initiative).
Audience: Operators deploying SweepController; reviewers signing off on go-live.
Status: Documentation-only. No contract changes are introduced by this file.
-->

# SweepController Initialization Checklist

> **Purpose.** Ensure every `SweepController` deployment goes live deliberately,
> with the destination mode, signer custody, and downstream account bindings
> chosen — and recorded — before any production traffic is routed through it.

| Field | Value |
| :--- | :--- |
| **Related issue** | [#295](https://github.com/bridgelet-org/bridgelet-core/issues/295) |
| **Owner / reviewer** | `_operator-name_` |
| **Deployment network** | `testnet` / `mainnet` |
| **Scheduled go-live (UTC)** | `_ISO-8601 timestamp_` |
| **Last reviewed** | `_ISO-8601 date_` |

## Table of Contents

1. [Pre-Deployment Inputs](#pre-deployment-inputs)
2. [Destination Mode: Locked vs Flexible](#destination-mode-locked-vs-flexible)
3. [Authorized-Signer Custody](#authorized-signer-custody)
4. [Authorized-Controller Mapping](#authorized-controller-mapping)
5. [Post-Deployment Verification](#post-deployment-verification)
6. [Sign-Off](#sign-off)
7. [Related Issues](#related-issues)

---

## Pre-Deployment Inputs

Before calling `SweepController::initialize(creator, authorized_signer, authorized_destination)`,
the operator must have the following on hand and recorded in the deployment ticket:

- [ ] **Creator address** — the contract-invoker that will authorize `initialize`. Must
      be a Stellar account the operator controls (typically a deployment multisig).
- [ ] **Authorized signer** — a `BytesN<32>` Ed25519 public key. The matching
      private key **must not** be present on the deployment host.
- [ ] **Authorized destination** — either a concrete `Some(Address)` (locked mode)
      or the literal `None` (flexible mode). The decision must be intentional
      and written down in the deployment ticket; defaulting to either is not
      acceptable.
- [ ] **Creator auth path** — confirmation that the creator account can actually
      satisfy `creator.require_auth()` from the operator's transaction-signer setup.

If any of the above is missing, **stop**. Do not call `initialize()`.

---

## Destination Mode: Locked vs Flexible

`SweepController` initializes with `authorized_destination: Option<Address>`.
This single argument determines the entire threat surface of the deployment and
**must be chosen deliberately, not by default**.

| Property | `Some(addr)` — Locked mode | `None` — Flexible mode |
| :--- | :--- | :--- |
| Authorized signer can change `destination` per sweep? | **No.** `validate_destination` rejects any destination that does not equal `authorized_destination`. | Yes. Any destination the signer authorizes is permitted. |
| Effect on `update_authorized_destination` | Become permanently read-only once the first sweep occurs (`nonce > 0` reverts with `AccountAlreadySwept`). | Same lock applies post-sweep. |
| Operational blast radius if the signer leaks | Funds can only exit to the single locked address. | Funds can be redirected to **any** address a malicious re-signing endpoint accepts. |
| Use when | Recipient is a known, audited hot/cold wallet with strict allow-listing. | Recipient can rotate, multi-recipient fan-outs are required, or signer is in a tightly-controlled HSM with strict replay protection. |

Reviewer must verify, in writing:

- [ ] **Locked vs flexible decision recorded** with an explicit business reason
      on the deployment ticket. The deployment is **blocked** if this field is
      left blank.
- [ ] **If locked**, the destination address has been confirmed against the
      intended recipient's known-good allow-list (not copied from a chat window,
      not committed to a branch that was never reviewed).
- [ ] **If flexible**, an explicit compensating control is documented — e.g.
      the signing endpoint sits inside an HSM and only signs against an
      allow-listed set of destinations, monitored by an off-chain policy gate.
- [ ] The opposite mode's risks have been considered and accepted (or
      mitigated). Reviewers should be able to articulate why the *other* mode
      was rejected.

> **Pitfall.** Choosing `None` to "decide later" leaves the deployment in a
> quietly permissive state. If the decision is genuinely pending, defer the
> deployment until it is made.

---

## Authorized-Signer Custody

The `authorized_signer` private key is the single cryptographic identity that
authorizes every `SweepController::execute_sweep` call. Compromise of this key
is equal to compromise of every account routed through this controller.

Before go-live, the operator must confirm the custody arrangement and record
the answer on the deployment ticket. **Do not deploy** without a documented
answer.

- [ ] **Custody location documented.** Where is the matching private key held?
      Acceptable answers include:
    - Hardware Security Module (HSM) with FIPS 140-2 Level 3 (or higher) certification.
    - Secure enclave on a hardened signing appliance.
    - A managed KMS (AWS KMS, GCP KMS, HashiCorp Vault with a HSM-backed
      unseal) where access is gated by an audited IAM policy.
- [ ] **Custody is NOT:**
    - On the deployment host's disk or in a developer's `.env` file.
    - In a soft wallet held by a single individual.
    - In plaintext in a CI secret store that lacks audit logging.
- [ ] **Key-rotation plan documented.** How is the signer rotated? The current
      `initialize()` does not expose a rotation hook, so rotation requires
      deploying a new contract instance with a new signer and migrating
      references. Confirm that whoever owns the controller bindings knows this.
- [ ] **Access policy reviewed.** Who can sign sweep authorizations and why?
      Cross-check against the "authorized_controller mapping" (§4).
- [ ] **Compromise runbook points to documented response.** A separate
      incident-response runbook must exist that names the contacts and the
      cutover plan if the signer is suspected to be leaked. (Reference to
      `bridgelet-audit/runbooks/` or an internal IR folder.)

> **Pitfall.** "It's in our team's password manager" is not a custody
> arrangement. It is an incident waiting to happen.

---

## Authorized-Controller Mapping

`EphemeralAccount::initialize(... authorized_controller)` records the controller
that is trusted to call `sweep()` on that account. If an `EphemeralAccount` is
pointed at the wrong `SweepController`, signed authorizations issued by the
intended controller will silently be rejected, and (worse) authorization
requests issued by an unintended controller for unrelated funds may succeed.

Before go-live, the operator must enumerate which account-creation paths will
use this **specific** controller and confirm binding correctness.

- [ ] **Issuing path enumerated.** List every code path or operator script
      that creates an `EphemeralAccount` and is intended to use this controller
      as `authorized_controller`. Typical paths include:
    - Direct `EphemeralAccount::initialize` calls from an SDK.
    - `AccountFactory::batch_initialize` invocations that pass this
      controller's address as `authorized_controller`. (See
      [`validate-batch-initialize-salt-uniqueness.md`](../runbooks/validate-batch-initialize-salt-uniqueness.md)
      for the related salt-uniqueness pre-flight that should run alongside
      this step.)
- [ ] **Binding sample verified.** From a test account created through the
      intended issuing path, call `get_info()` and confirm the returned
      `authorized_controller` matches the deployed `SweepController` instance
      address byte-for-byte.
- [ ] **No stale references retained.** Any previous controllers still
      referenced by production automation are documented and either (a) retired
      in favour of this one or (b) explicitly retained for intentional migration
      — not silently left behind.
- [ ] **Capture in the deployment ticket.** Paste the binding-evidence
      excerpt (e.g. JSON output from `get_info()` plus the deployed controller
      contract address) into the ticket.

> **Pitfall.** A reviewer signing off on this checklist while the binding
> has not yet been exercised end-to-end is signing off on a guess. The
> `get_info()` evidence is what closes this section.

---

## Post-Deployment Verification

After `initialize()` succeeds and before any production traffic is routed:

- [ ] `SweepController::get_authorized_signer()` returns the expected `BytesN<32>`.
- [ ] `SweepController::get_authorized_destination()` matches the chosen mode
      (`Some(addr)` for locked, `None` for flexible).
- [ ] `SweepController::get_sweep_nonce()` returns `0` (initialized but not yet
      swept).
- [ ] A `dest_auth` event was emitted **only if** the controller was placed
      into locked mode. If `dest_auth` is absent in locked mode, or present in
      flexible mode, halt and re-investigate.
- [ ] A smoke-test sweep against a freshly created `EphemeralAccount` succeeds
      end-to-end and emits a `SweepCompleted` event.

---

## Sign-Off

Two signatures are required to mark this checklist complete:

| Role | Name | Signature / commit SHA | Date (UTC) |
| :--- | :--- | :--- | :--- |
| Operator |  |  |  |
| Reviewer  |  |  |  |

A deployment must not advance past the `initialize()` call without both rows
populated.

---

## Related Issues

- **#288 — Verifying nonce state before trusting `update_authorized_destination`'s lock.**
  See [`verify-claim-vs-execute-sweep-nonce-state.md`](../runbooks/verify-claim-vs-execute-sweep-nonce-state.md)
  for the runbook that should be revisited whenever an operator is tempted to
  rotate the locked destination after go-live.
- **#290 — Pre-flight validation before calling `batch_initialize` in production.**
  See [`validate-batch-initialize-salt-uniqueness.md`](../runbooks/validate-batch-initialize-salt-uniqueness.md)
  for the companion runbook that should be executed alongside §4 above when
  the issuing path is `AccountFactory::batch_initialize`.

See also [`bridgelet-audit/README.md`](../README.md) for the folder index.
