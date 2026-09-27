# Testnet Security Disclosure Process

## Purpose

How to report a security-relevant finding you hit while testing against the
shared Bridgelet testnet deployment.

This is a **lightweight, testnet-appropriate process**, and it is deliberately
one. The main `README.md` status banner reads:

> **Status:** Active Development - MVP. Not audited.

The mainnet-grade process that banner implies — an embargo, a private security
team, a coordinated disclosure window, a CVE process — **does not exist in this
repository, and this document does not create it.** What exists is a short
route from "I found something" to "a maintainer knows about it", plus a
definition of what is and is not in scope.

## Status

| Question | Answer as of 2026-09-26 |
|---|---|
| Is there a live deployment to report against? | **No.** All four contracts are undeployed per `testnet/registry/contract-status.md`. Anything you report today is about the code or the tooling, not about an incident. |
| Is there a named security contact? | `security@bridgelet.org` and `admin@bridgelet.org` are listed in `testnet/docs/contact-and-ownership.md`. Neither is verified as monitored. |
| Is the repository's private vulnerability reporting enabled? | **No.** See [Private reporting](#private-reporting) below — this is a real gap, not an oversight on your part. |
| Is there an SLA? | No. See [Response commitment](#response-commitment). |

## This is not a mainnet disclosure policy

Read the difference before assuming you have the right procedure:

| | This document | A formal mainnet disclosure policy |
|---|---|---|
| Scope | One shared testnet deployment, pre-audit MVP | Mainnet contracts carrying real value |
| Route | A GitHub issue, or an email if a report is harmful in public | A private channel, ideally with an advisory |
| Timing | Whoever reads it next | Coordinated window, embargo agreed up front |
| Credits | Not offered | Normally offered, via an advisory |
| Existence here | **Yes — this file** | **No.** Nothing in this repo establishes one. |

**What would have to be true before a heavier policy is warranted.** Not a
matter of taste; these are the conditions:

1. The contracts are deployed somewhere real funds are at risk.
2. There is an audit in scope, with a defined reportable surface.
3. There is someone who can hold a private conversation and publish an
   advisory, i.e. a maintainer with release rights.
4. There is a body of users for whom disclosure timing actually matters.

None of these hold today. Building an elaborate process now would document an
intention the project has not made. When 1–3 become true, this file should be
replaced, not amended, and the replacement should point here for the history.

## Scope

**In scope:** anything a reporter can demonstrate against the checked-in
source or against the shared deployment that has a security consequence for
someone other than the reporter.

**Explicitly out of scope:**

| Out of scope | Why |
|---|---|
| Security testing against third-party deployments | Not ours to report on. Tell the operator, not us. |
| Denial of service you can only cause with your own testnet account | That is your own account. See `testnet/security/known-testnet-abuse-patterns.md` for what abuse of *others'* state looks like. |
| Anything requiring you to hold someone else's key | Report it, do not do it. If you are asking "how would I use the admin key", the answer is to not. See `testnet/security/admin-key-hygiene.md`. |
| Mainnet, or any non-testnet network | Out of project scope entirely. |
| Feature requests, gas optimisation, SDK ergonomics | Normal issues. No label in particular. |
| "This feels wrong" with nothing reproducible | Still welcome, but label `question` and expect a conversation, not triage. |

## Why "it's only testnet" is not the test

[`testnet/security/README.md`](README.md) argues this at length, and the
argument is worth not repeating. Its three reasons, in short:

1. **The deployment is shared, so it is a real multi-tenant system.** Other
   unrelated parties deploy, sweep through, and configure the same contracts.
   One party's action degrades everyone's experience. That is a property of the
   environment, not of the contracts.
2. **Some compromises are indistinguishable from bugs until you know to look.**
   A sweep failing with a bare `ed25519_verify` trap is frequently another
   party's nonce contention, not a signing bug. A `batch_initialize` returning
   `success: false` for every account may be index-slot exhaustion from an
   earlier caller.
3. **"Only testnet" reasoning has already produced a real weakness.**
   `EphemeralAccount::sweep()` accepts an `auth_signature` argument and
   discards it. Not exploitable for theft, but a live footgun — see
   [`unverified-signature-path-warning.md`](unverified-signature-path-warning.md).

The practical consequence for you as a reporter: a finding that only reproduces
on testnet is still worth reporting **if it degrades the shared deployment for
other integrators, or if it points at a signature-verification or
replay-protection gap.** Those two categories are the ones that would matter on
mainnet tomorrow.

## How to report

### Ordinary findings: a public issue

Title it with the `[testnet]` prefix, per
[`testnet/docs/support.md`](../docs/support.md):

```
[testnet] execute_sweep returned the wrong destination on a fresh account
```

Apply these labels, all of which already exist in this repository:

| Label | When |
|---|---|
| `security` | Always. This is what routes it to the security set. |
| `testnet` | Always. The deployment you observed against. |
| `bug` | The behaviour differs from what the code specifies. |
| `severity:critical` / `severity:high` / `severity:medium` / `severity:low` / `severity:info` | Your assessment. See [Severity](#severity). |
| `area:ephemeral-account` / `area:sweep-controller` / `area:account-factory` / `area:reserve-contract` / `area:cross-contract` | Which contract is involved. |
| `type:threat-model` | The finding is about a missing or wrong analysis, not a live behaviour. |
| `good-first-issue` | The finding is documentation-scale, not code-scale. |

If you cannot set labels yourself, put the same words in the issue body:

```
Labels: security, testnet, bug, severity:high, area:sweep-controller
```

### Actively harmful right now, versus merely suspicious

These need different handling, and the difference is about **public
contribution to the harm**, not about how sure you are.

| Situation | Route | Why |
|---|---|---|
| Tokens moved to a destination the reporter did not intend | [Private reporting](#private-reporting), and say so in a one-line public issue only after a maintainer acknowledges | A public issue that pastes the transaction is a public timeline for whoever did it |
| A griefing vector that degrades the deployment for others right now | [Private reporting](#private-reporting) | Same reason. Also: stopping it may need a maintainer decision, not yours |
| An account is stuck in a terminal state you cannot exit | Public issue is fine | Degraded, not dangerous, and the account is likely yours |
| A signature is rejected and you cannot tell why | Public issue is fine | `testnet/runbooks/failed-sweep-signature.md` probably answers it first |
| You proved a *path* to theft but have not executed it | [Private reporting](#private-reporting) | Do not execute it, even to be sure |
| A working exploit against your own throwaway account | Private, if it needs a detail that helps someone else; public if it does not | Judge by whether the write-up helps or arms |

**Do not** retry the suspect transaction, re-run a suspected exploit, or
upgrade/redeploy anything as part of investigating it. Those are the
"immediate safety actions" in
[`testnet/runbooks/reentrancy-suspicion.md`](../runbooks/reentrancy-suspicion.md)
and they are not optional.

### Private reporting

The intended private route is GitHub's **Report a vulnerability** button on the
repository's Security tab, and
[`testnet/docs/support.md`](../docs/support.md) tells reporters to use it.

**That button is not currently available.** As of 2026-09-26 the repository's
private vulnerability reporting is disabled, and the repository has published
zero security advisories. Two other documents in this tree give the same
instruction, and they are describing a facility that does not exist yet. Until
somebody enables it, treat this section as describing intent, not behaviour.

Practical substitutes, in order of preference:

1. **`security@bridgelet.org`**, from `testnet/docs/contact-and-ownership.md`.
   The most likely to reach a human privately. Whether it is *monitored* is
   not established; treat a non-reply as "nobody is watching", not as "nobody
   cares".
2. **A public issue carrying only the impact, not the mechanism.** Say what
   you observed and what it would enable. Omit the working steps. Then note
   that a private write-up is available. This is strictly worse than a real
   private channel and should be a fallback, not a first choice.
3. **`admin@bridgelet.org`** for anything that needs the key holder rather than
   a security reviewer.

Whatever you do, do not file a *detail-free* public issue and wait. Say which
of the three routes you used.

## What to include

A good report reproduces. The bar is deliberately low, and the shape follows
what [`testnet/runbooks/reentrancy-suspicion.md`](../runbooks/reentrancy-suspicion.md)
captures at investigation time and what
[`testnet/runbooks/incident-postmortem-template.md`](../runbooks/incident-postmortem-template.md)
formalises afterwards.

**Required, or the report cannot be triaged:**

| Field | Why it is the field and not something else |
|---|---|
| **Transaction hash** | The single most useful item. It identifies the exact call; a maintainer can look it up and see everything you saw. |
| **Ledger sequence and UTC time** | A shared deployment is a timeline. "Now" is not a position on it. |
| **Contract ID called** | The full `C...` strkey. Not a name, not "the account". |
| **The exact function called** | `SweepController::execute_sweep`, not "the sweep". Argument names help. |
| **Observed vs. expected** | Two separate statements. "Observed: `Swept` with `swept_to` set to a destination I did not pass. Expected: rejected, or `swept_to` equal to my argument." |
| **Exact error, verbatim** | `Error(Contract, #N)`, the trap text, the CLI output. Not a paraphrase. |
| **Client and version** | `stellar-cli` version, SDK and version, whether Rust or JS. |

**Strongly recommended:**

- The arguments you actually passed, redacted to remove credentials — see
  [the rule below](#what-must-never-be-posted).
- The result of the read-only probes the relevant runbook asks for:
  `get_status`, `get_info`, `get_nonce`, `can_sweep`.
- Whether anyone else touched the same account or controller, if you know.
- Your **severity** estimate and your **reasoning** for it, even if unsure.
- Whether this is reproducible from a clean run, or only in your environment.

**The single question maintainers will ask anyway**, so answer it up front:
*did this affect only your own testnet account, or shared state other people
depend on?*

## What must never be posted publicly

This is a rule, not a caveat. It applies to issue bodies, comments, logs,
attached files, screenshots, JSON payloads, and anything copied out of
`.env`.

**Never post:**

- A Stellar **secret key** — any StrKey beginning with `S` (56 characters,
  base32). Including the value of `SIGNER_SECRET_KEY`.
- An **Ed25519 signing seed** — 32 bytes of hex or binary, in any encoding.
  Including `SWEEP_SIGNING_KEY_SEED` and `AUTHORIZED_SIGNER_SECRET`, which
  `tools/sweep-signer` reads from the environment.
- **Anyone else's signature**, or any unexpired signed payload — including a
  sweep `auth_signature` you received, were sent, or intercepted.
- A **`.env` file**, unredacted, or any paste that brings one along.
- Any credential, token, or API key of any kind.

Testnet key material is not "just testnet". A testnet deployer key is
**authority** over a shared deployment — see
[`admin-key-hygiene.md`](admin-key-hygiene.md) — and a sweep signature you
publish is a signature a third party may be able to submit before you do.
The same no-secrets discipline that governs `testnet/config/` applies here in
full; it is restated here because a security report is precisely the situation
in which people paste a little too much.

**If you have already posted a secret:** edit or delete the post immediately,
then say so in the same thread so a maintainer knows to treat the value as
burned. Do not quietly remove it. Rotating testnet key material is cheap;
assuming nobody noticed is not.

## Severity

The repository already has a five-step label family; use it rather than
inventing a rubric:

| Label | Use for |
|---|---|
| `severity:critical` | Unauthorised loss or state change on the shared deployment; a working authorisation or signature bypass |
| `severity:high` | A griefing vector that degrades the deployment for other integrators; a replay-protection gap; privilege escalation without the admin key |
| `severity:medium` | Something that costs another integrator real time, or a missing guard with no demonstrated exploitation path |
| `severity:low` | Confusing or misleading behaviour with no security consequence |
| `severity:info` | A hardening observation, a doc correction, an abuse pattern that is reasoned about but not observed |

Two cautions:

- **Severity is about the shared deployment, not about your afternoon.** A
  one-line annoyance that is only visible to the person who tripped it is
  usually `severity:low`. The design-level rubric, where a different question
  is asked of a static threat model, lives in
  [`docs/security.md`](../../docs/security.md). Do not derive a third rubric
  here; if neither fits, use the labels and explain yourself in prose.
- **This is a shared, low-stakes surface, so severity labels will be
  recalibrated.** If a maintainer disagrees with your label, that is a
  conversation, not a rejection of the finding.

## Response commitment

Stated plainly, because a vague promise is worse than a small one:

| Commitment | Detail |
|---|---|
| Acknowledgement | Best effort. No response time is guaranteed. `testnet/docs/support.md` already says this, and it is not superseded here. |
| First substantive reply | No promise. This is a volunteer project with no paid maintainers, no on-call rotation, and no `CONTRIBUTING.md` that promises anything. |
| Triage into a code fix | No promise. A finding may be documented and never fixed; that is a legitimate outcome and not a judgement on the reporter. |
| Credit | Not offered. Do not file expecting attribution, a CVE, or a bounty. |
| Confidentiality after a private report | **Not promised.** There is no embargo process to honour one. If that matters to you, that is a legitimate reason to not send a full write-up — see [what would make this heavier](#what-would-count-as-making-this-heavier). |

The last row is the honest cost of a lightweight process, and it is the main
thing a heavier policy would have to buy.

## Contract bug, or operational observation?

The most useful thing this document does is tell you which of two quite
different reports you are filing. The main README's "Not audited" status is
the reason the distinction matters: a **genuine contract bug is a claim about
the code that would still be a bug on mainnet**, and it is the kind of finding
this project most wants and must act on.

| | A contract bug | An operational observation |
|---|---|---|
| The claim is about | The code in `contracts/` | One shared deployment's current behaviour |
| Would it matter on mainnet? | Yes, as-is | Possibly — but the mechanism is environmental |
| Typical example | A signature that is never verified; a missing terminal-state guard; a replay window in the nonce scheme | Someone else's sweep consumed your nonce; a `batch_initialize` batch failing with `error: None`; the deployment responding differently than `network-config.md` says |
| Where it goes | Public issue, `security` + `bug` + `area:*`, severity from the table above. Mention the unverified-signature path and the namespacing claim in `testnet/docs/changelog.md` if they are involved. | Start with the runbooks, not with an issue. `testnet/runbooks/` is the right first stop for all of these. If the runbook does not explain it, file it — still `security` + `testnet`, but say plainly which state belonged to whom. |
| What makes it a bug rather than an observation | It reproduces **from a clean run, with state you created**, and reading the source explains it | It depends on shared state, and reading the source does not explain it |

**How to tell them apart in practice.** Deploy your own `EphemeralAccount`
from the WASM hash and drive it alone, per
[`testnet/examples/README.md`](../examples/README.md). If the behaviour follows
you into a private instance, it is a bug. If it disappears, it was an
observation about shared state.

**Do not upgrade the contract to "check".** The `upgrade()` path is
admin-gated, emits no event, and changes the code for every other integrator —
see [`admin-key-hygiene.md`](admin-key-hygiene.md). Reproduce in your own
instance instead.

Two findings in this tree are already known and are **not** yours to report
again. Both are documented contract bugs that the source confirms:

- `EphemeralAccount::sweep()` accepts `auth_signature: BytesN<64>` and discards
  it, so a direct call succeeds with a zero signature. See
  [`unverified-signature-path-warning.md`](unverified-signature-path-warning.md).
- The **"Unreleased"** section of `testnet/docs/changelog.md` claims error
  codes are namespaced per contract (EphemeralAccount 1000–1999, and so on).
  The source on `main` contains no such namespacing — the enums are 1–15
  (`EphemeralAccount`), 1–11 then 13 (`SweepController`, with a gap at 12), and
  1–6 (`ReserveContract`). A client that trusts the changelog decodes error
  codes wrongly.

If you find either in the wild, report the *consequence* you hit; the
mechanism is already on the record.

## Related Documentation

- [`README.md`](README.md) — index of this directory, and the
  design-versus-observation distinction
- [`known-testnet-abuse-patterns.md`](known-testnet-abuse-patterns.md) — the
  abuse surface a finding is judged against
- [`unverified-signature-path-warning.md`](unverified-signature-path-warning.md)
  — a known contract bug, and the "how to check" walkthrough
- [`admin-key-hygiene.md`](admin-key-hygiene.md) — why testnet key material is
  authority, not "just testnet"
- [`testnet/runbooks/reentrancy-suspicion.md`](../runbooks/reentrancy-suspicion.md)
  — the evidence-first investigation, including the "pause before you publish"
  actions
- [`testnet/runbooks/incident-postmortem-template.md`](../runbooks/incident-postmortem-template.md)
  — the long-form record, once triage is under way
- [`testnet/docs/support.md`](../docs/support.md) — the general-purpose
  reporting route, and the `[testnet]` title prefix
- [`testnet/docs/contact-and-ownership.md`](../docs/contact-and-ownership.md)
  — the contact addresses used above
- [`testnet/config/README.md`](../config/README.md) — the no-secrets
  discipline this document inherits
- [`docs/security.md`](../../docs/security.md) — design-level threat model; a
  different question, a different document

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-26 | Initial testnet security disclosure process |
