# Deprecation Policy for Superseded Testnet Contract IDs

> **Status: proposed policy.** No maintainer has ratified the window below. It is
> a concrete default that should be confirmed (or changed) by the owner named in
> `testnet/docs/contact-and-ownership.md`. Until ratified, follow it as guidance.

## Problem

Every redeploy mints new contract IDs (`testnet/policies/reset-policy.md`). Old IDs
remain in documents, SDK configs and integrator code. Without a rule, the
registry accumulates indefinite historical IDs, or loses them before integrators
have a chance to understand what happened.

## Terms

| Term | Meaning |
|---|---|
| **Live** | The ID is the current deployment and verified (`testnet/registry/verify-contract-live.md`) |
| **Previously live** | Superseded by a newer deployment or destroyed by a reset; kept only as a historical record |
| **Deprecated** | Removed from active documentation; recoverable only from version control |

Note: this repository has no `testnet/reset/historical-deployments.md` today.
Historical IDs currently live in `testnet/registry/upgrade-history.md` and
`testnet/docs/changelog.md`; use those until a dedicated file exists.

## Proposed rule

1. **At supersession** the old ID is marked *previously live* in the same pull
   request that records the new deployment, with: the old ID, the date it was
   last known live, the date it was superseded, and a pointer to the replacement.
2. **Previously live window: 90 days** after supersession. During this window
   the ID stays in `testnet/registry/` as a labelled historical row so integrators who
   still hold it can recognize it and migrate.
3. **After 90 days** the ID is *deprecated*: remove it from active registry
   tables and from examples. Keep one line in `testnet/docs/changelog.md`
   (ID prefix, dates, replacement) so searches still resolve; the full record
   remains in git history.
4. A platform-level testnet reset (SDF) supersedes everything at once. The same
   window applies from the date the reset was detected, not announced.
5. **Never reuse a deprecated ID's role silently.** Examples and runbooks must
   name the current ID source (the registry), not paste an ID.

## Extensions and exceptions

- Extend the window (to 180 days) if the ID is embedded in a published SDK
  release or a public demo still pointing at it.
- Shorten it (to 30 days) only if keeping the row would mislead, for example the
  ID was a misconfigured deployment. Record the reason.
- IDs for the long-lived demo accounts follow `testnet/policies/demo-account-etiquette.md`.

## Who applies it

Whoever performs or records a redeploy applies steps 1-3 as part of the
deployment pull request (see `testnet/policies/who-can-request-a-reset.md`).
A periodic sweep (for example when the weekly health summary is filled in)
should check for rows older than the window.

## Rationale

90 days is long enough to cover an integrator's typical release/test cycle and
short enough that the registry stays readable. It is a judgement call, not
derived from observed usage; adjust it with data.
