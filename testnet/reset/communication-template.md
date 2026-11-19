# Testnet Reset Communication Template & Post-Reset Checklist

## Purpose

This file holds the ready-to-use messages for a planned or completed Bridgelet
testnet reset, redeploy, or in-place upgrade. Copy a template into a dated
entry in [`testnet/docs/changelog.md`](../docs/changelog.md); the changelog is
the single notification channel described in
[`testnet/policies/reset-policy.md`](../policies/reset-policy.md) and
[`testnet/policies/breaking-change-notice.md`](../policies/breaking-change-notice.md).

Notice periods and exemptions come from those policies. This file supplies
the wording only; it does not create a new channel, a promise of lead time, or
a claim that a contract is live.

> **Verification first.** The canonical
> [`testnet/registry/contract-status.md`](../registry/contract-status.md)
> currently records all four contracts as undeployed. IDs in
> `deployment-artifacts/contract-ids.txt`, `deployments/testnet.json`, and
> older changelog entries are unverified historical records; see
> [`testnet/reset/historical-deployments.md`](historical-deployments.md). Do
> not present one as a live deployment in an announcement.

The former JSON pointer in this template was removed: that path does not exist
in this repository. Use
[`deployment-artifacts/contract-ids.txt`](../../deployment-artifacts/contract-ids.txt)
and [`deployments/testnet.json`](../../deployments/testnet.json) for IDs, and
[`testnet/registry/wasm-hash-reference.md`](../registry/wasm-hash-reference.md)
for hashes.

---

## Before publishing

| Field | Take it from | Rule |
|---|---|---|
| What changed | The change description and the deployed diff | One or two factual sentences; name the event as reset, redeploy, or in-place upgrade. |
| Version label | [`testnet/policies/versioning-scheme.md`](../policies/versioning-scheme.md) | `testnet-v1.x` or `testnet-v2.x` where assigned; otherwise write `not assigned`. |
| Deployed commit | The dated changelog entry | Copy the SHA exactly; write `not yet built` in an upcoming notice if unknown. |
| New contract IDs | `deployment-artifacts/contract-ids.txt` and `deployments/testnet.json` | Copy all four. Use `unchanged`, `TBD`, or `not deployed` where that is the actual value; never guess or reuse a historical ID. |
| WASM hashes | `testnet/registry/wasm-hash-reference.md` and the upload output | 64 lowercase hex characters, or `TBD`. Do not present a placeholder as a deployed hash. |
| Breaking changes | The interface/event/error diff | State "none" or give old and new signatures verbatim; see the worked example in `breaking-change-notice.md`. |
| Action required | The integrator migration | A specific edit, or "none" - not a description of a symptom. |
| Verification links | `verify-contract-live.md`, `explorer-links.md`, and the transaction hash | Use exact hashes and paths; adjust relative links if the message is copied outside `testnet/docs/`. |
| Status | Canonical registry plus read-only checks | "verified deployed" only after checks pass; otherwise "unverified - registry records undeployed". |

---

## Upcoming reset or redeploy announcement

```markdown
## YYYY-MM-DD - Upcoming testnet <reset | redeploy | in-place upgrade>

Planned for: <YYYY-MM-DD HH:MM UTC> (or "date not yet set - SDF-initiated")
Version label: <testnet-v1.x | testnet-v2.x | not assigned>
Deployed commit: <SHA | not yet built>
Freeze: <not started | declared at <UTC TIMESTAMP>>

**What changed:** <what is being deployed or reset, and why>

**Contract IDs:** <unchanged | new - table below>

| Contract | Previously recorded ID | New ID | Verification |
|---|---|---|---|
| EphemeralAccount | <ID or TBD> | <new ID, unchanged, or TBD> | <unverified until checked> |
| SweepController | <ID or TBD> | <new ID, unchanged, or TBD> | <unverified until checked> |
| ReserveContract | <ID or N/A> | <new ID, unchanged, or TBD> | <unverified until checked> |
| AccountFactory | <ID or N/A> | <new ID, unchanged, or TBD> | <unverified until checked> |

**WASM hashes:** <from wasm-hash-reference.md / upload output; TBD if unknown>

| Contract | Previous hash | New hash |
|---|---|---|
| EphemeralAccount | <hash or TBD> | <hash or TBD> |
| SweepController | <hash or TBD> | <hash or TBD> |
| ReserveContract | <hash or N/A> | <hash or TBD> |
| AccountFactory | <hash or N/A> | <hash or TBD> |

**Breaking changes:** <none | old signature -> new signature; event or error-code changes>
**Action required:** <specific edit for integrators, or "none">
**Verification links:** [status registry](../registry/contract-status.md), [live-check procedure](../registry/verify-contract-live.md), [explorer links](../monitoring/explorer-links.md), <transaction hash after submission>
```

For an in-place `upgrade()`, keep the "New ID" column as `unchanged` and
describe the hash change; the ID change itself is not the breaking event. For
an SDF-initiated reset, write `date not yet set` and say in "What changed" that
the project learned of it late - that is an exemption, not notice.

---

## Completed reset or redeploy follow-up

```markdown
## YYYY-MM-DD - Testnet <reset | redeploy | in-place upgrade> complete

Completed at: <YYYY-MM-DD HH:MM UTC>
Version label: <testnet-v1.x | testnet-v2.x | not assigned>
Deployed commit: <SHA>
Status: <verified deployed | unverified - canonical registry records undeployed>

**What changed:** <what actually shipped, including any difference from the announcement>

**Contract IDs:** <unchanged | new - final values from deployment-artifacts/contract-ids.txt and deployments/testnet.json>

| Contract | Final ID | Verification |
|---|---|---|
| EphemeralAccount | <ID or TBD> | <check performed and UTC time> |
| SweepController | <ID or TBD> | <check performed and UTC time> |
| ReserveContract | <ID or TBD> | <check performed and UTC time> |
| AccountFactory | <ID or TBD> | <check performed and UTC time> |

**WASM hashes:** <final values from wasm-hash-reference.md; TBD if not recorded>

| Contract | Final hash |
|---|---|
| EphemeralAccount | <hash or TBD> |
| SweepController | <hash or TBD> |
| ReserveContract | <hash or TBD> |
| AccountFactory | <hash or TBD> |

**Breaking changes:** <none | old signature -> new signature>
**Action required:** <specific edit for integrators, or "none">
**Verification:** [status registry](../registry/contract-status.md), [live-check procedure](../registry/verify-contract-live.md), <exact transaction hash>, <ledger and UTC of verification>
**Rollback:** <none | re-upgrade or redeploy performed; see rollback-considerations.md and, for an upgrade, upgrade-history.md>
```

If verification did not pass, say so in the first paragraph and keep `Status:
unverified`. A follow-up that lists IDs as live without checks is worse than one
that reports failure.

---

## Communication rules

| Rule | Detail |
|---|---|
| One channel | Publish in `testnet/docs/changelog.md`. Do not treat another channel as authoritative. |
| Timing | Use the notice tiers in `breaking-change-notice.md`. Exempt events get the maximum possible notice and a prominent retrospective entry. |
| Pairing | A completed follow-up is required after an upcoming notice; if the operation is cancelled, publish a short cancellation line instead. |
| Consistency | The IDs and hashes in the message must match `deployment-artifacts/contract-ids.txt`, `deployments/testnet.json`, and `wasm-hash-reference.md` at the time of publication. |
| Honesty | Mark unverified records as unverified; never claim liveness from a file alone. |
| Freeze | For planned changes, the banner in `pre-reset-freeze-notice.md` is added before the change and removed only after the post-reset checklist below is complete. |
| Secrets | No seed phrases, secret keys, signatures, or environment files in an announcement; see `testnet/policies/data-retention.md`. |

---

## Post-reset verification checklist

- [ ] All deployed contract WASM hashes match the release artifacts and are
      recorded in `testnet/registry/wasm-hash-reference.md`; `TBD` remains
      `TBD` where nothing was verified.
- [ ] Every ID now claimed live was resolved with
      `testnet/registry/verify-contract-live.md`, with UTC time and ledger
      recorded.
- [ ] `deployment-artifacts/contract-ids.txt` and `deployments/testnet.json`
      agree with each other and with the changelog entry.
- [ ] `testnet/registry/contract-status.md` matches the evidence: verified
      deployed values, or undeployed / `TBD` as it does today.
- [ ] `testnet/registry/last-verified.json` was refreshed **after** the checks.
- [ ] Update contract IDs in SDK documentation, examples, and test suites;
      remove hard-coded IDs where possible.
- [ ] For an in-place upgrade, append the upgrade (or rollback) to
      `testnet/registry/upgrade-history.md` with old and new hashes and the
      transaction hash; do not describe a redeploy as an upgrade.
- [ ] If a redeploy or reset changed contract IDs, add a generation to
      `testnet/reset/historical-deployments.md` and close the previous
      generation's date range. A hash-only in-place upgrade does not create a
      generation; record it in `testnet/registry/upgrade-history.md` and
      `testnet/registry/wasm-hash-reference.md`.
- [ ] Re-read `get_nonce` from `SweepController` and apply
      `testnet/runbooks/nonce-desync.md`: invalidate caches for a new ID, do
      not reset a cache merely because an upgrade occurred.
- [ ] Confirm Friendbot funding still operates cleanly for fresh test
      accounts, using `testnet/faucet/README.md`.
- [ ] Publish the completed-reset follow-up in `testnet/docs/changelog.md`.
- [ ] Remove the in-progress banner from `testnet/registry/contract-status.md`
      only after every applicable item above is complete.

---

## Related Documentation

- [`testnet/docs/changelog.md`](../docs/changelog.md) - the notification
  channel where these templates are used
- [`testnet/policies/breaking-change-notice.md`](../policies/breaking-change-notice.md)
  - notice periods, required fields, and exemptions
- [`testnet/policies/reset-policy.md`](../policies/reset-policy.md) - reset
  types, authority, and same-change artefacts
- [`testnet/reset/pre-reset-freeze-notice.md`](pre-reset-freeze-notice.md) -
  banner text and unfreeze criteria
- [`testnet/reset/historical-deployments.md`](historical-deployments.md) -
  generation labels and unverified historical IDs
- [`testnet/reset/rollback-considerations.md`](rollback-considerations.md) -
  how to describe a reversal
- [`testnet/registry/contract-status.md`](../registry/contract-status.md) -
  canonical status
- [`testnet/registry/verify-contract-live.md`](../registry/verify-contract-live.md)
  - verification procedure
- [`testnet/monitoring/explorer-links.md`](../monitoring/explorer-links.md) -
  copy-ready explorer routes and their verification rules
- [`deployment-artifacts/contract-ids.txt`](../../deployment-artifacts/contract-ids.txt)
  and [`deployments/testnet.json`](../../deployments/testnet.json) - the real
  ID sources

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-11-18 | Replaced the thin stub with upcoming and completed templates, corrected artefact paths, and an unchecked post-reset checklist |
