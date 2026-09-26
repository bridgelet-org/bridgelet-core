# Testnet Security Observations

## What this directory is for

This directory tracks **security-relevant observations from the live testnet deployment** — things that actually happened, or could plausibly happen, to the shared contracts that integrators are building against right now.

It is deliberately a different document set from [`docs/security.md`](../../docs/security.md).

| | `docs/security.md` | `testnet/security/` (here) |
|---|---|---|
| Question | "Is the design sound?" | "What is the shared deployment actually doing, and who can affect it?" |
| Nature | Design-level threat model, static | Operational, observational, time-varying |
| Scope | The contracts as specified | **One shared deployment** that many unrelated parties use |
| Time | Stable | Goes stale; the deployment changes underneath it |
| Answers | "What are the mitigations?" | "Someone just did X — is that a bug or an attack?" |
| Changes when | The design changes | Anyone touches the shared deployment |

The distinction is not stylistic. `docs/security.md` can be correct and complete while the testnet deployment is being actively misbehaved by one of its users, and nothing in a design document would reflect that.

> `docs/security.md` is not edited by anything in this directory, and nothing here should be read as a correction to it. The two document different things.

## Why testnet needs its own security tracking

Testnet is usually dismissed as "not real", so the instinct is that it does not need this. That instinct is wrong here, for three specific reasons.

**1. The deployment is shared, and that makes it a real multi-tenant system.** Unlike a local sandbox, other unrelated parties deploy accounts against, sweep through, and configure the same contracts. An action taken by one integrator degrades the experience of every other integrator. That is a security property of the environment, not of the contracts.

**2. Some compromises here are indistinguishable from bugs until you know to look.** A sweep that fails with a bare `ed25519_verify` trap looks like a signing bug. It is frequently nonce contention from someone else's sweep. A `batch_initialize` that returns `success: false` for every account looks like a broken factory. It may be index-slot exhaustion caused by a previous caller. Triage requires knowing what abuse looks like — which is what this directory provides.

**3. "It's only testnet" reasoning has already produced a real weakness.** `EphemeralAccount::sweep()` accepts an `auth_signature` parameter and ignores it entirely. It is not exploitable for theft, precisely because it does not move tokens and requires the controller's auth. But it is a live footgun on a deployment people are experimenting against, and people are already misreading it. See [`unverified-signature-path-warning.md`](unverified-signature-path-warning.md).

## Index

| Document | Scope | Read it when |
|---|---|---|
| [`unverified-signature-path-warning.md`](unverified-signature-path-warning.md) | `EphemeralAccount::sweep()` does not verify its signature | Before test-calling `sweep()` directly, or when reasoning about whether signature checking works |
| [`admin-key-hygiene.md`](admin-key-hygiene.md) | Handling the testnet deployer/admin key | You hold, are about to hold, or are reviewing the testnet admin key |
| [`known-testnet-abuse-patterns.md`](known-testnet-abuse-patterns.md) | What griefing the shared deployment looks like, and the contract behaviour each produces | Testnet behaves unexpectedly and you need to decide "broken" vs "someone is messing with shared state" |

## Standing rules for this directory

- **Observations, not accusations.** Record what the contracts did and what the plausible cause was. A report that names a party without evidence is noise, and on a shared deployment it is also unfair.
- **Public information only.** Contract IDs, addresses, transaction hashes, ledger numbers, and event data. **Never** record a secret key, a seed, a signature intended for someone else, or any credential. The same no-secrets rule that governs `testnet/config/` applies here.
- **Separate observation from interpretation.** State the on-chain fact first, then the hypothesis. Anyone should be able to check the first part independently.
- **Prefer the runbooks for mechanics.** These documents say what an attack looks like and what it does. The step-by-step diagnostics live in `testnet/runbooks/`.
- **Timestamp observations.** A shared deployment changes. "This was true on 2026-09-26 at ledger N" is useful; an undated claim is not.

## Current state of the deployment

As of the last review, **no Bridgelet contracts are deployed to testnet.** `testnet/registry/contract-status.md` records all four contracts as undeployed, and `testnet/registry/known-test-accounts.md` and `testnet/registry/admin-addresses.md` are unpopulated templates.

This directory is therefore being written **ahead of the deployment**, describing the abuse surface the code has and the triage reference that will be useful once it is live. `deployments/testnet.json` and `deployment-artifacts/contract-ids.txt` do contain IDs from a 2026-07-12 deployment, but those are not reflected in the status registry — treat them as unverified until someone confirms them on-chain per `testnet/registry/verify-contract-live.md`.

When the deployment goes live, the first entries here should be: a baseline observation recording the deployed WASM hashes and admin/controller addresses, and confirmation of which abuse patterns have been observed versus merely reasoned about.

## Related documentation

- [`docs/security.md`](../../docs/security.md) — design-level security model (different concern)
- [`docs/reentrancy-analysis.md`](../../docs/reentrancy-analysis.md) — reentrancy reasoning
- [`docs/SIGNATURE_FORMAT.md`](../../docs/SIGNATURE_FORMAT.md) — signature message format
- `testnet/runbooks/` — step-by-step diagnostics; pair with the abuse patterns doc
- `testnet/registry/admin-addresses.md` — who holds which privileged roles
- `testnet/registry/upgrade-history.md` — the log for unauthorised upgrades
- `testnet/config/README.md` — the same no-secrets discipline applied to config docs

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-26 | Initial testnet security observations index |
