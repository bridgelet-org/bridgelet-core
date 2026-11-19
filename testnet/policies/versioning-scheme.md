# Testnet Deployment Versioning Scheme

## Status

> **This is a proposed convention, not an enforced reality.** No Bridgelet
> deployment currently carries a tag. The canonical
> `testnet/registry/contract-status.md` records all four contracts as
> undeployed, with two `TBD` and two `N/A` IDs and Last Verified `Never`.
> Nothing on-chain stores a version label, no tool validates one, and no
> workflow applies one.

This document defines a lightweight label so that three existing records can
be read as one deployment epoch. It does not create a release process, change
the deploy script, or make any claim about what is live.

## Purpose

Three files already describe the same deployment from different angles, and
nothing ties them together:

| Record | What it answers |
|---|---|
| `testnet/docs/changelog.md` | When did the deployment change, and what did integrators have to do? |
| `testnet/registry/upgrade-history.md` | Which in-place `upgrade()` changed the WASM behind an unchanged ID? |
| `testnet/registry/wasm-hash-reference.md` | Which WASM hash and commit is that ID supposed to be running? |

A single tag — for example `testnet-v1` or `testnet-v2` — lets a reader find
the matching row in all three. Without it, correlating them by date and commit
SHA is manual, and the repository already contains a deployment record that
disagrees with the canonical registry.

## The tags

| Tag | Intended scope |
|---|---|
| `testnet-v1` / `testnet-v1.x` | Initial MVP deployment for ephemeral-account sweep validation. |
| `testnet-v2` / `testnet-v2.x` | Multi-asset support and governance modules. |

These are the existing v1/v2 notions, made explicit. They are **proposed
labels for future deployment epochs**, not names of anything deployed today.
Whether `testnet-v2.x` is additive or breaking is undecided — see
`testnet/policies/breaking-change-notice.md` § What Is Not Decided. Both tags
are deployment tags, not document revisions: the `## Version History` table at
the bottom of each file is unrelated.

## What a tag means

A tag is a human label for **one deployment epoch**: a set of records that
refer to the same code, contract IDs, WASM hashes, and configuration.

| Aspect | Precise meaning |
|---|---|
| Format | `testnet-vN` for a major epoch; `testnet-vN.M` for a revision within it. |
| Scope | One shared testnet deployment epoch. Not the repository, not a contract crate, not a document. |
| Lifetime | From the dated changelog entry that first carries it until the next tag is applied. |
| Authority | Undecided. No role in this repository is recorded as able to assign one. |
| Evidence | A tag is a claim in documentation. It is corroborated only by the ID, WASM hash, commit SHA, and date recorded beside it. |

A tag is **not**:

- an on-chain value or a contract function;
- a substitute for a contract ID, WASM hash, or commit SHA;
- evidence of liveness — `verify-contract-live.md` is the only procedure that
  turns a stored ID into a verified one;
- a semantic-version guarantee enforced by CI;
- a replacement for the canonical `contract-status.md`, which remains the
  source of truth for deployed/not-deployed.

If a tag and the status registry disagree, the registry wins and the tag is a
documentation defect.

## When to bump

Proposed rules. They are deterministic enough to apply consistently, but
nobody is currently obliged to apply them.

| Event | Tag action | Rationale |
|---|---|---|
| First deployment of the MVP | Assign `testnet-v1` | Starts the epoch. |
| Deployment of multi-asset and governance modules | Assign `testnet-v2` | A new supported feature era, as defined above. |
| Breaking interface change requiring integrator migration | Bump **major** | Same class as a new era; the changelog entry carries the old and new signatures. |
| Additive deployed change, same IDs | Bump **minor** (`testnet-v1.1`) | A new revision within the era; no migration required. |
| In-place `upgrade()` changing behaviour with an unchanged interface | Bump **minor** | The ID is unchanged, so the tag is the only visible revision marker. Log the upgrade too. |
| Recovery redeploy after an SDF reset, same code | Bump **minor** | New IDs, same era. The changelog entry states the IDs changed and the code did not. |
| Redeploy of unchanged code with no behaviour change | Bump **minor** | Every redeploy gets a dated entry and a distinct label. |
| Security hotfix applied without advance notice | Bump **minor**, record retrospectively | Notice is owed after the fix, in the same day, per the notice policy. |
| Source-only change in the changelog's "Unreleased" section | **No bump** | Not deployed. The tag describes a deployment, not `main`. |
| Documentation-only change | **No bump** | Use the file's own `## Version History` instead. |

A merge to `main` is never a bump trigger. Nothing deploys automatically, as
`testnet/docs/faq.md` records.

## Where recorded

The tag must appear in all three records for the same deployment. If it
appears in only one, it is not applied.

| Record | Where the tag goes | Notes |
|---|---|---|
| `testnet/docs/changelog.md` | A `**Deployment tag:**` line in the new dated entry, beside the deployed commit | Use the existing five-field entry template; the tag is a sixth field, not a replacement for any of them. |
| `testnet/registry/upgrade-history.md` | A `Tag` column on the row for an in-place `upgrade()` | A fresh redeploy is a new ID, not an upgrade. Do not invent an upgrade row just to carry a tag. |
| `testnet/registry/wasm-hash-reference.md` | A `Tag` column on the Current WASM Hashes row and on the Commit-to-Hash Mapping row | The commit SHA and hash stay authoritative; the tag is a cross-reference. |

Proposed changelog entry shape, using the existing template:

```markdown
## YYYY-MM-DD — <short title>

Deployed commit: `<sha>`
**Deployment tag:** `testnet-v1.1`

**Contract IDs:** new (see below)
**Breaking changes:** none
**Other changes:** …
**Action required:** none
```

`testnet/registry/contract-status.md` is deliberately **not** a fourth tagged
record: it records deployed/not-deployed state and must not be made to imply a
tag is live. `testnet/registry/last-verified.json` stays timestamp-only.

## Reporting convention

An integrator reporting an observation should cite the tag **and** the
specific identifiers, not the tag alone:

| Context | Report as |
|---|---|
| A successful sweep | tag, `SweepController` ID, transaction hash, ledger |
| An unexpected behaviour change | tag, contract ID, WASM hash, deployed commit |
| A possible unannounced upgrade | tag, contract ID, expected hash vs observed hash, entry in `upgrade-history.md` |
| A reset or failing ID | tag from the last known deployment, the ID that failed, and the latest-ledger value |
| An entry still in "Unreleased" | `untagged / unreleased`, source commit — never a tag |

Example wording, with no real identifiers:

> Observed on `testnet-v1.1`, `SweepController` `<contract-id>`, ledger
> `<N>`, transaction `<hash>`. `get_nonce` was `<n>` immediately before
> signing.

Find which tag was in force at a given entry without leaving the repository:

```bash
grep -n -E '^## [0-9]{4}-[0-9]{2}-[0-9]{2}|^\*\*Deployment tag:\*\*' \
  testnet/docs/changelog.md
```

A report that names only a tag is not actionable. A report that names a tag,
an ID, and a ledger is checkable.

## Applying a tag

- [ ] Confirm the deployment against
      `testnet/registry/verify-contract-live.md`; do not tag an unverified
      deployment.
- [ ] Decide major vs minor using [When to bump](#when-to-bump), and record
      the rationale in the changelog entry.
- [ ] Add the tag to the new dated `testnet/docs/changelog.md` entry.
- [ ] Add the tag to the relevant `testnet/registry/upgrade-history.md` row,
      if this was an in-place upgrade.
- [ ] Add the tag to the matching rows in
      `testnet/registry/wasm-hash-reference.md`.
- [ ] Confirm the same commit SHA and WASM hash appear in all three places.
- [ ] Use the same tag in the announcement and in
      `testnet/reset/post-reset-checklist.md`.
- [ ] Do **not** mark any contract deployed in `contract-status.md` without
      the verification step above.

## What is not decided

Genuinely open. Writing a convention here does not settle who runs it.

1. **Who assigns a tag.** No authorisation policy names a role.
2. **Whether every redeploy is tagged.** `testnet/policies/reset-policy.md`
   lists this as undecided; no recorded deployment carries one.
3. **Whether a machine-readable companion exists.** An integrator could diff
   a JSON file more reliably than a Markdown table, but none is proposed or
   built.
4. **Whether `testnet-v2.x` is additive or breaking**, and therefore whether
   it resets the notice clock.
5. **Whether a recovery redeploy after a reset keeps the pre-reset minor
   number.** The rule above says bump minor; it has never been applied.
6. **Whether the tag should appear in `verify-contract-live.md` output.**
   Today that procedure returns contract state, not documentation metadata.

## Related Documentation

- [`testnet/docs/changelog.md`](../docs/changelog.md) — the notification
  channel and its entry template
- [`testnet/registry/upgrade-history.md`](../registry/upgrade-history.md) —
  in-place upgrades, which keep the ID
- [`testnet/registry/wasm-hash-reference.md`](../registry/wasm-hash-reference.md)
  — the commit-to-hash mapping
- [`testnet/registry/contract-status.md`](../registry/contract-status.md) —
  the canonical status record a tag cannot override
- [`testnet/registry/verify-contract-live.md`](../registry/verify-contract-live.md)
  — the only procedure that converts a stored ID into a verified one
- [`testnet/policies/reset-policy.md`](reset-policy.md) — triggers,
  authority, and the undecided questions this scheme inherits
- [`testnet/policies/breaking-change-notice.md`](breaking-change-notice.md) —
  what notice a major bump owes
- [`testnet/policies/access-model.md`](access-model.md) — who may call the
  deployment, independent of how it is labelled
- [`testnet/reset/post-reset-checklist.md`](../reset/post-reset-checklist.md)
  — where a tag is applied during a redeploy

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-11-18 | Replaced the thin stub with a proposed tag convention; explicitly no deployment is tagged |
