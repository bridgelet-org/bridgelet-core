# Testnet Tooling: Index and Scope

## Status

> **`testnet/tooling/` contains specifications, not runnable tools.** Every
> `*-spec.md` file here is **spec only — nothing in it is implemented.** The one
> runnable tool referenced from this directory, `tools/sweep-signer`, lives in a
> separate top-level directory.

That status line is repeated on the first line of every spec so a reader who
opens one directly still sees it. It is not restated inside each document.

The reasoning behind this state — and the list of reasonable-but-wrong ways to
"fix" it — is in [`decision-log.md`](decision-log.md). Read that before
proposing to add, move, or delete anything here.

## What this directory is for

It is the home for **specifications of testnet-facing helper tools that consume
the already-deployed contracts from outside the repository.**

| | `scripts/` | `testnet/tooling/` |
|---|---|---|
| Category | Build and CI entrypoints | Specifications for external consumer tools |
| Runs | In this repo, against this repo's build output | Outside the repo, against a deployment on a network |
| Invoked by | Developers and CI | Integrators, and their own CI |
| Versioned with | The code it builds | Nothing — it has no build step in this repo |
| Contains today | `build.sh`, `deploy-testnet.sh`, `test.sh` | Markdown only |

`scripts/` is not empty, unused, or a candidate for relocation. The full
argument for why the two categories are different is in
[`decision-log.md`](decision-log.md) under "What `scripts/` Is For"; it is
summarised here rather than repeated.

## If the directory looks empty, that is not a bug

A directory named `tooling/` with no code in it reads as a mistake. Three
reactions follow, and all three are wrong for the reasons
[`decision-log.md`](decision-log.md) gives at length:

| Tempting reaction | Why it is wrong |
|---|---|
| Add the missing scripts here | The batch was scoped to specs. Writing the tools is separate, larger work that has not been agreed. |
| Move `scripts/` in here | `scripts/` has a real existing purpose. Relocating it would be a silent architectural decision. |
| Conclude the directory is done | The specs describe tools that do not exist. Anyone expecting to run one will be misled. |

This batch was explicitly scoped to avoid `scripts/`. Nothing in it adds a
runnable tool to the repository.

## Index

| Document | Scope | Read it when |
|---|---|---|
| [`decision-log.md`](decision-log.md) | Why this directory holds specs, and which open questions that leaves | Before adding, moving, or deleting anything here — it explains the three wrong reactions above |
| [`nonce-inspector-spec.md`](nonce-inspector-spec.md) | Read-only CLI that prints `SweepController::get_nonce()` and can check whether a signature you already hold would still verify | Your sweep signature is being rejected with no error code, or you are about to sign one and want to catch a stale nonce before paying a fee |
| [`faucet-helper-spec.md`](faucet-helper-spec.md) | CLI that funds, deploys, and initialises a fresh `EphemeralAccount` in one step, with immutable parameters handled correctly | You are about to hand-type six `initialize` arguments, two of which cannot be fixed afterwards |
| [`contract-id-lookup-spec.md`](contract-id-lookup-spec.md) | CLI that resolves a contract name to its current testnet ID **and reports where that ID came from and how fresh it is** | You are about to copy a `C...` ID out of a file, or your integration tests hard-code an ID that may be stale |
| [`event-tail-spec.md`](event-tail-spec.md) | CLI that streams live contract events to a terminal, `tail -f` style, with a ledger-sequence cursor | You are running a demo or a manual multi-step test and keep re-issuing `stellar contract events` against a start ledger |
| [`sweep-signer-integration-notes.md`](sweep-signer-integration-notes.md) | **Usage notes, not a spec.** How to build and invoke the real `tools/sweep-signer` CLI: its two subcommands, its flags, its env vars, and its exact output | You need an `auth_signature` for `execute_sweep`, and `.env.example` only showed you the `pubkey` subcommand |

The last row is the one document here that is **not** design-only. It
describes a tool that exists and works; the other five describe tools that do
not exist.

## What is genuinely present versus proposed

| Thing | Status | Where it lives |
|---|---|---|
| `tools/sweep-signer` | **Real, working CLI.** Two subcommands, builds and runs today. | `tools/sweep-signer/` — a **separate top-level directory**, not part of `testnet/tooling/` |
| `bridgelet-nonce` (nonce inspector) | Spec only | Would be new. See [`nonce-inspector-spec.md`](nonce-inspector-spec.md) |
| `bridgelet-faucet` | Spec only | Would be new. See [`faucet-helper-spec.md`](faucet-helper-spec.md) |
| Contract-ID lookup tool | Spec only | Would be new. See [`contract-id-lookup-spec.md`](contract-id-lookup-spec.md) |
| Event tailer | Spec only | Would be new. See [`event-tail-spec.md`](event-tail-spec.md) |

`sweep-signer` is a good precedent for how one of these would be built — a
small standalone Rust binary, outside the contract workspace. It is a
**precedent, not a decision**: [`decision-log.md`](decision-log.md) lists
language, distribution, location, and maintenance model as undecided.

Because `tools/sweep-signer/` is a top-level sibling of this directory, a
reader of [`sweep-signer-integration-notes.md`](sweep-signer-integration-notes.md)
should not conclude the tool lives under `testnet/tooling/`. It does not, and
this batch did not move it.

## The sandbox caveat, carried forward

These specs assume the repository has **no local Soroban sandbox**, so every
manual step goes through the live shared testnet. That is why helpers that
smooth over network latency, nonce races, and hand-copied IDs seem worth
building at all.

If a local sandbox is ever added, several of these tools get substantially less
useful: instant ledgers remove the need for careful expiry arithmetic, and a
local deployment removes most of the latency. The event tailer is the clearest
case — a sandbox is its natural home.

**Re-evaluate these specs against a sandbox before implementing any of them.**
Building a nonce inspector to smooth over live-testnet races is a workaround
for a missing sandbox, not a permanent need. This applies to the three newer
specs here as much as to the two that preceded them. `decision-log.md` records
this in "The Dependency That Might Obsolete All of This".

## Proposing a new document here

**What belongs here:** a specification, or usage notes, for a tool that
*consumes* a Bridgelet testnet deployment from outside this repository. Not a
build script, not a deploy script, not a contract.

**What a spec must contain:**

- An explicit `**Spec only. Nothing in this document is implemented.**` line
  near the top, and a link to `decision-log.md`.
- A `## Problem` section naming the concrete failure it removes, not a
  category.
- A `## Scope` section with an explicit out-of-scope list.
- An `## Interface` section: invocation, an options table, human **and** JSON
  output samples, and **distinct exit codes per outcome** so CI can tell
  "retry" from "network down" from "data is stale".
- A `## Non-Goals` table.
- An `## Integration` section showing how a caller actually consumes it.
- An `## Open Questions` section. Where a decision is genuinely outstanding,
  say so; inventing certainty to fill the section is a defect, not thoroughness.

**Conventions:**

- Status goes in the document, once, at the top. It is not repeated per
  section.
- Verification claims are labelled. Distinguish what is confirmed from source
  from what needs an on-chain check. As of the last review, **no Bridgelet
  contracts are deployed to testnet** — see
  [`../security/README.md`](../security/README.md) and
  [`../registry/contract-status.md`](../registry/contract-status.md).
- The no-secrets rule from [`../config/README.md`](../config/README.md) applies.
  No `S...` secret keys, no Ed25519 seeds, no example key material, not even
  labelled as fake.
- Tables for anything enumerable. Fenced code blocks with a language tag.
  Prose wraps at roughly 78–84 characters.
- Close with `## Related Documentation` as a bulleted list, then a
  `## Version History` table with one row per revision.

**Versioning:** these documents follow
[`../policies/versioning-scheme.md`](../policies/versioning-scheme.md) for the
testnet deployment versioning they refer to, and carry their own
`## Version History` table for their own revisions. A document that starts at
`1.0 | 2026-09-26` is the current convention here.

## Related Documentation

- [`decision-log.md`](decision-log.md) — the reasoning behind this directory's current state
- [`../security/README.md`](../security/README.md) — shared-deployment hazards; the read-only discipline
- [`../security/known-testnet-abuse-patterns.md`](../security/known-testnet-abuse-patterns.md) — what interference with shared state looks like
- [`../config/README.md`](../config/README.md) — the no-secrets standard these documents follow
- [`../examples/README.md`](../examples/README.md) — the manual workflows the proposed tools would wrap
- [`../monitoring/README.md`](../monitoring/README.md) — the adjacent, also-unimplemented, monitoring direction
- [`../registry/contract-status.md`](../registry/contract-status.md) — current deployment status
- `scripts/` — repository build/CI scripts; a different category of thing
- `tools/sweep-signer/` — the one real signing tool that exists

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-26 | Index and scope for `testnet/tooling/` |
