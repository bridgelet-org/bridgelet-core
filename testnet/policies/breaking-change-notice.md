# Breaking Change Notice Policy for the Shared Testnet Deployment

## Purpose

The Bridgelet testnet deployment is **shared and permissionless**. Other
people's integration code calls these contracts. When the interface changes
under them, their tests break at a moment they did not choose, against a
network they do not control, with no SLA and no rollback.

This policy defines **how much notice is owed, through which channel, before a
breaking redeploy**, and — just as importantly — **which changes cannot be
scheduled at all**, so that the honest limits are written down rather than
implied away.

## Status and verification

| Statement | Basis |
|---|---|
| `EphemeralAccount::initialize` currently takes five arguments: `env, creator, expiry_ledger, recovery_address, authorized_controller, admin` | Read from `contracts/ephemeral_account/src/lib.rs` |
| The changelog's "Unreleased" section describes a sixth argument, `authorized_signer: BytesN<32>`, inserted between `authorized_controller` and `admin` | `testnet/docs/changelog.md` |
| `AccountStatus` is `0 Active`, `1 PaymentReceived`, `2 Swept`, `3 Expired` | Read from `contracts/shared/src/types.rs` |
| `EphemeralAccount` errors are discriminants 1–15; `SweepController` errors are 1–11 then 13, **skipping 12**; `ReserveContract` errors are 1–6 | Read from each `contracts/*/src/errors.rs` |
| `AccountFactory` defines no `#[contracterror]` enum at all | Read from `contracts/account_factory/src/lib.rs` |
| No Bridgelet contract is currently deployed | `testnet/registry/contract-status.md` records all four as undeployed, IDs `TBD`, Last Verified `Never` |

Nothing here is an on-chain observation. The notice obligations below are
written for the deployment that will exist, and they are the reason to set them
now: a policy written after the first surprise is a policy written by someone
who has already lost an integrator.

## Scope

**In scope:** any change to the shared testnet deployment that makes existing
integrator code **fail, produce different results, or silently misbehave** —
whether delivered by a redeploy under new contract IDs, an in-place
`EphemeralAccount::upgrade()` to a new WASM hash, or an SDF testnet reset.

**Explicitly out of scope:**

- Source-only changes not yet deployed. They belong in the "Unreleased" section
  of `testnet/docs/changelog.md`, which already exists for this.
- Breaking changes in a hypothetical mainnet deployment. A different problem,
  with different consumers and different stakes.
- Non-breaking additions: new functions, new events, new optional arguments,
  new error codes, new enum variants appended at the end. These are logged as
  "Other changes" with no notice period.
- The behaviour of the contracts as designed. See
  [`docs/security.md`](../../docs/security.md); this policy is about
  *deployment events*, not design review.

## What counts as breaking

The test is behavioural, not textual: **would a correct, unmodified client
built against the current deployment still behave correctly after this
change?** If no, it is breaking.

| # | Class | Why it breaks someone | Present in the source today? |
|---|---|---|---|
| 1 | **An argument added to an existing function** | Arity mismatch; the call is rejected | Yes — see the worked example below |
| 2 | **An argument removed** | Arity mismatch, and possibly no way to supply a value that had a default meaning | — |
| 3 | **An argument type changed** | XDR encoding differs; the call is rejected or, worse, is interpreted | — |
| 4 | **Argument order changed** | Silently dangerous: same types, different meaning. The worst class | — |
| 5 | **A function removed or renamed** | `no such function` | — |
| 6 | **A return type changed** | Decoding failure in the client, possibly on an already-successful call | — |
| 7 | **An event name changed** | Indexers silently stop matching. No error anywhere | `created`, `payment`, `multi_pay`, `swept_mul`, `expired`, `reserve`; controller: `sweep`, `dest_auth`, `dest_upd` |
| 8 | **An event payload shape changed** (field added, removed, or reordered) | Decode failure in the indexer; data silently dropped | Same events as above |
| 9 | **An error discriminant changed or reused** | A client matching on numeric codes now misclassifies failures | Enums are 1–15, 1–13 (gap at 12), 1–6 |
| 10 | **An enum variant reordered or renumbered** | Same problem, wider blast radius — `AccountStatus` is persisted in storage and returned by `get_status()` | `AccountStatus` is `0`–`3` |
| 11 | **A new required precondition** (passphrase, network, feature flag, protocol version) | Calls that used to succeed now fail, with an error the integrator has never seen | Documented in the changelog; see the discrepancy note |
| 12 | **A WASM or protocol upgrade changing behaviour** with the same interface | The hardest case: code compiles, tests pass, and the answer is different | `upgrade()` emits no event — see `testnet/registry/upgrade-history.md` |
| 13 | **An environment change** (testnet reset, protocol upgrade, RPC change) | Identical failure mode to 12, with nobody to blame and nobody to ask | See Exemptions |

Class 12 deserves emphasis because it is the one that has already defeated
integrators' detection: `EphemeralAccount::upgrade()` replaces the contract's
code, keeps the contract ID, and **emits no event**. A consumer's address
configuration reveals nothing, and almost nobody reads transaction history
looking for `upgrade` invocations.

### Worked example: the change already sitting in the tree

`testnet/docs/changelog.md`'s "Unreleased" section is a live instance of class
1, and the clearest justification for this policy existing.

**Current, verified from source:**

```rust
pub fn initialize(
    env: Env,
    creator: Address,
    expiry_ledger: u32,
    recovery_address: Address,
    authorized_controller: Address,
    admin: Address,
) -> Result<(), Error>
```

**As the changelog describes the next deployment:**

```rust
pub fn initialize(
    env: Env,
    creator: Address,
    expiry_ledger: u32,
    recovery_address: Address,
    authorized_controller: Address,
    authorized_signer: BytesN<32>,   // new, inserted before admin
    admin: Address,
) -> Result<(), Error>
```

Every existing five-argument `initialize` call **fails after redeployment**,
including every caller inside this repository:
`AccountFactory::batch_initialize` and `::batch_initialize_paginated` both
call `client.try_initialize(&creator, &request.expiry_ledger,
&request.recovery_address, &creator, &creator)` with five arguments. A factory
deployed from a WASM hash that was initialised against the new six-argument
interface will fail on **every** account, and
`batch_initialize` swallows the per-account error (`error: None`), so the
caller learns only that each account failed, never why.

That last sentence is the strongest argument in this document: **the failure
mode of this particular change is silent by construction.** Nobody debugging a
batch failure will be pointed here.

### A second example, and a discrepancy to record

The same "Unreleased" section states that `initialize` will enforce a
network-passphrase check whose expected value on `main` is currently the
Standalone passphrase, which would reject testnet calls with
`Error(Contract, #1007)`.

Two things are worth being precise about:

1. **If that ships as described, it is a class-11 breaking change that is
   completely silent to the integrator** until their calls start failing. It
   would affect *every* call to *every* Bridgelet contract on testnet, not
   just `initialize`. That is the largest blast radius available and it belongs
   at the top of the longer-notice ladder below.
2. **A repository-wide search of `contracts/` and `tools/` finds no
   passphrase check and no occurrence of `1007` or `Standalone` in any `.rs`
   file.** The check is described in the changelog as being on `main`; it is
   not in the source this policy is written against. This is the same class of
   discrepancy as the changelog's error-namespacing claim, described in the
   next section. Do not treat the changelog as evidence that a change is
   implemented — check the source.

`#1007` is also not a discriminant in any of the three error enums, which is
consistent with it being a host/framework-level failure rather than a contract
error. A client matching on contract error codes will not have a case for it.

### Known discrepancy: the changelog asserts an unimplemented error scheme

`testnet/docs/changelog.md` claims, under "Other changes": *"Error codes are
namespaced per contract: EphemeralAccount uses 1000–1999, SweepController
2000–2999, and ReserveContract 3000–3999."*

**The source does not do this.** The three enums on `main` are 1–15, 1–13
(skipping 12) and 1–6 respectively, and there is no offset helper anywhere in
`contracts/shared/`. The same assumption is baked into
[`testnet/docs/support.md`](../docs/support.md), which tells reporters to
expect errors like `Error(Contract, #2003)`.

This matters *here* rather than only as trivia: if the namespacing ever does
land, **every** client matching on error codes breaks at once, and
`SweepController`'s deliberate-looking gap at 12 becomes a trap for a decoder
that assumes contiguity. That is a class-9 change of the largest possible
scope, and it is exactly the kind of thing that needs more than the default
notice period. Until the code says otherwise, the source is right and both
documents are wrong.

## The minimum notice period

> **Default: 7 calendar days between a dated changelog notice and the
> redeploy that implements it.**

This is a proposed number, and it is a commitment the project has to actually
keep — see the honesty note at the end.

### Why 7 days and not 3, and not 14

A consumer's realistic loop is not "notice, edit one line, done". It is:

| Step | Realistic elapsed time |
|---|---|
| Notice is published; the consumer is not watching the changelog daily | up to 2 days |
| Someone notices, triages whether it affects them | 0.5–1 day |
| Change the code, including the six-argument `initialize` call and every derived fixture | 0.5–1 day |
| Run the integration suite against testnet — where the shared deployment is multi-tenant, ledgers close roughly every 5 seconds, and an unrelated party's sweep can move the controller nonce out from under the test | 1–2 days |
| Rehearse, discover a second-order break, fix, re-run | 1 day |
| Merge and release | 0.5 day |

That totals roughly 6–8 days of *work*, before any slack. Three days is not
enough to run an integration suite against a shared deployment, which is the
specific thing a testnet notice period exists to protect. Fourteen days is
defensible for a system with a paying customer base, and is the right number to
propose once there is one — but a project with no committed support
responsibility and a small maintainer population should not promise a
discipline it has not yet had to sustain, because a missed notice teaches
integrators that the changelog is advisory.

**7 days is therefore the number that is both defensible and honourable
today.** It is short enough that a solo maintainer can genuinely keep it, and
long enough to cover one realistic integration cycle. If it is missed, that is a
policy failure to be recorded, not an inconvenience to be quietly absorbed.

### The ladder

| Change | Minimum notice | Rationale |
|---|---|---|
| New function, new event, new optional argument, new error code appended | **None** — log as "Other changes" | Additive; existing clients unaffected |
| Argument added, removed, reordered, or retyped on a public function | **7 days** | Class 1–4; the `initialize` example |
| Function removed or renamed; return type changed | **7 days** | Class 5–6 |
| Event name or payload shape changed | **7 days** | Class 7–8; indexers fail silently, so integrators cannot self-detect |
| Error discriminant or enum ordering changed | **14 days** | Class 9–10; every numeric match in every client is wrong, and failures surface as misclassification rather than an error |
| A new required precondition on all calls (network, protocol, feature gate) | **14 days** | Class 11; the passphrase-check scenario, largest reach |
| Behaviour change with an identical interface | **14 days**, plus a deprecation window if one is possible | Class 12; undetectable by the client |
| Contract IDs change for any reason, including a reset | **Maximum possible**, plus a prominent entry | Nobody's fault, everybody's outage |
| Documented in the source but not deployed | **None** — stays in "Unreleased" | Already handled by the existing changelog process |

**Nothing is exempt from logging.** The exemption is from *waiting*, not from
*writing it down*.

## The notification channel is `testnet/docs/changelog.md`

Not a mailing list, not Discord, not a release tag. One file, already
described by its own header as *"the only file you need to watch"* if you
integrate against testnet. A second channel would split the audience and
guarantee that some integrators see only half the message.

The file already defines its own process — a **"How to update this file"**
section and an entry template. This policy operationalises that process rather
than inventing a parallel one. A breaking entry **must** use the file's
existing template, with all five required fields present:

```markdown
## YYYY-MM-DD — <short title>

Deployed commit: `<sha>`

**Contract IDs:** unchanged | new (see below)
**Breaking changes:** none | …
**Other changes:** …
**Action required:** none | …
```

| Field | Requirement for a breaking entry |
|---|---|
| **Date** (UTC, `YYYY-MM-DD`) | The date of the **deploy**, not the date the notice was written |
| **Deployed commit** | The git commit whose WASM was uploaded |
| **Contract IDs** | New IDs, or "unchanged" — required even when unchanged, so the reader knows it was considered |
| **Breaking changes** | **Not optional.** The old and new signature, verbatim |
| **Other changes** | Additive changes, or "none" |
| **Action required** | **Not optional.** The specific edit an integrator must make |

The "How to update this file" section also requires `network-config.md` and
`deployment-artifacts/contract-ids.txt` to be updated **in the same PR**. This
policy adopts that requirement: a notice that points at contract IDs which have
not been updated elsewhere in the same change is not a complete notice.

Note the file's own trigger list already includes "every time a contract is
upgraded in place". An in-place `upgrade()` is a redeploy with the same ID, and
it owes the same notice.

## What notice looks like concretely

A notice is a dated entry in `testnet/docs/changelog.md`, **published before
the redeploy**, containing all five fields, and containing in its "Breaking
changes" and "Action required" lines:

1. **The old signature, verbatim.**
2. **The new signature, verbatim.**
3. **A migration example** — before and after, in the form a reader can paste.
4. **An explicit action-required line**, not a description of one.

```markdown
## 2026-10-14 — EphemeralAccount::initialize gains authorized_signer

Deployed commit: `<sha>`

**Contract IDs:** new (see below)

**Breaking changes:**

`EphemeralAccount::initialize` gains a sixth argument,
`authorized_signer: BytesN<32>`, between `authorized_controller` and `admin`.
Five-argument calls are rejected after this deployment.

Before:
    initialize(creator, expiry_ledger, recovery_address,
               authorized_controller, admin)

After:
    initialize(creator, expiry_ledger, recovery_address,
               authorized_controller, authorized_signer, admin)

Derive the public key with `tools/sweep-signer pubkey`. It is the same Ed25519
key `SweepController` verifies sweep signatures against, and it is a
**signing-only** key — it is never a funded Stellar account.

**Other changes:** new read-only `is_initialized() -> bool`.

**Action required:** update every `initialize` call to pass
`authorized_signer`. `AccountFactory::batch_initialize` currently passes five
arguments internally and will fail on every account until it is updated;
because it reports `error: None` per account, the symptom is a batch in which
every account fails with no reason given.
```

> **A notice published after the fact is not notice.** It does not discharge
> this policy; it documents a breach of it. The retrospective entry is still
> required — the fix is doing it before next time, not skipping it this time.

## Deprecation: announce, deprecate, warn, remove

Most breaking changes here are not forced. A new function can sit alongside the
old one, and the old one can keep working. That staging is the difference
between an integrator reading a note and an integrator reading a postmortem.

| Stage | What happens | Minimum lead time before the next stage |
|---|---|---|
| **Announce** | Dated changelog entry describing the change, without a date | — |
| **Deprecate** | The old path keeps working; a new path exists and is documented as the replacement | **7 days** before it becomes non-recommended |
| **Warn at runtime** | The old path returns a distinguishable signal when called | **7 days** before removal |
| **Remove** | The old path is gone | **7 days** after the runtime warning |

Rules for staging:

- **Where a change can be staged, it must be.** Adding
  `initialize_v2(env, ..., authorized_signer, ...)` beside the existing
  `initialize` and deprecating the latter is strictly better than replacing
  the signature. It is not always possible — Soroban contract interfaces are
  the deployed WASM's interface, and a new name is a new export, which is a
  code change with its own risk — but "we replaced it" is only acceptable with
  a stated reason why staging was not.
- **The deprecation window has a minimum, not a maximum.** 7 days is the floor
  for the whole ladder. If the old path has to go sooner, that is an exemption
  and goes in the exemptions table below.
- **A runtime warning must be distinguishable.** A warning that is
  indistinguishable from the failure itself is not a warning.
- `SweepController::get_nonce` is a good in-tree precedent for the *shape* of
  this — a deprecation surface that is observable rather than silent. Note that
  the controller's nonce is global to the controller, not per account, so any
  deprecation warning on it is likewise global. There is no per-account
  signalling available in the current interface.

Version tags follow [`versioning-scheme.md`](versioning-scheme.md): `testnet-v1.x`
for the initial MVP deployment, `testnet-v2.x` for multi-asset and governance
modules. A breaking change that cannot be staged should be expected to
correspond to a new major tag, and the tag should be named in the same
changelog entry that gives notice.

## Exemptions

These are real. Dressing a limitation up as a commitment would be worse than
naming it, and an integrator who discovers the exemption by suffering from it
will trust nothing else in this document.

| Exemption | Why it cannot be scheduled | What is still owed |
|---|---|---|
| **An SDF-initiated testnet reset** | The SDF controls the cadence and the announcement. The project learns late or not at all | The **maximum notice the project can give**: a prominent entry in `testnet/docs/changelog.md` as soon as the reset is known, and a follow-up entry with the new contract IDs once redeployed. `testnet/reset/communication-template.md` exists for exactly this message |
| **A security fix where notice would widen exposure** | Publishing "we are upgrading in 3 days" invites preparation of a workaround | Notice **after** the fix, in the same day, as a prominent entry — never silently. Plus an entry in `testnet/registry/upgrade-history.md` |
| **A platform or network change outside project control** | A Soroban protocol upgrade or a `soroban-sdk` requirement arrives on someone else's schedule | Same as a reset: maximum possible notice, prominent entry, and a note in the entry that this was not a project decision |

Every exemption carries the same two obligations, without exception:

1. **A prominent retrospective changelog entry**, dated, explaining what
   changed, when, and that integrators were not given notice.
2. **A record in `testnet/registry/upgrade-history.md`** — date, contract ID,
   old and new WASM hash, reason, admin address, transaction hash. That log
   exists precisely because `upgrade()` emits no event, so it is the only
   record an investigator will find.

**These are the cases integrators will actually be bitten by.** An unannounced
SDF reset is far more likely than a badly-timed feature change, and this
policy does nothing to prevent it. What it does is ensure the outage is
narrated rather than mysterious.

Note that an SDF reset also invalidates every contract ID in
`testnet/docs/network-config.md`, `deployment-artifacts/contract-ids.txt`, and
`deployments/testnet.json` at once — the registry conflict already documented
in `testnet/monitoring/README.md` gets worse with every reset, not better.

## What changed → what notice is owed

| What changed | Notice owed | Where |
|---|---|---|
| Source change, not deployed | None; stays in "Unreleased" | `testnet/docs/changelog.md` |
| Additive change, deployed | Logged as "Other changes" | `testnet/docs/changelog.md` |
| Breaking change, 7-day class | Dated entry, five fields, pre-redeploy | `testnet/docs/changelog.md` |
| Breaking change, 14-day class | Dated entry, five fields, pre-redeploy, plus stated migration path | `testnet/docs/changelog.md` |
| Breaking change staged behind a deprecation | Announce, then deprecate, then warn, then remove — 7 days per stage | `testnet/docs/changelog.md` |
| In-place `upgrade()` | Same notice as a redeploy; plus the upgrade log row | `testnet/docs/changelog.md`, `testnet/registry/upgrade-history.md` |
| Exempt change | Maximum possible notice, prominent retrospective entry, upgrade-log row | `testnet/docs/changelog.md`, `testnet/registry/upgrade-history.md` |
| Contract IDs changed | The ID change itself, in the same PR | `testnet/docs/network-config.md`, `deployment-artifacts/contract-ids.txt` |
| A new deployment record written | Reconcile against the status registry | `deployments/testnet.json`, `testnet/registry/contract-status.md` |

## Accountability and audit trail

| Item | Position |
|---|---|
| **Accountable for publishing notice** | The person running the deploy. The changelog is not a separate task — `scripts/deploy-testnet.sh` is the trigger the changelog's own process names, so notice is part of deploying, not something done afterwards |
| **Accountable for not scheduling around it** | Same role. Choosing *when* to deploy is a governed decision, not an artefact of when a build finished |
| **Consulted** | `testnet/docs/contact-and-ownership.md` names a Testnet Infrastructure Lead address and a Contract Security address. Which address receives a 14-day notice, and who may approve an exemption, is **undecided** — see below |
| **Audit trail** | `testnet/docs/changelog.md` for the notice itself; `testnet/registry/upgrade-history.md` for the code change; `deployments/testnet.json` and `deployment-artifacts/contract-ids.txt` for the resulting IDs; `testnet/registry/last-verified.json` for when someone last confirmed the live state |
| **What proves compliance** | Nothing automated. There is no bot watching for a redeploy without a prior entry. Compliance is a review step, which is a weaker control and should be described as one |

This is the honest position: a notice period enforced by a person remembering
to write an entry is a habit, not a mechanism. The only mechanical control
available today is a review checklist — see
[`testnet/runbooks/upgrade-verification.md`](../runbooks/upgrade-verification.md)
for the post-change verification that a redeploy should also produce.

## What Is Not Decided

Named, not papered over. **None of the following has been decided**, and each
is a prerequisite for this policy being enforceable rather than aspirational.

1. **Who may authorise an exempt change.** "Security fix where notice would
   widen exposure" is a judgement about severity made by a person. That person
   is not identified, and no role in the tree has the authority.
2. **Whether 7 days is the right number, or too short.** It was chosen for
   the reasons above, with no integrator population to validate it against.
   The first real instance should be reviewed and the number revisited.
3. **Whether there will be a deprecation window at all**, given that Soroban
   interfaces are fixed at WASM export time. A new function name is a new
   export; whether the project will accept that cost per change is undecided.
4. **Who monitors for compliance.** Nobody. There is no check that a redeploy
   was preceded by an entry, and `testnet/monitoring/README.md` states plainly
   that no monitoring of any kind is implemented in this repository.
5. **Whether 14 days should be the floor for everything.** It is currently
   proposed only for silent-breakage classes. A stricter reading — that any
   integrator-visible change gets 14 — is defensible and untested.
6. **Whether the exemption list is complete.** It is a list of what we could
   think of, not a taxonomy of all un-schedulable events.
7. **Whether `testnet-v2.x` implies a clean break.** Per
   [`versioning-scheme.md`](versioning-scheme.md), `testnet-v2.x` covers
   multi-asset and governance modules. Whether that is additive or breaking,
   and whether it resets the notice clock, is undecided.

## The honesty note

Two things are true at once and both matter:

**Nothing is deployed.** `testnet/registry/contract-status.md` records all four
contracts as undeployed, with `TBD` IDs and Last Verified `Never`. There is
therefore **no active integrator population** that this policy could protect
today, and no redeploy for it to apply to. The identifiers in
`deployments/testnet.json` and `deployment-artifacts/contract-ids.txt` are
from a `2026-07-12` record that the status registry does not corroborate; treat
them as unverified until someone confirms them per
`testnet/registry/verify-contract-live.md`.

**That cuts both ways, and both halves are the reason to write this now:**

- It is the reason the policy can be set **before** the first break rather than
  in the aftermath of one. A period nobody has yet had to honour is a
  commitment made in calm conditions, which is the only time it is cheap.
- It is the reason its real-world cost is **unvalidated**. Seven days was
  derived from a reasoned model of an integrator's loop, not from observation.
  The first time it is applied, the honest thing to do is record how it went
  and change the number if it was wrong. Writing a period here and never
  revisiting it would be a worse outcome than writing nothing.

## What would count as fixing this

1. **Add a mechanical check that a dated breaking entry exists before a deploy
   is merged.** A CI job that fails when `scripts/deploy-testnet.sh` is
   touched without a new changelog entry. This is the single highest-value
   change and it does not require any contract work.
2. **Resolve the changelog-vs-source discrepancies** documented above — the
   error namespacing claim, the `#1007` passphrase check, and the
   `is_initialized()` addition — so the "Unreleased" section can be trusted as
   a reliable forecast. A notice channel that is routinely wrong about what is
   coming is not a notice channel.
3. **Name the roles** in `testnet/docs/contact-and-ownership.md` that can
   authorise an exemption, and the role that publishes notice.
4. **Test the policy once on a synthetic change** before it matters.
5. **Do not** claim the period is enforced. Until something checks it, this
   document is a commitment and a checklist, not a control.

## Related Documentation

- [`testnet/docs/changelog.md`](../docs/changelog.md) — the notification
  channel, its update process, and the entry template this policy requires
- [`testnet/docs/network-config.md`](../docs/network-config.md) — must be
  updated in the same change as a redeploy
- [`testnet/registry/contract-status.md`](../registry/contract-status.md) —
  current status: all four contracts undeployed
- [`testnet/registry/upgrade-history.md`](../registry/upgrade-history.md) —
  where every in-place `upgrade()` must be recorded
- [`testnet/registry/verify-contract-live.md`](../registry/verify-contract-live.md) —
  the read-only procedure that would turn a stored ID into a verified one
- [`testnet/registry/event-topics.md`](../registry/event-topics.md) — the
  current event names and payload shapes, for the class 7–8 assessment
- [`testnet/reset/communication-template.md`](../reset/communication-template.md) —
  the message for an unschedulable redeployment
- [`testnet/runbooks/upgrade-verification.md`](../runbooks/upgrade-verification.md) —
  post-change verification
- [`testnet/security/admin-key-hygiene.md`](../security/admin-key-hygiene.md) —
  the `upgrade()` blast radius, key custody, and why upgrades must be announced
- [`testnet/security/known-testnet-abuse-patterns.md`](../security/known-testnet-abuse-patterns.md) —
  what an unauthorised upgrade looks like from the outside
- [`testnet/monitoring/README.md`](../monitoring/README.md) — confirms no
  monitoring exists that could detect a compliance failure
- [`testnet/tooling/decision-log.md`](../tooling/decision-log.md) — the tree's
  habit of recording undecided questions as a feature
- [`versioning-scheme.md`](versioning-scheme.md) — the `testnet-v1.x` /
  `testnet-v2.x` tags
- [`data-retention.md`](data-retention.md) — the sibling policy on what may be
  written to the shared deployment at all
- `contracts/ephemeral_account/src/lib.rs`,
  `contracts/sweep_controller/src/lib.rs`, `contracts/shared/src/types.rs`,
  `contracts/*/src/errors.rs` — the source for every signature and error value
  cited here

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-26 | Initial breaking change notice policy; proposes a 7-day default with a 14-day tier and three named exemptions |
