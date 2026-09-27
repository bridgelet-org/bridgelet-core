# Testnet Reset and Redeploy Policy

## Status

**The honest answer, stated up front: there is no fixed cadence, no guaranteed
notice, and no named authority. What exists is a changelog and a detection
procedure.**

This document records that situation precisely, and then gives integrators
something they can rely on anyway — which is a set of **signals**, not
promises.

| Question | Answer as of 2026-09-26 |
|---|---|
| Are the contracts deployed? | **No.** All four are undeployed per `testnet/registry/contract-status.md`. Every "current IDs" in the repository is a historical record from a `2026-07-12` deployment, unverified. |
| Is there a testnet reset cadence? | **No cadence is recorded anywhere in this repository.** The SDF resets testnet periodically; the period is not stated. See [Platform-level reset](#1-platform-level-reset-the-sdf-resets-testnet). |
| Is there a notice commitment before a reset? | **No.** See [Advance notice](#advance-notice). |
| Who may authorise a redeploy? | **Undecided.** No document names an owner. See [Undecided](#undecided). |
| Can a redeploy be announced outside this repo? | No. Nothing else is monitored. See [Undecided](#undecided). |

## Two independent kinds of reset

**These are different events with different warning characteristics, and
conflating them is the main way an integrator gets surprised.** A CI pipeline
does not care which one happened; both mean *your stored contract IDs may be
gone*. But they have different causes, different cadences, different remedies,
and only one of them is under any person's control.

| | 1. Platform-level reset | 2. Bridgelet-specific redeploy |
|---|---|---|
| **What** | The Stellar Development Foundation resets testnet itself. Every account and contract on the network goes away. | Someone re-runs `scripts/deploy-testnet.sh`, or an admin performs an in-place contract `upgrade()`. |
| **Who decides** | The SDF. Not this project, not its maintainers, not its issue tracker. | This project. Currently: whoever holds the deployer key, which is not recorded. |
| **Affects other projects on testnet** | Yes, everyone. | No — only Bridgelet's contracts. |
| **Warning you can get** | None in advance. None from this project, because the project is not told first. | Whatever notice the project chooses to give. Currently: a changelog entry, possibly after the fact. |
| **Recovery** | Re-run the deploy script; new IDs; announce them. | Already recovered, from the project's point of view. An integrator's job. |
| **Detectable by** | Ledger going backwards, IDs disappearing — see [`testnet/runbooks/rpc-outage.md`](../runbooks/rpc-outage.md) §6 | Stored IDs failing to resolve; `contract-status.md` disagreeing with reality |
| **Can it happen without a changelog entry?** | **Yes.** Nobody may be watching. | **It has happened** — see the drift example below. |

`testnet/docs/changelog.md` is specified to record both: its "How to update
this file" section requires an entry when `scripts/deploy-testnet.sh` is run
**and** when an SDF testnet reset takes the deployment down. So the changelog is
the right place to record either. Whether the entry actually appears is the
unreliable part, and it is unreliable in different ways for each — see above.

## What is destroyed, and what is not

The practical consequence an integrator needs is concrete.

### Destroyed by a redeploy

| Destroyed | Consequence |
|---|---|
| **All four contract IDs.** `stellar contract deploy` mints a new address every run. | Every ID you stored — in code, in `.env`, in a fixture, in a test — becomes garbage. This is the headline item. |
| **Every `EphemeralAccount` child instance**, because each is a separate contract on testnet. | Recorded payments, nonces, expiry ledgers, terminal status: all gone. |
| **The `SweepController` nonce**, which restarts at `0` after `initialize()`. | Any locally-tracked nonce is now meaningless. A nonce that was high is now low, and vice versa. See [`testnet/runbooks/nonce-desync.md`](../runbooks/nonce-desync.md). |
| **The controller's `authorized_destination` and `authorized_signer`**, unless the redeploy is given the same values. | The signer key is not derived from the chain — `deployments/testnet.json` `config.authorizedSigner` is an input to the deploy. A different value means your signatures will not verify, which looks exactly like a signing bug. |
| **The `ReserveContract` admin and base reserve.** | A redeploy with a different `RESERVE_ADMIN_ADDRESS` leaves the new contract initialised but unconfigured. |

**The one that actually costs people money, in the only currency that matters
here:** if an integrator submitted a **real payment** into a testnet
`EphemeralAccount` — real in the sense that a real `G...` address sent real
testnet XLM and the contract recorded it — **that payment is gone** at a
redeploy, along with the ability to sweep it. The contract is the only place
the record of it lived. There is no recovery path and no counterpart on the
other side.

So: **do not treat a testnet ephemeral account as storage for anything you
would miss.** If you have put a payment into one and you still need it, the
deployment is the only copy, and the deployment is transient. This is the same
posture `testnet/examples/README.md` takes — snippets are integration guides,
not a production SDK, and every contract ID must be verified before use.

### Not destroyed

| Survives | Note |
|---|---|
| The contract source, in `contracts/` | Obviously. |
| The `soroban-sdk` version, via `Cargo.lock` | Recorded in [`testnet/security/dependency-audit-notes.md`](../security/dependency-audit-notes.md). |
| Every document in `testnet/` and `docs/` | Which is the only reason this policy can exist. |
| **The deployer key.** | Per the main `README.md`, the key is what the project holds; a reset does not rotate it. A redeploy, though, gives whoever holds it a fresh set of contracts to deploy — see [`testnet/security/admin-key-hygiene.md`](../security/admin-key-hygiene.md). |
| Testnet itself, its passphrase, and its endpoints | Only the *state* is reset. `Test SDF Network ; September 2015` and `https://soroban-testnet.stellar.org` are stable. |

The key asymmetry: **the source and the documentation are the durable
artifacts; every contract ID and every scrap of on-chain state is not.** Build
your assumptions on the first category.

## Advance notice

**`testnet/docs/changelog.md` is the notification channel.** Its own opening
line makes the claim: *"If you integrate against testnet, this is the only file
you need to watch."* It is specified to carry an entry for every redeploy,
every in-place upgrade, and every SDF reset.

Its reliability, assessed honestly:

| Property | Assessment |
|---|---|
| Is it the single channel? | Yes. Nothing else in the repository is a notification mechanism — no watch, no mailing list, no bot, no CI artifact you can subscribe to. |
| Will a Bridgelet redeploy always be recorded? | **It should.** The specified process requires it and there is one person doing it. It has nevertheless gone unrecorded for other events — see the drift example below. |
| Will an SDF reset always be recorded? | **No.** The project may not be told, may not notice, and may not act before you do. Treat a missing entry as uninformative, not as "no reset happened". |
| Is there any lead time? | **No commitment, and none is possible for an SDF-initiated reset.** A project with a `good-first-issue`-scale maintenance budget cannot promise notice of a decision made elsewhere. Saying otherwise would be a promise nobody can keep. |
| Can you watch it automatically? | Technically, yes — it is a file in a public repository. Nobody has built that, and the repo has no `CONTRIBUTING.md` promising anything. |

**If you need to be told, you have to check.** There is currently no mechanism
by which this project will come to you. That is a real gap, and the honest
response to it is [don't rely on notice](#what-integrators-should-do-instead-of-assuming-permanence)
rather than a promise that notice will improve.

## Detecting that your world is no longer valid

This is the part you can actually rely on, because it does not depend on anyone
telling you anything. Run it as a pre-flight, before a test run that matters.

### Step 1 — Re-verify the IDs, do not trust the copy

`testnet/registry/verify-contract-live.md` is the procedure. A read-only
invocation is enough to prove an ID resolves:

```bash
export NETWORK=testnet
export RPC_URL="https://soroban-testnet.stellar.org"
export READER="<funded-testnet-identity>"

# A controller that answers at all is live.
stellar contract invoke \
  --simulate-only \
  --id "<your-stored-sweep-controller-id>" \
  --network "$NETWORK" \
  --source "$READER" \
  -- \
  get_nonce
```

Then confirm it is the *controller you think it is*, not just a contract at
that address:

```bash
# A freshly redeployed controller is at nonce 0. A live one usually is not.
stellar contract invoke --simulate-only \
  --id "<your-stored-sweep-controller-id>" --network "$NETWORK" \
  --source "$READER" -- get_nonce

# get_info is the real initialization probe; get_status returns Active (0)
# for an uninitialized instance, so it proves nothing.
stellar contract invoke --simulate-only \
  --id "<your-stored-ephemeral-account-id>" --network "$NETWORK" \
  --source "$READER" -- get_info
```

`testnet/registry/verify-contract-live.md` gives the full per-contract
procedure and the expected values for each read-only method.

### Step 2 — Check how stale the record is

`testnet/registry/last-verified.json` carries a timestamp per contract, all
currently `2026-09-24T00:00:00Z`. `testnet/registry/contract-status.md` names a
**30-day** staleness threshold for CI.

Two things to understand about that mechanism, because it is easy to
misread:

- It is a **staleness mechanism, not evidence of liveness.** A recent
  timestamp says someone looked; it does not say the contract is there. All
  four timestamps are recent while every contract is recorded as undeployed.
- **Age is a proxy.** A two-week-old record can still be wrong — see below — and
  a two-month-old record on a network nobody has reset can still be right. Use
  it to decide whether to re-verify, not to decide whether to trust.

### Step 3 — Check whether the network itself moved

A platform-level reset has its own signature, and
[`testnet/runbooks/rpc-outage.md`](../runbooks/rpc-outage.md) §6 sets it out:
treat the network as reset if the latest ledger unexpectedly decreases, old
account or contract IDs disappear, or deployment records no longer resolve.

```bash
curl -fsS "$RPC_URL" -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getLatestLedger"}' |
  jq -r '.result.sequence'
```

If that number is lower than the one your tooling remembers, you are in a new
epoch. Do not replay pre-reset transaction history, do not assume prior
contract storage or nonce values exist, and treat redeployment as a separate
manual operation rather than an outage workaround.

### Worked example: the drift is already in the repository

The situation this policy is about is not hypothetical. It is the current
state of this repository, and it is worth reading as the worked example.

**`testnet/registry/contract-status.md` and `deployments/testnet.json` disagree
today.** The registry records all four contracts as undeployed, IDs `TBD`, Last
Verified `Never`. `deployments/testnet.json` records four concrete `C...` IDs
and a WASM hash from `2026-07-12T10:29:25Z`. `deployment-artifacts/contract-ids.txt`
records the same four IDs. `testnet/docs/network-config.md` publishes the same
four IDs. `testnet/docs/changelog.md` has a dated entry with a deployed commit,
`741aec2`, and the same four IDs.

An integrator who reads `network-config.md` — the file the config
documentation points at for current IDs — gets four contract IDs. An integrator
who reads the canonical status registry gets `TBD`. **One of them is wrong,
and nothing in the repository says which.** `testnet/examples/README.md` says so
directly: *"The canonical status registry currently conflicts with older
deployment artifacts, so a copied ID is not proof of availability."*

That is exactly the failure mode this document is about, one step removed from
a reset: **an ID was recorded once, in several places, and nobody has since
checked whether it still resolves.** A redeploy or a reset produces the same
symptom. The difference is that a redeploy *will* be written down, and this
drift was not.

Two more instances of the same class, for anyone auditing their own setup:
`testnet/docs/network-config.md` states that `deployments/testnet.json` "still
contains `null` placeholders" — it does not; it contains the IDs above. And
`testnet/registry/verify-contract-live.md` heads its `EphemeralAccount` and
`SweepController` sections with "**Expected deployed**: Yes (see
`testnet/registry/contract-status.md`)" — pointing at a document that says the
opposite. Both are documentation defects in files that integrators are told to
trust. Neither is a reset, and both would mislead you exactly as badly.

The generalisable lesson, and the reason steps 1–3 above are a pre-flight
rather than a one-off: **a contract ID in a file is a claim, not a fact. Resolve
it at runtime.**

## Policy for this project

The normative part. What triggers a redeploy, who authorises it, and what must
change in the same commit.

### Triggers

| Trigger | Rationale |
|---|---|
| A contract change that cannot be made in place, or that changes the constructor arguments | `EphemeralAccount::initialize` gaining `authorized_signer` is exactly this case — the "Unreleased" section of `testnet/docs/changelog.md` records that existing five-argument calls will fail after redeployment. |
| The `soroban-sdk` version changes | Per [`testnet/security/dependency-audit-notes.md`](../security/dependency-audit-notes.md), the SDK bump must move `tools/sweep-signer` with it, and that is worth doing deliberately and once. |
| A `testnet` deployment tag is cut | `testnet-v1.x` per [`testnet/policies/versioning-scheme.md`](versioning-scheme.md). No recorded deployment carries one, which makes this trigger currently theoretical. |
| An SDF reset takes the deployment down | Recovery, not a choice. |
| A security fix that cannot wait for a workflow | No pause/timelock exists; see [`testnet/security/admin-key-hygiene.md`](../security/admin-key-hygiene.md). |

A merge to `main` is **not** a trigger. `testnet/docs/faq.md` puts it
correctly: in that narrow sense, contract IDs are *more* stable than on
mainnet, because nothing deploys automatically.

### Authority

**Undecided, and this document does not invent it.** No file in the repository
names who may run `scripts/deploy-testnet.sh` or approve a redeploy. The
closest available statements are the main `README.md`, which places the key
with the project, and `testnet/docs/contact-and-ownership.md`, which lists a
Testnet Infrastructure Lead and a Contract Security Team. Neither is a
deployment-authorisation policy, and the deploy workflow is
`workflow_dispatch`-only with no environment protection.

The one rule that *is* derivable from existing documents, and that this
document adopts: **a redeploy is a state-changing operation on shared
infrastructure and must be treated like one.** The immediate-safety-actions
list in [`testnet/runbooks/reentrancy-suspicion.md`](../runbooks/reentrancy-suspicion.md)
says it directly — do not upgrade or redeploy as an investigation step, because
any emergency mutation needs separate authorisation and a recorded rationale.

### What must change in the same change

`testnet/docs/changelog.md` states this list in its "How to update this file"
section. A redeploy is not complete until **all** of these are true:

| # | Artefact | What must be true afterwards |
|---|---|---|
| 1 | `testnet/docs/changelog.md` | A new dated entry at the top: UTC date, the **deployed commit**, new contract IDs or "unchanged", breaking changes, other changes, and "Action required". |
| 2 | `deployments/testnet.json` | Regenerated by the deploy script, with new IDs and a new `deployedAt`. |
| 3 | `deployment-artifacts/contract-ids.txt` | The four `*_CONTRACT_ID` values updated. |
| 4 | `testnet/docs/network-config.md` | The "Deployed contract IDs" table and the WASM hash row updated. |
| 5 | `testnet/registry/contract-status.md` | Status flipped to deployed, IDs and WASM hashes recorded, per-contract blockers cleared. |
| 6 | `testnet/registry/last-verified.json` | ISO timestamps refreshed **after** someone has actually verified on-chain. |
| 7 | `testnet/registry/wasm-hash-reference.md` | WASM hashes and the commit-to-hash mapping populated. |
| 8 | `testnet/security/dependency-audit-notes.md` | Version chain refreshed, if the SDK or CLI changed. |
| 9 | `testnet/registry/upgrade-history.md` | Only for an in-place `upgrade()`, not for a redeploy — a redeploy is a new ID, not an upgrade. |

### No redeploy is announced only in source

**A redeploy that is not in `testnet/docs/changelog.md` did not properly
happen**, regardless of what is true on-chain or what the deploy script wrote.

The script is the enforcement point, which makes this cheap: make
`deployments/testnet.json` write the commit SHA — which it currently does not
— and the changelog entry has something to reference. The absence of that
field is recorded as a gap in
[`testnet/security/dependency-audit-notes.md`](../security/dependency-audit-notes.md).
`scripts/` is out of scope for this work per
[`testnet/tooling/decision-log.md`](../tooling/decision-log.md); naming the
requirement is the contribution available here.

For an announcement, use the template in
[`testnet/reset/communication-template.md`](../reset/communication-template.md)
rather than writing a new one. Note that its current text publishes updated IDs
to `config/testnet_contracts.json`, **a path that does not exist in this
repository** — fix the template to point at
`deployment-artifacts/contract-ids.txt` and `deployments/testnet.json` before
using it.

## What integrators should do instead of assuming permanence

The most valuable section of this document, because it makes the policy
statement above less load-bearing. Every item here is defensive advice that
holds regardless of what this project decides.

| Do | Because |
|---|---|
| **Resolve contract IDs at runtime, never hard-code them.** Read them from a config file or environment variable that you can update without a code change. | A redeploy changes the ID and nothing else. This is the single highest-value change. `testnet/docs/faq.md` says it: "Keep your IDs in config, not in code." |
| **Re-verify before a test run**, using the steps above, not once at setup. | The drift example shows a repository that disagrees with itself. Your setup ran once. |
| **Make runs idempotent and re-creatable.** Deploy your own `EphemeralAccount` per run rather than reusing a stored one; expect to start from zero. | `testnet/examples/README.md`: "Use a new value for `NEW_ACCOUNT_ID` on each run." A reusable deployment ID and a newly deployed child account are different things. |
| **Never treat testnet as a system of record.** No real user data, no real value, nothing you would have to reconstruct. | The specific point here is that the *record* is the contract, and the contract is transient. |
| **Treat the controller nonce as having no meaningful value across runs.** Re-read `get_nonce()` immediately before signing; serialise sign-and-submit per controller. | The nonce is global to the controller, not per account, and it restarts at `0` on a redeploy. See [`testnet/runbooks/nonce-desync.md`](../runbooks/nonce-desync.md). |
| **Design your CI to fail loudly on a missing contract, and treat that failure as expected, not as a bug.** | A hard-coded ID that stops resolving is the *correct* signal. A test that passes against the wrong contract is worse. |
| **Keep a canary that is cheap to recreate.** A funded testnet identity via Friendbot, regenerated on failure. | Friendbot is not reset-proof for your *identity* either. `testnet/faucet/README.md` is the reference. |
| **Re-read the changelog at the start of an integration, not once at onboarding.** | It is the only notification channel there is, and it is unreliable. |

The design consequence, stated as a policy position: **if every integrator
resolves IDs at runtime and re-verifies before a run, a reset becomes a
scheduled annoyance rather than an incident.** That is a better outcome than a
guarantee of stability that cannot honestly be given, and it is why this
section is longer than the policy section.

## Undecided

Genuinely open. None of this has been decided, and this document does not
decide it by writing it down in prose.

1. **Is there a reset cadence?** No. `testnet/docs/network-config.md`,
   `testnet/docs/support.md`, `testnet/docs/faq.md`, and
   `testnet/examples/README.md` all say the SDF resets testnet periodically.
   **None states a period.** The SDF does not publish a fixed one as far as
   this repository is concerned, and this project has not recorded an observed
   interval. Until someone observes and dates two resets, any number here would
   be invented.
2. **Who may authorise a redeploy?** Not decided. See
   [Authority](#authority).
3. **Is there any notice commitment?** Not decided, and a *lead time* one is
   probably not available for an SDF-initiated reset at this project's
   maintenance scale. The honest commitment available today is the detection
   procedure, which is why that section is normative and this one is not.
4. **Should deployment tags be applied?** `testnet-v1.x` exists in
   [`versioning-scheme.md`](versioning-scheme.md); the `2026-07-12` deployment
   carries no tag. Whether every redeploy gets one, and whether the tag is
   written down anywhere machine-readable, is undecided.
5. **Does the changelog need a machine-readable companion?** An integrator
   could diff a JSON file far more reliably than a Markdown table. Not
   proposed, not built, not promised.
6. **Should the drift in [the worked example](#worked-example-the-drift-is-already-in-the-repository)
   be treated as an incident?** It is currently three documentation defects
   and one registry/artefact disagreement, unrecorded anywhere. Whether it
   gets an entry, an issue, or just gets fixed, is a maintainer decision.

**What would count as fixing this document.** A named deployment authority; a
decision on notice, even a decision that notice is "none beyond the
changelog"; a recorded observed reset interval; a corrected
`communication-template.md`; and a resolution of the ID drift — at which point
this document becomes shorter and more confident, because it would be
describing a mechanism rather than documenting an absence.

---

## Related Documentation

- [`testnet/docs/changelog.md`](../docs/changelog.md) — the notification
  channel, and its "How to update this file" process
- [`testnet/docs/network-config.md`](../docs/network-config.md) — endpoints,
  passphrase, and the published contract IDs
- [`testnet/reset/communication-template.md`](../reset/communication-template.md)
  — the announcement template and post-reset checklist
- [`testnet/registry/contract-status.md`](../registry/contract-status.md) — the
  canonical status table, currently in disagreement with the deployment record
- [`testnet/registry/verify-contract-live.md`](../registry/verify-contract-live.md)
  — the pre-flight procedure from [Step 1](#step-1--re-verify-the-ids-do-not-trust-the-copy)
- [`testnet/registry/last-verified.json`](../registry/last-verified.json) — the
  staleness mechanism
- [`testnet/registry/upgrade-history.md`](../registry/upgrade-history.md) —
  in-place upgrades, which are *not* a redeploy
- [`testnet/runbooks/rpc-outage.md`](../runbooks/rpc-outage.md) — §6, detecting
  a reset during recovery
- [`testnet/runbooks/nonce-desync.md`](../runbooks/nonce-desync.md) — what a
  nonce reset means for a locally-tracked counter
- [`testnet/examples/README.md`](../examples/README.md) — the posture this
  document adopts, and the ID-drift caveat
- [`testnet/security/admin-key-hygiene.md`](../security/admin-key-hygiene.md) —
  who holds the key a redeploy would use
- [`testnet/security/dependency-audit-notes.md`](../security/dependency-audit-notes.md)
  — the version chain a redeploy refreshes
- [`testnet/policies/versioning-scheme.md`](versioning-scheme.md) — deployment
  tags
- [`testnet/tooling/decision-log.md`](../tooling/decision-log.md) — why
  `scripts/` is untouched by this work

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-26 | Initial testnet reset and redeploy policy; no cadence or notice commitment established |
