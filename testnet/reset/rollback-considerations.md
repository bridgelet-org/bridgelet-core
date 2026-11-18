# Testnet Rollback Considerations

## Purpose

"Rollback" means two different things on this shared testnet deployment, and
choosing the wrong one makes an incident worse:

- **In-place rollback**: call the admin-gated `upgrade()` again with a previous
  WASM hash. The contract ID and its stored state remain; only the code pointer
  changes.
- **Fresh redeploy**: run the deploy script again and re-initialise. New
  contract IDs are minted, all old state is gone, and the controller nonce
  restarts. This is a **replacement**, not a rollback of state.

This document compares the two, states their preconditions and limits, and
says what must be written down afterwards. It is decision guidance, not a
record that any rollback has occurred.

> The canonical state is
> [`testnet/registry/contract-status.md`](../registry/contract-status.md):
> all four contracts are recorded as undeployed. Nothing here asserts that a
> target contract or a previous hash is live.

---

## Decision table

| Situation | In-place re-upgrade | Fresh redeploy | IDs and state | Notes |
|---|---|---|---|---|
| A recent in-place upgrade introduced a regression on one `EphemeralAccount` instance; admin key and a known-good hash are available | **Preferred** | No | ID unchanged; instance storage preserved | Requires a functioning admin-gated `upgrade()` in the currently deployed code. |
| A behavior change with an identical interface needs to be undone across several instances | Possible, **per instance** | No | Each upgrade affects only the selected instance | A factory template or other instances are not updated automatically; see [`upgrade-verification.md`](../runbooks/upgrade-verification.md). |
| Constructor or configuration must change: signer, destination, admin, reserve base, or expiry inputs | No | **Required** | New IDs; old state, payments, and nonces gone | `initialize()` values are fixed at deployment; a code swap does not re-run them. |
| An SDF testnet reset removed the contracts | No | **Only option** | New IDs on a new network epoch | There is no contract left to upgrade; see [`reset-policy.md`](../policies/reset-policy.md). |
| The previous WASM hash is unknown, `TBD`, or its WASM is not available on testnet | No | Only if a new deployment is acceptable | Depends on path chosen | Do not guess a hash or assume a local rebuild reproduces it. |
| The new code removed or broke `upgrade()`, or changed/lost the stored admin | No | **Required**, if any | New IDs | No in-contract path back exists. |
| Stored state is incompatible with the previous WASM's layout or enum ordering | **Do not** | Only if losing state is acceptable | Re-upgrade risks misreading state; redeploy discards it | An interface match does not prove storage compatibility. |
| The rollback would reintroduce a critical security flaw | No, without explicit decision | No, without explicit decision | - | Treat as an incident decision, not a routine revert; see [`breaking-change-notice.md`](../policies/breaking-change-notice.md). |

An in-place upgrade and a redeploy are recorded differently. Only the former
is an upgrade-history event; only the latter creates a new entry in
[`historical-deployments.md`](historical-deployments.md).

---

## Where the previous hash comes from

The intended source for known-good hashes is
[`testnet/registry/wasm-hash-reference.md`](../registry/wasm-hash-reference.md),
cross-checked with
[`testnet/registry/upgrade-history.md`](../registry/upgrade-history.md). Today
that reference records `TBD` for every contract, so it **cannot supply a
rollback hash**.

The hash `5e667ea0687341bdccc81538492143b573777dd04b2450cdeb89b02cee62c58e`
appears in `testnet/docs/changelog.md`, `testnet/docs/network-config.md`, and
`deployments/testnet.json` as an EphemeralAccount hash. Those are unverified
historical records (see [`historical-deployments.md`](historical-deployments.md)),
not proof that the WASM was installed on testnet or that the hash is
known-good. Before using any hash:

- [ ] Confirm it is 64 lowercase hexadecimal characters.
- [ ] Confirm the WASM is installed on the **same** testnet network.
- [ ] Confirm where it was recorded and whether a verification was performed.
- [ ] If rebuilding from a commit, compute the resulting hash and record that
      value. Do not assert it equals a historical hash without comparing them.
- [ ] Record the hash source in the incident note and in
      `upgrade-history.md`; write `UNKNOWN` if it cannot be established.

---

## Preconditions for an in-place re-upgrade

The checked-in `upgrade()` is admin-gated on `EphemeralAccount`. Do not assume
`SweepController`, `ReserveContract`, or `AccountFactory` expose the same
method. There is no multi-sig or timelock in the current MVP, so the single
admin key is the whole control.

- [ ] The target contract ID and network passphrase are verified, and `get_info`
      proves the instance is initialized.
- [ ] The stored admin address is known from deployment/initialization evidence
      and the available identity matches it.
- [ ] The previous known-good hash is established per the section above and its
      WASM is available on testnet.
- [ ] A pre-change state snapshot exists: `get_info`, `get_status`, reserve
      reads, current ledger, and UTC time.
- [ ] The current and candidate interfaces and error-code schemes were
      compared; any difference is an intentional, announced change.
- [ ] Storage keys and persisted enum orderings were reviewed against the
      snapshot. `AccountStatus` values `0`-`3` are persisted, so reordering
      them silently changes their meaning.
- [ ] The currently deployed code still exposes a working admin-gated
      `upgrade()`.
- [ ] The operation has separate approval. Do not upgrade as an investigation
      step; see [`reset-policy.md`](../policies/reset-policy.md).
- [ ] A freeze is declared per [`pre-reset-freeze-notice.md`](pre-reset-freeze-notice.md),
      or the emergency exception is recorded retrospectively.
- [ ] Registry files do not still contain unresolved deployment placeholders
      that would make the target ambiguous.

Stop if any item fails. A re-upgrade on an unverified target is not a rollback.

---

## Preconditions for a fresh redeploy

- [ ] The trigger genuinely requires new IDs: a constructor/config change or
      an SDF reset. Do not redeploy to "undo" code when an in-place upgrade is
      available and safe.
- [ ] Acceptance that every stored ephemeral-account ID, payment record,
      expiry ledger, and terminal status is lost. See the destroyed-state table
      in [`reset-policy.md`](../policies/reset-policy.md).
- [ ] Deploy-time configuration is deliberately reproduced from
      `deployments/testnet.json` (`authorizedSigner`, `creatorAddress`,
      `reserveAdminAddress`, `recoveryAddress`, and the WASM hash). Copy public
      values only; never reconstruct or paste key material.
- [ ] The new version label and deployed commit are known, or are explicitly
      recorded as not assigned.
- [ ] The announcement, artifact updates, registry updates, and new generation
      record are part of the same change.

---

## Storage and state compatibility risks

| Risk | In-place re-upgrade | Fresh redeploy |
|---|---|---|
| Storage layout change | The host does not migrate storage. Old keys or types can be misread by the previous WASM; compare against the snapshot before proceeding. | No old state remains; nothing to misread. |
| Persisted enum or discriminant ordering | `AccountStatus` is stored and returned; renumbering changes the meaning of existing records without an error. | Status values are created fresh under the new build. |
| Interface and error codes | Calls can still reach the same ID but decode differently; clients and indexers break silently if events or codes changed. | Every consumer must switch IDs; the break is loud. |
| Scope | Only the selected instance changes. Other instances and a factory template keep their old code pointer. | All children of the old deployment are gone. |
| Upgrade evidence | `upgrade()` emits no contract event. The transaction, code hash, and registry log are the evidence. | Deployment artifacts and the changelog entry are the evidence. |
| Partial failure | A failed or unresolved transaction did not complete the upgrade; re-check by hash before retrying. | A half-run deploy leaves artifacts disagreeing; the freeze stays open until they agree. |

A code-hash match and an interface match are necessary but not sufficient.
The state-preservation reads in
[`testnet/runbooks/upgrade-verification.md`](../runbooks/upgrade-verification.md)
are the proof that the rollback did not corrupt what it was meant to save.

---

## Nonce and configuration implications

| Item | In-place re-upgrade | Fresh redeploy |
|---|---|---|
| `SweepController` nonce | Preserved in storage. Re-read `get_nonce`; do not reset a local cache because an upgrade occurred. | Restarts at `0` after `initialize()`; locally tracked nonces are meaningless. |
| Signature validity | Same controller ID, destination, nonce, and digest algorithm means a signature can remain valid. If the digest, encoding, or interface changed, re-sign. | Signatures include the controller ID; old signatures cannot be reused even if the nonce is also `0`. |
| `authorized_signer` / `authorized_destination` | Preserved in controller storage. | Re-supplied at deployment; a different signer makes existing signatures fail, which resembles a signing bug. |
| `EphemeralAccount` admin, controller, recovery address | Fixed at initialization; preserved. | Re-supplied at initialization; no rotation path exists on an existing instance. |
| Reserve admin and base reserve | Preserved in the selected instance. | Re-supplied; otherwise the new contract is initialised but unconfigured. |
| Indexers and local caches | Keep watching the same ID, but re-check events and error codes after the code swap. | Invalidate all ID-keyed caches and replay from the new generation's records. |

For either path, the authoritative nonce is `SweepController::get_nonce()`;
follow [`testnet/runbooks/nonce-desync.md`](../runbooks/nonce-desync.md) and
serialise sign-and-submit per controller.

---

## Verification

| Check | In-place rollback | Fresh redeploy |
|---|---|---|
| Transaction finality | Query the exact transaction hash; a pending or unknown result is not success. | Confirm the deploy transaction and the generated IDs. |
| Code pointer | Repeat the code-hash query; the result must equal the target hash exactly. | Confirm the installed hash against `wasm-hash-reference.md` and the upload output. |
| Interface | Inspect the deployed interface and compare with the intended release. | Confirm the initialized interface, including argument counts. |
| Behavior | Run the release-specific read-only probe from `upgrade-verification.md`. | Run the read-only checks in `verify-contract-live.md`. |
| State | Compare every field with the pre-change snapshot; unexplained change is a compatibility failure. | Confirm the new state is intentionally clean; do not expect old payments or status. |
| Nonce/config | Re-read `get_nonce` and confirm signer/destination are unchanged. | Expect `0`; re-supply and confirm configuration. |
| Evidence | Record the UTC time, ledger, transaction hash, hashes, and probe output. | Record the new IDs, version label, commit, and verification links. |

If the old hash cannot be established, record `UNKNOWN` and explain the gap.
Do not fill it with a placeholder or a copied historical value.

---

## Recording the outcome

| Path | Where it is recorded | What is written |
|---|---|---|
| In-place rollback | `testnet/registry/upgrade-history.md` | A **new** row with the rollback reason (for example, "Emergency rollback from `<version>`"), contract ID, old/new hashes, admin, and transaction hash. Never overwrite the row describing the failed upgrade. |
| In-place rollback | `testnet/registry/wasm-hash-reference.md`, `contract-status.md`, `last-verified.json` | Current hash, status, and a timestamp refreshed after verification. |
| Any rollback | `testnet/docs/changelog.md` | A dated entry with deployed commit, IDs (`unchanged` or new), breaking changes, other changes, and action required. An emergency entry published late is retrospective, not notice. |
| Fresh redeploy | `testnet/reset/historical-deployments.md` | A new generation with the previous generation's end date closed; all values marked unverified until the registry proves otherwise. |
| Fresh redeploy | `deployment-artifacts/contract-ids.txt`, `deployments/testnet.json`, `testnet/docs/network-config.md` | The new IDs and deploy-time configuration, in the same change as the changelog entry. |
| Both | `testnet/reset/communication-template.md` | The completed-reset follow-up announcement, using the template rather than a new ad-hoc message. |

---

## When rollback is not possible

| Case | Why | What remains |
|---|---|---|
| Previous hash is `TBD`, unknown, or its WASM is unavailable and not reproducible | There is no verified target for `upgrade()`. | Record `UNKNOWN`; redeploy only if a new deployment is acceptable. |
| New code removed or broke `upgrade()`, or admin authority is lost | No callable in-contract path back exists. | Fresh redeploy under new IDs. |
| Stored state is incompatible with the previous WASM | Re-upgrading can corrupt or misread the state it was meant to preserve. | Keep the failing version isolated; redeploy if state loss is acceptable. |
| SDF platform reset | Contract and state were destroyed at the network level. | Redeploy in the new epoch; no rollback of old IDs or payments. |
| A redeploy already replaced the IDs | Old contract addresses cannot be restored, and child accounts and payments are gone. | New generation record and a new announcement. |
| Reverting would reintroduce a critical vulnerability | The rollback itself is a security decision, not a routine revert. | Escalate per the incident process; do not silently revert. |
| No contract is actually deployed and verified | There is nothing to roll back. | Current canonical state: all four undeployed. Do not invent a target. |

---

## Related Documentation

- [`testnet/registry/upgrade-history.md`](../registry/upgrade-history.md) -
  where an in-place rollback is logged
- [`testnet/registry/wasm-hash-reference.md`](../registry/wasm-hash-reference.md)
  - intended source for known-good hashes; currently all `TBD`
- [`testnet/runbooks/upgrade-verification.md`](../runbooks/upgrade-verification.md)
  - the full upgrade and rollback verification procedure
- [`testnet/registry/contract-status.md`](../registry/contract-status.md) -
  canonical status, currently all undeployed
- [`testnet/reset/historical-deployments.md`](historical-deployments.md) - why
  historical hashes are not proof of a deployable target
- [`testnet/reset/pre-reset-freeze-notice.md`](pre-reset-freeze-notice.md) -
  banner and unfreeze rules for the operation
- [`testnet/reset/communication-template.md`](communication-template.md) - the
  announcement for either outcome
- [`testnet/policies/reset-policy.md`](../policies/reset-policy.md) - what a
  redeploy destroys and what must change in the same commit
- [`testnet/policies/breaking-change-notice.md`](../policies/breaking-change-notice.md)
  - notice obligations, including emergency exemptions
- [`testnet/runbooks/nonce-desync.md`](../runbooks/nonce-desync.md) - nonce and
  signature handling after either path

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-11-18 | Initial comparison of in-place re-upgrade rollback and fresh redeploy, with preconditions, compatibility risks, and recording rules |
