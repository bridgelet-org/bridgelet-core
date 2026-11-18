# Who Can Request or Approve a Testnet Reset / Redeploy

> **Status: proposed policy.** `testnet/policies/reset-policy.md` states that no
> named authority exists today. This document proposes one so that no individual
> contributor resets shared testnet state unilaterally. It becomes binding only
> once a maintainer ratifies it.

## Scope: what "reset" means here

Two events (see `testnet/policies/reset-policy.md`):

1. **SDF platform reset** — decided by the Stellar Development Foundation; this
   project cannot request or approve it. This policy covers only how the project
   *responds*.
2. **Bridgelet redeploy** — re-running `scripts/deploy-testnet.sh`, or an
   in-place `upgrade()`. **This policy governs this one.**

A "reset" request therefore means: "redeploy the Bridgelet contracts to testnet,
replacing the IDs integrators rely on."

## Proposed roles

| Role | Who (to be named by maintainers) | Authority |
|---|---|---|
| **Requester** | Any contributor | May open a reset request issue; may not execute a redeploy |
| **Approver** | Testnet Infrastructure Lead (`testnet/docs/contact-and-ownership.md`), or a named delegate | Approves or rejects the request |
| **Executor** | The holder of the deployer key | Runs the deployment only after approval is recorded |
| **Security consult** | Contract Security Team (same document) | Must be consulted if the reason is a security issue or the signer/admin keys change |

If a role is unfilled, the request waits; nobody self-approves.

## Request process

1. Open an issue titled `Testnet reset request: <reason>` with: why it is needed,
   what it will break (see "Destroyed by a redeploy" in the reset policy),
   whether a lighter option exists (e.g. deploying a new child account instead of
   redeploying everything), and a proposed date.
2. **Minimum notice: 7 days** before execution, except for emergencies (below).
   Announce using `testnet/reset/communication-template.md` and add a pending
   entry to `testnet/docs/changelog.md`.
3. The approver records approval on the issue. The executor links that approval
   in the deployment pull request.
4. Execute, then follow the registry update and verification steps
   (`testnet/registry/verify-contract-live.md`); mark old IDs per
   `testnet/policies/deprecation-policy.md`.

## Emergencies

A redeploy without the notice period is allowed only for: a discovered security
flaw, a broken deployment that blocks all integrators, or a leaked admin/signer
key. It still needs approver sign-off (a documented chat approval is acceptable
if followed by the issue within 24 hours) and an immediate changelog entry.

## What does not require approval

- Deploying **new** ephemeral accounts or a private scratch controller with your
  own identity for experiments, as long as the registry IDs are untouched.
- Read-only verification.

## Accountability

The executor is named in the changelog entry. Redeploys without a recorded
approval should be reverted in documentation (restore the previous registry state
and explain) and reviewed.

## Open items for maintainers

- Name the approver and delegate.
- Confirm the 7-day notice.
- Decide how the deployer key is held so execution is not possible by accident
  (see `testnet/security/admin-key-hygiene.md`).
