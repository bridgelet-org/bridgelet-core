# Testnet Reset and Fresh Deployment Procedure

## Status

> **There is no automated testnet deployment and no fixed reset cadence.**
> Every deployment is a manual run of `scripts/deploy-testnet.sh`. No named
> owner is recorded, and no Bridgelet contract is currently deployed per the
> canonical `testnet/registry/contract-status.md` (all four undeployed; two
> `TBD` and two `N/A` IDs; Last Verified `Never`).

This document is the **manual procedure** for standing up a fresh Bridgelet
deployment from scratch — after an SDF testnet reset, after a deliberate
redeploy, or as recovery from a broken deployment. It describes the operator
steps around the deploy. It does **not** reproduce, replace, or modify
`scripts/deploy-testnet.sh`.

## Scope

**In scope:** who initiates, preconditions, freeze and notice, running the
existing script as an external step, verification, documentation updates, and
notification.

**Explicitly out of scope:**

- The contents of `scripts/` or `tools/`. A bug found there is a separate
  issue, per `testnet/policies/contribution-guide.md`.
- `docs/api-reference.md`, and any change outside `testnet/` other than
  linking to the deployment artefacts the script writes.
- A mainnet deployment, an in-place `EphemeralAccount::upgrade()`, or an SDF
  platform reset itself. Those are covered by
  `testnet/policies/reset-policy.md` and
  `testnet/registry/upgrade-history.md`.
- Any promise of notice cadence or deployment stability. None exists.

## Who initiates

**The honest answer: authority is undecided.** No document names who may
authorise a redeploy. `testnet/policies/reset-policy.md` § Authority reaches
that conclusion explicitly, and `testnet/docs/contact-and-ownership.md` lists
a Testnet Infrastructure Lead and a Contract Security Team as *contacts*, not
as an authorisation policy.

| Question | Answer |
|---|---|
| Is a named deployment authority recorded? | No. Undecided. |
| Who can actually run the deploy? | In practice, whoever holds the testnet deployer key — the `SIGNER_SECRET_KEY` input the script requires. |
| Is the key rotated by a reset? | No. The deployer key survives; a redeploy gives its holder a fresh set of contracts. See `testnet/security/admin-key-hygiene.md`. |
| Who publishes the notice? | The person running the deploy. `testnet/policies/breaking-change-notice.md` puts accountability there, because running the script is the trigger its changelog process names. |
| Who is consulted? | The contacts in `testnet/docs/contact-and-ownership.md`; which one receives a 14-day notice is undecided. |

The rule this procedure adopts: **a redeploy is a state-changing operation on
shared infrastructure.** Treat it as one — get a recorded rationale, do not
redeploy as an investigation step, and publish the notice.

## Preconditions

- [ ] A specific commit has been chosen, and its SHA has been recorded. The
      deploy script does **not** write the deployed commit into
      `deployments/testnet.json`; capturing it is the operator's job.
- [ ] `stellar-cli` is installed and the `testnet` network is configured with
      passphrase `Test SDF Network ; September 2015`. See
      `testnet/config/soroban-cli-identity-setup.md`.
- [ ] The deployer identity is funded on testnet and is **testnet-only**.
      Per `testnet/policies/data-retention.md`, never use a mainnet key or any
      key that holds real value.
- [ ] The required deployment-time variables are set in the root `.env` flow:
      `SIGNER_SECRET_KEY`, `AUTHORIZED_SIGNER_PUBLIC_KEY`, `RECOVERY_ADDRESS`,
      `CREATOR_ADDRESS`, and optionally `RESERVE_ADMIN_ADDRESS`. Names only —
      no values belong in `testnet/`, in an issue, or in a commit.
- [ ] The working tree is clean and the chosen commit is checked out. No
      uncommitted `.env` is present.
- [ ] The applicable notice period has elapsed, or an exemption has been
      agreed and recorded. See Freeze and notice below.
- [ ] The build is known to produce the intended WASMs. Do not improvise a
      build step here; use the existing script.
- [ ] A funded read-only identity is available for verification, so the
      result can be checked independently of the deploy output.

## Freeze and notice

Before running the script, freeze further changes to the shared deployment:
no in-place upgrades, no re-initialisations, no ad-hoc invocations that
change state. The deploy creates new IDs; concurrent experiments against the
old deployment produce confusing evidence during verification.

Notice is owed under
[`../policies/breaking-change-notice.md`](../policies/breaking-change-notice.md):

| Change class | Minimum notice |
|---|---|
| Additive, published as "Other changes" | None |
| Argument, function, return, event or error-change classes | 7–14 days, per the ladder in that policy |
| Contract IDs change for any reason, including a reset | Maximum possible, plus a prominent changelog entry |
| Security fix where notice would widen exposure | Notice after the fix, same day, plus the upgrade log row |

The notification channel is `testnet/docs/changelog.md`, and nothing else.
Use the entry template there with all five fields present, and publish it
**before** deploying. Use
[`communication-template.md`](communication-template.md) for an unschedulable
redeployment, but note its current text publishes IDs to
`config/testnet_contracts.json`, a path that does not exist in this
repository; point it at `deployment-artifacts/contract-ids.txt` and
`deployments/testnet.json` first.

An SDF-initiated reset is exempt from advance notice. It still owes a
prominent retrospective entry as soon as the reset is known, and a follow-up
entry with the new IDs once redeployed.

## Deployment

Run the existing script as an external step from the repository root. Its
internals are deliberately not copied here: read `scripts/deploy-testnet.sh`
if you need to understand them, and do not edit it as part of a documentation
change.

```bash
# Record what is being deployed before anything changes.
git rev-parse HEAD

# Run the project's existing deployment script (it loads the root .env).
./scripts/deploy-testnet.sh
```

The script builds via `scripts/build.sh`, deploys the contracts, initializes
them with the configured creator/signer/reserve/factory parameters, and
rewrites `deployments/testnet.json` and
`deployment-artifacts/contract-ids.txt`. Treat those two files as **output**,
not as verified truth. Preserve the script's full output alongside the commit
SHA; it is the evidence from which the registry will be updated.

If the script fails partway, do not assume the deployment is clean. Some
contracts may have new IDs while others do not. Diagnose from the output, then
either complete the deployment deliberately or start again and treat the
result as another fresh deployment. Never replay pre-reset transaction
history as if the old state still existed.

## Verification

Nothing in the documentation may claim liveness until this step is done. The
procedure is
[`../registry/verify-contract-live.md`](../registry/verify-contract-live.md);
the three pre-flight steps are in `reset-policy.md`.

```bash
export NETWORK=testnet
export RPC_URL="https://soroban-testnet.stellar.org"
export READER="<funded-testnet-identity>"

# A controller that answers at all is live. A freshly redeployed one is at 0.
stellar contract invoke --send no \
  --id "<your-stored-sweep-controller-id>" --network "$NETWORK" \
  --source "$READER" -- get_nonce

# get_info proves initialization; get_status alone does not.
stellar contract invoke --send no \
  --id "<your-stored-ephemeral-account-id>" --network "$NETWORK" \
  --source "$READER" -- get_info
```

Check, in order:

1. **Each new ID resolves**, using the per-contract read-only methods in
   `verify-contract-live.md`.
2. **The WASM hash matches** the intended commit, per
   `testnet/registry/wasm-hash-reference.md`. An unannounced behaviour change
   with an unchanged ID is an upgrade, not a redeploy — see
   `testnet/registry/upgrade-history.md`.
3. **Configured roles are as intended**: creator, controller, admin,
   `authorized_signer`, `authorized_destination`, and reserve admin. Use
   `testnet/registry/admin-addresses.md` to record the public values.
4. **The network itself did not move** during the deploy. The latest-ledger
   check is in `testnet/runbooks/rpc-outage.md` §6. A ledger sequence that
   drops means the platform reset, not a project redeploy.
5. **The nonce starts at `0`** on the new controller. Any locally cached
   nonce is meaningless. See `testnet/config/nonce-tracking.md`.

Only after these checks pass, update `testnet/registry/last-verified.json`
with ISO timestamps. A recent timestamp alone is not evidence of liveness.

## Documentation updates

The deployment is not complete until the documentation is updated in the same
change. Work through
[`post-reset-checklist.md`](post-reset-checklist.md) row by row, assigning an
owner, a UTC date, and a status to each. The gate in that checklist — verify
before writing — is the point of doing it this way.

The minimum set is the dated changelog entry, `network-config.md`,
`deployment-artifacts/contract-ids.txt` (written by the script),
`contract-status.md`, `last-verified.json`, and `wasm-hash-reference.md`.
`testnet/policies/reset-policy.md` lists the same required artefacts and
explains why a redeploy announced only in source did not properly happen.

## Notification

Publish the dated changelog entry that was prepared in Freeze and notice. For
an unschedulable event, use
[`communication-template.md`](communication-template.md) after correcting its
stale path. For a breaking change, the entry must include the old and new
signatures verbatim, a migration example, and an explicit action-required
line; a notice published after the fact documents a breach rather than
discharging the obligation.

Record the application of the proposed deployment tag, if one was assigned,
consistently across `testnet/registry/upgrade-history.md`,
`testnet/docs/changelog.md`, and `testnet/registry/wasm-hash-reference.md` per
[`../policies/versioning-scheme.md`](../policies/versioning-scheme.md). No
current deployment carries such a tag; it is a proposed convention, not an
enforced one.

## What this procedure does not provide

- **No cadence.** No document states an observed SDF reset interval.
- **No guaranteed lead time.** An SDF reset gives none, and this project has
  no commitment beyond the changelog.
- **No automation.** The deploy workflow is `workflow_dispatch`-only and the
  procedure above is manual.
- **No stability promise.** A merge to `main` is not a deploy trigger.
- **No enforcement.** These are habits and checklists, not mechanical
  controls. Nothing watches for a redeploy without prior notice.

## Related Documentation

- [`post-reset-checklist.md`](post-reset-checklist.md) — every `testnet/` doc
  to update after the deploy
- [`communication-template.md`](communication-template.md) — announcement
  template and its stale-path caveat
- [`../policies/reset-policy.md`](../policies/reset-policy.md) — authority,
  destruction, detection, and required artefacts
- [`../policies/breaking-change-notice.md`](../policies/breaking-change-notice.md)
  — notice owed and the notification channel
- [`../policies/versioning-scheme.md`](../policies/versioning-scheme.md) — the
  proposed deployment tag
- [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md)
  — the read-only verification procedure
- [`../registry/contract-status.md`](../registry/contract-status.md) — the
  canonical status record
- [`../docs/changelog.md`](../docs/changelog.md) — the notification channel
- [`../docs/network-config.md`](../docs/network-config.md) — published IDs
- [`../security/admin-key-hygiene.md`](../security/admin-key-hygiene.md) — the
  deployer key and why a reset does not rotate it
- [`../policies/data-retention.md`](../policies/data-retention.md) — the
  no-real-data and no-real-keys rules
- [`../policies/contribution-guide.md`](../policies/contribution-guide.md) —
  why `scripts/` and `tools/` are left alone

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-11-18 | Initial manual fresh-deployment procedure |
