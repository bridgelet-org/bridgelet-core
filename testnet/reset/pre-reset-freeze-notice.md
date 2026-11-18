# Pre-Reset Freeze Convention

## Purpose

A **short, local convention** for planned Bridgelet testnet resets and
redeploys: from the moment the operation starts until the post-reset checklist
is complete, the shared status registry carries an in-progress banner. The
banner stops anyone from treating half-updated IDs as current.

This is a convention, not an authorisation policy and not a notice promise.
It creates no new role and extends no existing one.

## Status and scope

| Question | Position |
|---|---|
| What triggers a freeze? | A **planned** Bridgelet-controlled change that alters shared IDs, WASM hashes, or deploy-time configuration. See [Trigger](#trigger). |
| Who declares it? | **Authority is undecided.** In practice, the role acting as deployer/operator - whoever runs the operation and holds the relevant key - declares it. This is a description of who can act, not a grant of authority. |
| When does it start? | When the banner is added to `testnet/registry/contract-status.md`, before the first state-changing step. |
| When does it end? | When every [unfreeze criterion](#unfreeze-criteria) is met and the banner is removed. |
| How long is it? | As short as the operation allows. No fixed duration, SLA, or notice period is created here. |
| What about SDF resets? | No advance declaration is possible. Add a retrospective banner as soon as the reset is detected. |
| Does the banner mean contracts are live? | No. The canonical registry still decides status; today it records all four contracts as undeployed. |

The ownership roles named in
[`testnet/docs/contact-and-ownership.md`](../docs/contact-and-ownership.md)
are escalation contacts, not a deployment-authorisation decision. The
authority question remains open as recorded in
[`testnet/policies/reset-policy.md`](../policies/reset-policy.md).

---

## Trigger

| Event | Freeze? | Reason |
|---|---|---|
| Full redeploy via `scripts/deploy-testnet.sh` | **Yes** | All four IDs may change at once; artifacts are rewritten. |
| In-place `upgrade()` to a new WASM hash | **Yes** | ID is unchanged, but behavior and recorded hash change; integrators need an unambiguous record. |
| Re-initialisation that changes signer, destination, expiry, or reserve configuration | **Yes** | Deploy-time configuration is shared and cached by integrators. |
| Source-only change, documentation edit, or unreleased changelog entry | **No** | Nothing on the shared deployment changes. |
| SDF testnet reset | **Retrospective only** | The project does not control or get advance warning of it. |
| Emergency security fix | **Yes, at the earliest safe moment** | Do not delay a safety fix for paperwork; record the freeze and the changelog entry retrospectively, per the exemption in `breaking-change-notice.md`. |

If the operation is cancelled before any state-changing step, remove the
banner and record one line in the changelog saying the planned reset did not
happen. Do not leave a freeze open with no operation underway.

---

## Who declares and who lifts it

The same **role acting as deployer/operator** adds the banner, performs the
operation, and removes the banner. In practice this is whoever can run the
deploy or admin `upgrade()` call; no document names that person and this file
does not do so either.

If that role is unavailable, do not have another party "take over" the banner
as an authorisation gesture. Leave the banner in place, record the blocked
state in the changelog, and wait for the deployer/operator. A banner removed
by someone who did not perform or verify the change is worse than a stale one.

---

## Allowed and forbidden edits during the freeze

| Edit | Allowed? | Notes |
|---|---|---|
| Add or update the in-progress banner in `testnet/registry/contract-status.md` | **Yes** | This is the act that starts the freeze. |
| Publish the dated announcement in `testnet/docs/changelog.md` using `testnet/reset/communication-template.md` | **Yes** | Required before the change where notice is owed; retrospective only for exempt events. |
| Rewrite `deployment-artifacts/contract-ids.txt` and `deployments/testnet.json` with the operation's results | **Yes** | Copy values from the deploy output. Keep key material out of both files and out of this archive. |
| Record verified IDs, hashes, and timestamps in the registry files | **Yes** | Only after the relevant read-only checks have run. |
| Add a new generation to `testnet/reset/historical-deployments.md` | **Yes** | Only when a redeploy or reset changes IDs; close the previous generation's date range in the same edit. |
| Record an in-place upgrade in `testnet/registry/upgrade-history.md` and the new hash in `testnet/registry/wasm-hash-reference.md` | **Yes** | A hash-only upgrade is an upgrade event, not a new deployment generation. |
| Remove the banner once the checklist is complete | **Yes** | By the deployer/operator only. |
| Unrelated edits to status files, artifact files, or announcements | **No** | Do not use an open freeze as cover for unrelated changes. |
| Changing contract source or deploy scripts as part of the freeze | **No** | Source and script work belongs in its own reviewed change, before the freeze. |
| Writing a guessed or copied-from-another-generation ID or hash | **No** | Mark it `TBD` / `not recorded` instead. See `historical-deployments.md`. |
| Marking a contract live before verification, or presenting an unverified historical ID as current | **No** | The canonical registry records undeployed until evidence says otherwise. |
| Removing the banner early, or publishing new IDs through an unmonitored second channel as the authoritative record | **No** | The changelog is the single notification channel. |
| Pasting secrets, seeds, signatures, or environment files into changelog entries or status notes | **No** | See `testnet/policies/data-retention.md`. |

---

## Banner text

Place this at the top of
[`testnet/registry/contract-status.md`](../registry/contract-status.md),
above the Overview section:

```markdown
> **RESET IN PROGRESS - DO NOT TRUST CONTRACT IDs IN THIS TABLE**
>
> A planned Bridgelet testnet reset/redeploy started at `<UTC TIMESTAMP>`.
> Contract IDs and WASM hashes in this registry may be stale, replaced, or
> not yet deployed. Do not treat any ID here as live until this banner is
> removed. See `testnet/docs/changelog.md` for the announcement and
> `testnet/reset/pre-reset-freeze-notice.md` for the unfreeze checklist.
```

For an SDF-initiated reset discovered after the fact, replace the first sentence
with:

```markdown
> A Bridgelet testnet reset was **detected** at `<UTC TIMESTAMP>`; no advance
> declaration was possible.
```

Replace `<UTC TIMESTAMP>` with an actual UTC value; do not leave the literal
placeholder in the file. The banner is removed only through the
[unfreeze criteria](#unfreeze-criteria) below.

---

## Communication

| Moment | What is published | Where |
|---|---|---|
| Before a planned change | Dated announcement with what changed, version label, IDs, hashes, breaking changes, and action required | `testnet/docs/changelog.md`, using [`communication-template.md`](communication-template.md) |
| At freeze start | Banner added; announce the start if the announcement's planned time has arrived | `testnet/registry/contract-status.md`; `testnet/docs/changelog.md` |
| During the operation | No new channel is promised. Escalation uses the contacts documented in `contact-and-ownership.md`. | - |
| At completion | Dated follow-up with final IDs, hashes, verification evidence, and remaining action required | `testnet/docs/changelog.md`, using the completed-reset template |
| Exempt event | Maximum possible notice; retrospective entry when notice could not be given | `testnet/docs/changelog.md`, plus `upgrade-history.md` for an in-place upgrade |

The freeze is not a substitute for the notice periods in
[`testnet/policies/breaking-change-notice.md`](../policies/breaking-change-notice.md).
It is a guard on the registry while the change is in flight.

---

## Unfreeze criteria

- [ ] The changelog contains the dated announcement and the completed-reset
      follow-up, including the deployed commit and version label or an
      explicit `not assigned`.
- [ ] `deployment-artifacts/contract-ids.txt` and `deployments/testnet.json`
      agree with each other and with the changelog; any `TBD` value is
      deliberately recorded as undeployed, not left over from a partial edit.
- [ ] `testnet/registry/contract-status.md` reflects the evidence: verified
      IDs and hashes where checks passed, or undeployed / `TBD` where they did
      not.
- [ ] Read-only checks from
      [`testnet/registry/verify-contract-live.md`](../registry/verify-contract-live.md)
      have run for every ID now claimed live; for an in-place upgrade, the
      code-hash, interface, behavior, and state-preservation checks in
      [`testnet/runbooks/upgrade-verification.md`](../runbooks/upgrade-verification.md)
      have run.
- [ ] `testnet/registry/last-verified.json` was refreshed **after** those
      checks, not before.
- [ ] `testnet/registry/wasm-hash-reference.md` and, for an in-place upgrade,
      `testnet/registry/upgrade-history.md` were updated with recorded values.
- [ ] `testnet/reset/historical-deployments.md` contains the new generation
      only when a redeploy or reset changed IDs, and the previous generation's
      end date was closed. For a hash-only in-place upgrade, the record is in
      `testnet/registry/upgrade-history.md` and
      `testnet/registry/wasm-hash-reference.md`.
- [ ] Verification links, breaking changes, and action-required text in the
      changelog match the final artifacts.
- [ ] If the controller configuration changed, the nonce guidance in
      [`testnet/runbooks/nonce-desync.md`](../runbooks/nonce-desync.md) was
      applied: caches invalidated where IDs changed, fresh `get_nonce` reads
      where they did not.
- [ ] The deployer/operator removes the banner only after all items above are
      true.

If a criterion cannot be met, leave the banner in place and record which
criterion failed. A failed verification is reported as "undeployed / not
verified", never as a live deployment. Do not silently unfreeze.

---

## Related Documentation

- [`testnet/policies/reset-policy.md`](../policies/reset-policy.md) - triggers,
  undecided authority, and the same-change artefact list
- [`testnet/policies/breaking-change-notice.md`](../policies/breaking-change-notice.md)
  - notice periods and exemptions
- [`testnet/reset/communication-template.md`](communication-template.md) - the
  announcement and follow-up text
- [`testnet/reset/historical-deployments.md`](historical-deployments.md) - the
  generation archive updated at unfreeze
- [`testnet/reset/rollback-considerations.md`](rollback-considerations.md) -
  what to do if the change must be reversed
- [`testnet/registry/contract-status.md`](../registry/contract-status.md) - the
  file carrying the banner
- [`testnet/registry/verify-contract-live.md`](../registry/verify-contract-live.md)
  - read-only verification
- [`testnet/runbooks/upgrade-verification.md`](../runbooks/upgrade-verification.md)
  - in-place upgrade verification
- [`testnet/docs/changelog.md`](../docs/changelog.md) - the single notification
  channel
- [`testnet/policies/data-retention.md`](../policies/data-retention.md) - what
  must never be written into an announcement

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-11-18 | Initial short freeze convention with banner text, edit boundaries, and unfreeze checklist |
