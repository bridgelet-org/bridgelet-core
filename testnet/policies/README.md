# Testnet policies

## Purpose

This directory holds the process and governance documents for the shared
Bridgelet testnet deployment: how it is versioned, when it is reset, what
notice integrators are owed, and what may be written to it.

These are policies for **one shared deployment**, not for the contract design.
Design-level questions belong in `docs/`. See
[`contribution-guide.md`](contribution-guide.md) for the split.

## Status

No Bridgelet contracts are currently deployed to testnet
(`testnet/registry/contract-status.md`). Most policies here are written ahead
of first use, and each says so in its own status section. Numbers such as
notice periods and retention windows are proposed defaults, not validated ones.

## Index

| Document | Question it answers |
|---|---|
| [`breaking-change-notice.md`](breaking-change-notice.md) | How much notice is owed before a redeploy changes an interface? |
| [`contribution-guide.md`](contribution-guide.md) | How do I add to the `testnet/` documentation tree? |
| [`data-retention.md`](data-retention.md) | What may be written to testnet, and why is it permanent? |
| [`deprecation-policy.md`](deprecation-policy.md) | How long is a superseded contract ID kept as "previously live"? |
| [`reset-policy.md`](reset-policy.md) | What happens, and what can integrators rely on, when testnet is reset? |
| [`versioning-scheme.md`](versioning-scheme.md) | What do the `testnet-v1.x` / `testnet-v2.x` tags mean? |
| [`who-can-request-a-reset.md`](who-can-request-a-reset.md) | Who may request, approve, and carry out a reset? |

## Reading order

- **Integrators:** `reset-policy.md`, then `breaking-change-notice.md`, then
  `versioning-scheme.md`.
- **Maintainers running a deploy:** `breaking-change-notice.md`,
  `who-can-request-a-reset.md`, then `deprecation-policy.md`.
- **Contributors:** `contribution-guide.md`, then `data-retention.md`.

## Adding a policy

Follow [`contribution-guide.md`](contribution-guide.md), and add a row to the
index table above in the same pull request.

## Related Documentation

- [`testnet/docs/changelog.md`](../docs/changelog.md) — the notification
  channel the policies rely on
- [`testnet/registry/contract-status.md`](../registry/contract-status.md) —
  current deployment status
- [`testnet/reset/communication-template.md`](../reset/communication-template.md)
  — the announcement template for a reset or redeploy

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-10-18 | Initial index for `testnet/policies/` |
