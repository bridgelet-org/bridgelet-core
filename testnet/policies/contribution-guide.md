# Contributing to the `testnet/` Documentation Tree

## Purpose

`testnet/` is a large, purpose-built documentation tree covering the shared
Stellar testnet deployment. It has conventions that are visible in the existing
documents but not written down anywhere, which means a new contributor has to
reverse-engineer them from prose.

This guide records those conventions, states the non-negotiable rules, and
describes how to propose an addition.

## The situation this guide is responding to

Two facts, both verified against the repository:

1. **There is no `CONTRIBUTING.md` anywhere in this repository.** A
   case-insensitive search for any file matching `*contribut*` returns
   nothing. The main `README.md` says so in its own words: *"There is no
   `CONTRIBUTING.md` in this repository at present."*
   `testnet/tooling/decision-log.md` reaches the same conclusion from a
   different direction, listing "the repo has no `CONTRIBUTING.md`" as one of
   its undecided questions.
2. **`.github/` contains two files, both workflows:** `test.yml` and
   `deploy-testnet.yml`. There is **no issue template, no pull request
   template, no `CONTRIBUTORS.md`, and no `PULL_REQUEST_TEMPLATE.md`**.

So there is no repository-level contribution process, and this document does
not create one.

## Scope

**In scope:** proposing additions, corrections, and deletions **within
`testnet/**`**.

**Explicitly out of scope:**

- **A repository-level `CONTRIBUTING.md`.** This document does not replace one
  and is not a start on one. A project-wide guide would need to cover
  `contracts/`, `tools/`, `scripts/`, `docs/`, CI, and review norms — a
  separate and substantially larger piece of work that nobody has scoped.
  Treating this file as that guide would create exactly the confusion it
  exists to prevent.
- **Contract source changes.** Those are normal Rust work; see the main
  `README.md` for building and testing.
- **The contents of `scripts/` and `tools/`.** See the non-negotiable rules.
- **Review policy and code of conduct.** Not decided anywhere in this
  repository, and not invented here.

## What belongs in `testnet/`, and what does not

The tree has a real structure with real boundaries, and one of those boundaries
is already drawn in writing by `testnet/tooling/decision-log.md`. Summarised
here so a new contributor does not have to re-derive it:

> **`testnet/tooling/` holds specifications, not runnable tools. `scripts/`
> holds the repository's own build and CI entrypoints.**
> `scripts/build.sh` builds the contract WASMs, `scripts/deploy-testnet.sh`
> deploys to testnet, `scripts/test.sh` runs `cargo test`. These are
> project-maintenance scripts, versioned in lockstep with the code they build.
> `testnet/tooling/` specs describe tools for *consumers* of an already-deployed
> deployment. Moving runnable code between the two without deciding would be a
> silent architectural change — and nothing has been decided.

The general rule: **if a statement is about one shared testnet deployment, it
belongs in `testnet/`. If it is about how the contracts are designed, it
belongs in `docs/`.** See "Where a fact belongs" below.

### Subdirectory map

Read the `README.md` in a directory before adding to it — several state their
own scope explicitly, and the scope statements differ.

| Directory | What lives there | Has its own index? |
|---|---|---|
| `testnet/config/` | Deployment configuration and identity setup, under a **no-secrets** discipline; read-only, integrator-facing | Yes — `README.md` |
| `testnet/docs/` | Network configuration, getting started, changelog, FAQ, support, ownership | **No** — `overview.md` is the entry point |
| `testnet/examples/` | Integration walkthroughs: create, pay, sweep, recover | Yes — `README.md` |
| `testnet/faucet/` | Funding testnet accounts with Friendbot; rate-limit handling | Yes — `README.md` |
| `testnet/integration/` | Type mapping between Soroban types and SDK representations | **No** |
| `testnet/monitoring/` | Monitoring **plans and procedures** — explicitly not running monitoring | Yes — `README.md` |
| `testnet/policies/` | Process and governance: versioning, data retention, breaking-change notice, this guide | **No** — see open questions |
| `testnet/registry/` | The status and ID source of truth: status tables, event topics, upgrade log, verification procedures | Yes — `README.md` |
| `testnet/reset/` | The announcement template for a redeployment or reset | **No** |
| `testnet/runbooks/` | Step-by-step diagnostics for specific failure symptoms | Yes — `README.md` |
| `testnet/security/` | **Operational** security observations of the shared deployment | Yes — `README.md` |
| `testnet/tooling/` | **Specs for** consumer-side helper tools. Not runnable code | Referenced as an index, but see open questions |

`testnet/` itself has **no top-level `README.md`**. If you add one, that is
welcome, and it is a bigger decision than it looks — see open questions.

### The design/operational pairing

There is a house instinct worth knowing about, because it is easy to trip over:
**a design document is paired with an operational one.** `testnet/security/README.md`
draws the distinction most explicitly, with a table worth copying:

| | `docs/security.md` | `testnet/security/` |
|---|---|---|
| Question | "Is the design sound?" | "What is the shared deployment actually doing, and who can affect it?" |
| Nature | Design-level threat model, static | Operational, observational, time-varying |
| Scope | The contracts as specified | One shared deployment that many unrelated parties use |
| Answers | "What are the mitigations?" | "Someone just did this — is that a bug or an attack?" |

The rule that falls out: `docs/security.md` is not corrected by anything in
`testnet/security/`, and the two document different things. Before adding a
security document, work out which of the two you are writing.

## House conventions

Every existing document in this tree is dated, versioned, cross-referenced, and
explicit about its scope. A new document that follows the shape of an existing
one is easier to review than one that invents a shape.

Imitate these models rather than guessing — they are the highest-quality
documents in the tree:

| Document | What to take from it |
|---|---|
| `testnet/security/README.md` | The scope table, the directory index, the "current state" honesty block |
| `testnet/tooling/nonce-inspector-spec.md` | The `## Status` block opening a spec, and an explicit out-of-scope list |
| `testnet/tooling/decision-log.md` | Recording a decision, and a "What Is Still Undecided" section as a feature |
| `testnet/config/README.md` | A short, dense framing section and a comparison table |
| `testnet/examples/README.md` | Runnable procedures, an `export` preamble, and named placeholders |
| `testnet/registry/contract-status.md` | Canonical status tables, legends, and cross-reference indexes |
| `testnet/docs/changelog.md` | A "how to update this file" process and a strict entry template |

The conventions they share:

- **ATX headings only** (`#`, never setext). Sentence case, descriptive title.
- **A framing section near the top** — `## Purpose`, `## Problem`, `## Overview`,
  `## What this directory is for`, or a `## Status` block. Which one depends on
  the document. A **spec** must open with an explicit
  `**Spec only. Nothing in this document is implemented.**` line and link
  `testnet/tooling/decision-log.md`.
- **A `## Scope` section** with an explicit out-of-scope list, usually a table,
  where the boundaries are not obvious.
- **Tables for anything enumerable** — error codes, options, status, index of
  documents, comparison matrices. This tree leans heavily on tables, and a
  table is almost always the right rendering.
- **Fenced code blocks with a language tag**: `bash`, `rust`, `json`, `text`,
  `javascript`, `sql`, `dotenv`.
- **Relative markdown links when you are linking a document by title**; use
  `` `backticks` `` **when you are naming a path or an identifier** rather than
  linking it. `testnet/registry/contract-status.md` and
  `EphemeralAccount::initialize` are named in backticks; a document you want the
  reader to open is a link.
- **Prose wraps at roughly 78–84 characters.** Match the visual density.
- **A closing `## Related Documentation` bulleted list**, then a `## Version
  History` table with a single row: `| 1.0 | <date> | <one-line summary> |`.
  Use `2026-09-26` unless you have a reason not to.
- **No emojis** beyond the `⚠️` the existing tool output uses. No badges. No
  HTML. No images.

## Non-negotiable rules

These are separated out because violating them is harmful, not merely
inconsistent.

### 1. Never claim something was observed on-chain

**No Bridgelet contracts are currently deployed to Stellar testnet.**
`testnet/registry/contract-status.md` records all four — `EphemeralAccount`,
`SweepController`, `ReserveContract`, `AccountFactory` — as `❌ No`, with
contract IDs `TBD` and Last Verified `Never`.

Therefore:

- You may write what the **code specifies** and what has been **reasoned from
  the source**. Both are fine, and most of this tree is that.
- You may not write "we observed", "confirmed on chain", "this failed in
  testing against the live deployment", or "verified live" — unless you
  actually ran the read-only procedure in
  `testnet/registry/verify-contract-live.md` and recorded the evidence.
- Anything that would need a live check must be **marked as needing one**.

The identifiers in `deployments/testnet.json` and
`deployment-artifacts/contract-ids.txt` are a trap here. They are populated
with IDs from a `2026-07-12` record, and the status registry does not
corroborate them. Cite them, if you must, as unverified.

The same rule in the other direction: where a document would naturally read
like an observation log, **start the log empty with an explicit "no entries
yet" state.** See `testnet/registry/upgrade-history.md` and
`testnet/runbooks/` for how this is done here.

### 2. Never commit or paste a secret

No `S...` Stellar secret keys. No Ed25519 signing seeds. No signatures that
belong to someone else. No `.env` file containing any of those, and not in an
issue body, a PR description, a test fixture, or a code block.

`testnet/config/README.md` establishes the standard for the config tree, and
`testnet/security/README.md` repeats it for observations. It applies to every
document in `testnet/`. Fork PRs are not private: a secret in a PR is
published. And the root `.env.example` contains a commented example seed used
in a `--signer-seed-hex` invocation — it is a documentation artefact, not a
key, and must not be copied anywhere or treated as usable.

See also `testnet/policies/data-retention.md`, which is stricter: it does not
just forbid key material, it forbids associating real people with on-chain
identifiers at all.

### 3. Verify every fact against the source, and prefer the source over another document

**A document in this tree is not evidence about the code.** At least two
claims currently in the tree are wrong, and a contributor who trusted the
document over the code would get both of them wrong:

| Claim | Where | What the source says |
|---|---|---|
| *"Error codes are namespaced per contract: EphemeralAccount 1000–1999, SweepController 2000–2999, ReserveContract 3000–3999"* | `testnet/docs/changelog.md`, "Unreleased" | **Not implemented.** The enums are `1`–`15`, `1`–`13` and `1`–`6`. There is no offset helper in `contracts/shared/`. The same assumption appears in `testnet/docs/support.md`, which tells reporters to expect errors like `#2003` |
| `SweepController`'s errors run contiguously | An easy assumption to make | **They do not.** `InvalidNonce = 11` is followed by `UnauthorizedDestination = 13`. Discriminant **12 is unused**. A client decoding a bare integer must not assume contiguity. Do not "fix" this by renumbering — a value already in a client's switch statement is a compatibility break |

Two more that a careful contributor will run into:

- The changelog's "Unreleased" section describes an `initialize`
  network-passphrase check that would reject testnet calls with
  `Error(Contract, #1007)`, and a new read-only `is_initialized() -> bool`.
  A search of `contracts/` and `tools/` finds **neither**: the only
  `is_initialized` is an internal helper in
  `contracts/ephemeral_account/src/storage.rs`, not a contract function.
  `#1007` appears in no `.rs` file.
- `testnet/docs/network-config.md` states that
  `bridgelet_shared::passphrase::TESTNET_PASSPHRASE` exposes the passphrase
  constant. **There is no `passphrase` module.** `contracts/shared/src/` contains
  `lib.rs` and `types.rs` only. The passphrase to use is in that document's own
  table.

The habit to build: for every code fact you assert — an error discriminant, a
function signature, a flag name, a version, a path, an event symbol, a type
name — **open the source file and read it.** Do not retype from memory, and do
not copy from a sibling document. If a document and the source disagree, the
source is right; say so in your document rather than quietly picking a side.

### 4. Do not edit `scripts/` or `tools/` from a docs change

`scripts/` holds the repository's own build and CI entrypoints. `tools/`
holds `sweep-signer`, a **working CLI** that is documented here and never
modified by documentation work. Both are out of scope for a `testnet/` change.
If a document change reveals a real bug in one of them, raise it as a separate
issue and say so in the PR description.

### 5. Keep additions inside `testnet/`, unless the change genuinely requires otherwise

`docs/`, `contracts/`, and the root `README.md` are someone else's territory in
a change scoped to this tree. Where a `testnet/` document duplicates or
contradicts a `docs/` document, prefer fixing it inside `testnet/` and
cross-referencing. If the change really cannot be made without touching
elsewhere, **say so explicitly in the PR description** and explain why.

## Where a fact belongs

The test: **is this about the contracts' design, or about this one shared
deployment?**

| Statement | Belongs in |
|---|---|
| "Sweep signatures are verified over `SHA256(destination ‖ nonce ‖ controller_id)`" | `docs/SIGNATURE_FORMAT.md` — design |
| "A nonce read on 2026-09-24 at ledger N was 7, and a sweep at ledger M was rejected" | `testnet/config/nonce-tracking.md` or `testnet/runbooks/nonce-desync.md` — this deployment |
| "The design threat model and its mitigations" | `docs/security.md` |
| "Someone upgraded the shared contract and nobody logged it" | `testnet/security/known-testnet-abuse-patterns.md` — operational |

The `docs/security.md` versus `testnet/security/` comparison above is the
worked example. Reuse it.

### The source of truth for contract IDs — a documented conflict

There is a genuine conflict here and it is **not yours to resolve in passing.**

`testnet/config/README.md` states that `deployments/testnet.json` and
`deployment-artifacts/contract-ids.txt` are the source of truth, because they
are produced by the deploy flow. `testnet/registry/contract-status.md` is
described as the canonical status table — and records every contract as
undeployed with `TBD` IDs. `testnet/monitoring/README.md` calls the registry
"internally inconsistent" and instructs readers to treat every stored ID as
unverified until verified live.

**What to do:** record the conflict where you touch it, and say which one you
are relying on and why. Do not silently pick a side, and do not "fix" one
document to match the other without verifying live first — the fix requires
`testnet/registry/verify-contract-live.md` and the deployment is not verified.

## Workflow

This tree was built the same way, and the issues that drove it are still
visible in the repository.

1. **Open an issue before a substantial addition.** Agree the scope before
   writing. Issues 658, 659, and 660 are what produced the three policy
   documents beside this one; the tree grew directory by directory from issues
   of the same shape. A new runbook, a new policy, or a new monitoring plan is
   in scope for this. A one-line correction is not — just make it.
2. **Keep the addition small and focused.** One document, one subject. A pull
   request that adds four documents at once is four reviews wearing a trench
   coat.
3. **Cross-reference siblings; do not duplicate them.** If
   `testnet/registry/event-topics.md` already has the payload struct, link to
   it. Duplicated facts diverge, and then the tree has two wrong answers.
4. **Update the containing directory's index if it has one.**
   `testnet/security/README.md` and `testnet/examples/README.md` both carry an
   index table; `testnet/runbooks/README.md` even documents its own template
   and update procedure. A document in an indexed directory that is not in the
   index is a document nobody finds.
5. **Add the `## Version History` row.** Bump the version, add a dated row, one
   line of summary. Every document in this tree is versioned because the shared
   deployment changes underneath them — an undated, unversioned document cannot
   be reasoned about later.
6. **Say what is undecided.** If you could not settle something, add it to a
   "What Is Not Decided" or "Open questions" section. This tree treats such a
   section as a feature: `testnet/tooling/decision-log.md` has a whole one, and
   inventing false certainty to fill a section is a defect, not thoroughness.
7. **Verify your own work before opening the pull request.**

## Definition of done

A contributor or reviewer can run this list.

### Content

- [ ] Every code fact — signature, error discriminant, event symbol, path,
      version, flag name — was read from the source, not from a sibling
      document or from memory.
- [ ] No sentence claims an on-chain observation, or that a contract was
      verified live, unless it was, with the procedure and date recorded.
- [ ] Anything needing a live check is marked as needing one.
- [ ] No secret, seed, or real personal data anywhere in the file.
- [ ] Every claim about a discrepancy between documents and source says so
      explicitly, rather than picking a side.

### Structure

- [ ] ATX headings, sentence-case title.
- [ ] A framing section near the top; `## Status` for a spec, with the
      `**Spec only. Nothing in this document is implemented.**` line.
- [ ] `## Scope` with an explicit out-of-scope list.
- [ ] Tables for anything enumerable.
- [ ] Fenced code blocks with a language tag.
- [ ] Documents linked by title as relative markdown links; paths and
      identifiers named in backticks.
- [ ] `## Related Documentation` bulleted list.
- [ ] `## Version History` with a `| 1.0 | <date> | <summary> |` row.
- [ ] Prose wrapping at roughly 78–84 characters.

### Fit

- [ ] Correctly classified as design (`docs/`) or this-deployment
      (`testnet/`).
- [ ] Cross-references siblings rather than restating them.
- [ ] Added to the containing directory's index, if that directory has one.
- [ ] No file outside `testnet/` modified — or, if one was, the PR description
      says so and explains why.
- [ ] `scripts/` and `tools/` untouched.

### Honesty

- [ ] Any undecided question is recorded, not invented around.
- [ ] The version history row is dated and says what the document is, not what
      changed in the author's head.

## Not decided

Genuine open questions about this tree. **None of these has been decided**, and
answering any of them is a prerequisite for someone else building on it
confidently.

1. **Should `testnet/` gain a top-level `README.md`?** It has twelve
   subdirectories and no entry point. A contributor arriving cold has no
   reading order. The obvious content already exists in pieces —
   `testnet/docs/overview.md`, `testnet/registry/README.md`, and the twelve
   directory READMEs — but no one has decided what the index should be, or
   whether it duplicates them.
2. **Should `testnet/policies/` gain a `README.md`?** It has none, and the
   sibling directories `testnet/security/`, `testnet/config/`, `testnet/examples/`,
   `testnet/monitoring/`, `testnet/runbooks/`, and `testnet/registry/` all do.
   The asymmetry is probably an oversight rather than a decision, but the fix
   should be agreed rather than assumed. Note that
   `testnet/tooling/README.md` is **referenced by
   `testnet/tooling/decision-log.md` as "the index for this directory" but does
   not exist** — a real broken reference, worth fixing as part of this.
3. **Should the tree gain `testnet/adr/` (architecture decision records)?** The
   tree has exactly one ADR-shaped document,
   `testnet/tooling/decision-log.md`, and it is doing a lot of work in a single
   file. Splitting decisions out has not been proposed or rejected.
4. **Should any of this be generated from source?** Signatures, error enums, and
   `AccountStatus` values are all mechanically derivable from the contracts,
   and all three currently drift by hand. Generation would remove a whole class
   of defect — including the namespacing discrepancy, which would be impossible
   to state falsely. Nothing has been scoped, and a generated tree is a
   different artefact with different maintenance.
5. **Who reviews `testnet/` changes?** No owner, no review policy, and no
   `CODEOWNERS` file. `testnet/docs/contact-and-ownership.md` names a testnet
   infrastructure lead address and a contract security address, but those are
   contact points, not reviewers.
6. **What is the deprecation policy for `testnet/` documents themselves?** See
   `testnet/policies/breaking-change-notice.md`, which governs the deployment
   and explicitly does not cover the documentation. A document here can be
   wrong about the deployment without any notice obligation.
7. **Should the tree be indexed by audience rather than by topic?** The current
   split is by artefact type (`config/`, `runbooks/`, `policies/`). An
   integrator arriving with one question — "how do I sweep" — has to know that
   the answer is in `examples/` and `runbooks/`. The alternative is a curated
   reading order per audience. Not proposed, not rejected.
8. **Should the known document conflicts be resolved as part of this work?**
   The changelog namespacing claim, the `network-config.md` passphrase-module
   reference, and the `deploy-testnet.yml` versus the root README's CI status
   claim are all currently open. Resolving them requires a live deployment to
   verify against, which does not exist.

## Related Documentation

- [`versioning-scheme.md`](versioning-scheme.md) — the sibling policy, and the
  `testnet-v1.x` / `testnet-v2.x` tags
- [`data-retention.md`](data-retention.md) — what may be written to the shared
  deployment at all
- [`breaking-change-notice.md`](breaking-change-notice.md) — notice owed before
  the deployment changes
- [`testnet/security/README.md`](../security/README.md) — the `docs/` versus `testnet/` scope table, and
  the model for a directory index
- [`testnet/config/README.md`](../config/README.md) — the no-secrets standard every document follows
- [`testnet/examples/README.md`](../examples/README.md) — the model for runnable procedures and named
  placeholders
- [`testnet/tooling/decision-log.md`](../tooling/decision-log.md) — the specs-not-tools boundary, and the
  model for a "What Is Still Undecided" section
- [`testnet/tooling/nonce-inspector-spec.md`](../tooling/nonce-inspector-spec.md) — the model for a spec's `## Status`
  block and out-of-scope list
- [`testnet/registry/contract-status.md`](../registry/contract-status.md) — the canonical status table, and the
  reason rule 1 exists
- [`testnet/registry/verify-contract-live.md`](../registry/verify-contract-live.md) — the only procedure that converts
  a stored ID into a verified one
- [`testnet/registry/upgrade-history.md`](../registry/upgrade-history.md) — the model for an intentionally empty
  log
- [`testnet/docs/changelog.md`](../docs/changelog.md) — the "How to update this file" process this tree
  already uses
- [`testnet/runbooks/README.md`](../runbooks/README.md) — the only directory that documents its own
  template and contribution steps
- [`testnet/monitoring/README.md`](../monitoring/README.md) — the model for stating what does *not* exist
- [`testnet/docs/support.md`](../docs/support.md) — where reports go, and the rule against posting key
  material
- [`testnet/docs/contact-and-ownership.md`](../docs/contact-and-ownership.md) — contact points, and the reason
  "who reviews this" is an open question
- [`README.md`](../../README.md) (repository root) — the `CONTRIBUTING.md` statement, and the
  *"Active Development - MVP. Not audited."* status banner

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-26 | Initial contribution guide for the `testnet/` documentation tree |
