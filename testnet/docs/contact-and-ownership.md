# Testnet Documentation: Contact & Ownership

## Status

**The repository does not name a single maintainer, owner, or on-call contact
for the Bridgelet testnet deployment or the `testnet/` documentation tree.**
"Maintainers" is used generically in the docs, but no person, team, `CODEOWNERS`
file, or review policy is recorded. The role labels and addresses listed below
are existing contact placeholders; this document does not establish that any
inbox or channel is monitored.

| Question | Answer from repository evidence |
|---|---|
| Who owns testnet? | Not assigned. Generic maintainers only. |
| Who reviews `testnet/` changes? | No owner, no review policy, no `CODEOWNERS`; see [`../policies/contribution-guide.md`](../policies/contribution-guide.md). |
| Who may authorize a redeploy? | Undecided; see [`../policies/reset-policy.md`](../policies/reset-policy.md). |
| Is anyone on call? | No. Support is best effort; see [`support.md`](support.md) and [`../runbooks/on-call-testnet.md`](../runbooks/on-call-testnet.md). |
| Are the listed contacts verified? | No. Treat them as unverified unless a maintainer confirms them. |

## Contact roles and escalation

These are the contact labels already present in this file, retained for
continuity. **They are not evidence of staffed roles.**

| Role label | Listed contact | What the repository establishes | What it does not establish |
|---|---|---|---|
| Testnet Infrastructure Lead | `admin@bridgelet.org` | Also referenced by `../security/disclosure-process.md` as a possible route for key-holder matters | That the inbox exists, is monitored, or belongs to a named deployer |
| Contract Security Team | `security@bridgelet.org` | Also referenced by `../security/disclosure-process.md` as the preferred private email fallback | That a security team is staffed or that private vulnerability reporting is enabled |
| Community Discord / Support Channel | `#testnet-support` | Previously listed here as a community contact | A channel link, a moderator, or a response expectation |

Do not add secret keys, signing seeds, or unredacted configuration to any of
these routes.

## Maintenance responsibilities

These are **role descriptions**, not assignments to a named person.

| Area | Responsibility | Where the process is defined |
|---|---|---|
| Deployment status registry | Verify status on-chain, then curate `contract-status.md`, `last-verified.json`, `wasm-hash-reference.md`, `upgrade-history.md`, and admin-address records | [`../registry/README.md`](../registry/README.md) and [`../registry/contract-status.md`](../registry/contract-status.md) |
| Network and integration docs | Keep network settings, getting started, FAQ, support, and ownership docs consistent with verified state | [`network-config.md`](network-config.md), [`getting-started.md`](getting-started.md), [`faq.md`](faq.md), this file |
| Changelog and notices | Add a dated entry at the top for every deploy-script run, in-place upgrade, or SDF reset; include deployed commit, IDs or "unchanged", breaking changes, other changes, and required action | [`changelog.md`](changelog.md) and [`../policies/breaking-change-notice.md`](../policies/breaking-change-notice.md) |
| Deployment artifacts | Regenerate `deployments/testnet.json` and `deployment-artifacts/contract-ids.txt`, and update `network-config.md` in the same change | [`../policies/reset-policy.md`](../policies/reset-policy.md) |
| Runbooks and incident notes | Preserve evidence, classify the symptom, route ordinary problems to `[testnet]` issues, and complete a postmortem | [`../runbooks/on-call-testnet.md`](../runbooks/on-call-testnet.md) and [`../runbooks/incident-postmortem-template.md`](../runbooks/incident-postmortem-template.md) |
| Security observations | Triage source/deployment findings under the lightweight disclosure process; do not claim an audit or embargo process exists | [`../security/disclosure-process.md`](../security/disclosure-process.md) and [`../security/README.md`](../security/README.md) |

The canonical status registry currently records all four contracts as
undeployed, with `TBD`/`N/A` IDs and `Last Verified: Never`, despite conflicting
deployment artifacts. Updating a status claim requires on-chain verification,
not copying an ID from another file.

## Incident reporting route

From [`support.md`](support.md):

1. For deployment, ID, configuration, or behavior problems, open a repository
   issue titled with **`[testnet]`** and apply the `testnet` label. Include
   contract IDs, transaction hash, network passphrase, client versions, ledger
   or UTC time, and what you tried, expected, and observed.
2. For a bug in source, tests, build scripts, or a feature request, open a
   regular issue without the `testnet` label.
3. For a possible vulnerability, **do not open a public issue with the
   mechanism**. [`support.md`](support.md) directs reporters to GitHub's private
   vulnerability reporting control. However,
   [`../security/disclosure-process.md`](../security/disclosure-process.md)
   records that this control is currently disabled; it suggests the
   `security@bridgelet.org` and `admin@bridgelet.org` contacts above, or an
   impact-only public issue, as substitutes. That is a gap, not a staffed
   escalation path.
4. Never post `S...` secret keys, signing seeds, or private authorization data.

Support is best effort. No acknowledgement time, response time, fix date, or SLA
is promised.

## Escalation gaps

| Gap | Repository evidence | Practical consequence |
|---|---|---|
| No named deployment or docs owner | Generic "maintainer" language; no `CODEOWNERS`; `../policies/contribution-guide.md` leaves the reviewer question open | There is no person to page for a docs correction or redeploy decision |
| No active on-call rotation | `../runbooks/on-call-testnet.md` states `testnet_on_call: not_staffed` | Incidents wait for whoever sees the issue |
| No monitored alert route | `../runbooks/on-call-testnet.md` states `testnet_monitoring: not_configured` | Drift and outages are detectable only by checking manually |
| Unverified role contacts | This file's addresses/channel are not confirmed as monitored by any repo evidence | A non-reply is not meaningful status information |
| No deployment authority | `../policies/reset-policy.md` and `../policies/breaking-change-notice.md` leave authorization and notice roles undecided | Emergency mutation has no recorded approver |
| Private vulnerability reporting disabled | `../security/disclosure-process.md` | Sensitive reports have no guaranteed private channel |
| No mechanical notice enforcement | `../policies/breaking-change-notice.md` describes a habit and review checklist, not a control | A redeploy can go unannounced |
| Documentation drift is unresolved | [`terminology-drift-log.md`](terminology-drift-log.md) | A copied ID or deployment claim may be stale |

## What would close the gaps

- Name a primary and backup maintainer, with acknowledged responsibilities for
  the registry, `testnet/docs/`, and the changelog.
- Record who may authorize a deploy, upgrade, or emergency change.
- Verify or replace the contact routes above, and enable a private reporting
  channel before treating security reports as confidential.
- Add review ownership for `testnet/` changes.
- Add automated health checks and a monitored alert route before claiming
  operational coverage.
- Resolve the logged drift and reconcile the canonical registry with deployment
  artifacts only after on-chain verification.

## Related documentation

- [`support.md`](support.md) — incident and support intake
- [`overview.md`](overview.md) — testnet documentation entry point
- [`changelog.md`](changelog.md) — deployment-visible change log
- [`terminology-drift-log.md`](terminology-drift-log.md) — known testnet-facing
  claim drift
- [`../registry/README.md`](../registry/README.md) — registry maintenance
  workflow
- [`../policies/contribution-guide.md`](../policies/contribution-guide.md) —
  contribution scope and unresolved review questions
- [`../policies/reset-policy.md`](../policies/reset-policy.md) — reset, notice,
  and redeploy gaps
- [`../runbooks/on-call-testnet.md`](../runbooks/on-call-testnet.md) — support
  posture and activation requirements
- [`../security/disclosure-process.md`](../security/disclosure-process.md) —
  private reporting status
- [`../security/admin-key-hygiene.md`](../security/admin-key-hygiene.md) —
  expected custody practices for privileged keys

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-11-18 | Replaced ownership stub with verified responsibilities, support routing, and explicit escalation gaps |
