# Decision Log: Why Tooling Specs Live in `testnet/tooling/`

## Purpose of this file

This is a record of a decision, so that a future contributor does not "fix" the current state of `testnet/tooling/` by moving runnable tools into it — or by concluding the directory is broken because it contains only Markdown.

The short version:

> **`testnet/tooling/` currently holds specifications, not runnable scripts. That is deliberate for this batch of work, and it is not a claim that this directory is the right long-term home for runnable tools. That question has not been decided.**

Related: `testnet/tooling/README.md` is the index for this directory. This file is the reasoning behind it.

---

## The Decision

The documentation batch that produced this directory was **explicitly scoped to avoid `scripts/`**. Contributors were asked to specify testnet-facing tooling, not to build it, and not to add runnable tooling to the repository's script directory.

The natural place for a spec is next to the index that points at it, so specs landed in `testnet/tooling/`.

## Why This Needs Writing Down

The resulting state — a directory called `tooling/` that contains no tools — looks like an error. There are at least three reasonable-but-wrong reactions:

| Reaction | Why it is wrong |
|---|---|
| "There's no code here, so the tooling was never written — let me add the scripts." | The batch was scoped to specs. Writing the tools is a separate, larger piece of work that has not been scoped or agreed. |
| "The tools clearly belong here, so let me move the existing scripts into `testnet/tooling/`." | Nothing has established this as the right home. `scripts/` has a real existing purpose (see below). Moving files would be a silent architectural decision. |
| "Specs are the deliverable, so this directory is complete." | The specs describe tools that do not exist. Anyone expecting to run them will be misled unless they read the status line at the top of each spec. |

Every spec in this directory therefore opens with an explicit **Status: spec only, nothing implemented** line. That is the mitigation for the current state, and this file is the durable explanation of why it is the way it is.

---

## What `scripts/` Is For

`scripts/` is not empty or unused. It holds the repository's own build and CI entrypoints:

| Script | Purpose |
|---|---|
| `scripts/build.sh` | Builds the contract WASMs |
| `scripts/deploy-testnet.sh` | Deploys contracts to testnet |
| `scripts/test.sh` | Runs `cargo test` across the workspace |

These are **project-maintenance scripts**: they run in the repo, are invoked by developers and CI, and operate on the repository's own build output. They are versioned with the code they build.

`testnet/tooling/` specs describe something categorically different: **helper tools for people and CI suites consuming the already-deployed contracts.** A nonce inspector or a faucet helper is used by integrators against a live deployment. It has no build step, no place in this repo's CI, and no reason to be versioned in lockstep with the contract source.

That distinction — *scripts that build this repo* vs. *tools that consume this repo's deployment* — is why the two are not the same thing, and why moving files between them without deciding would be wrong.

## What Is Still Undecided

Honest inventory of the open questions. **None of these has been decided.**

1. **Do the specified tools get built at all?** Nothing commits to implementing them. They are proposals.
2. **If built, where do they live?** Options include `testnet/tooling/`, a separate top-level `tools/` package (alongside the existing `tools/sweep-signer/`), an entirely separate repository, or distribution as SDK subcommands. This has not been chosen.
3. **What language and distribution model?** The existing `tools/sweep-signer/` is Rust. That is a precedent, not a decision.
4. **Who maintains them?** The repo has no `CONTRIBUTING.md`. Maintenance expectations for integrator-facing tools are undefined.
5. **What is the stability promise?** Whether these are experimental conveniences or something a consuming project's CI may depend on has not been discussed.
6. **Would a local sandbox change the calculus?** See below — it may make some of these tools unnecessary.

Any of these being answered is a prerequisite for moving runnable code into this directory.

---

## What Would Count as "Fixing" This

Legitimate follow-ups, in rough order of value:

1. **Answer the questions above and write the answers here.** This is the real fix. A recorded decision is worth more than any amount of code.
2. **Build one tool as a pilot**, in whatever location the decision names, and see whether the location holds up in practice.
3. **Revisit the specs against a real implementation.** Specs written without code tend to miss things; that is expected and worth revisiting rather than treating as a defect.
4. **Do not** silently relocate existing scripts, and do not treat the absence of code here as a bug to be patched by moving things.

---

## The Dependency That Might Obsolete All of This

Several specs in this directory assume the repo has **no local Soroban sandbox** — so every manual step goes through the live shared testnet, and helpers that smooth that path are worth building.

That is the current state: there is no sandbox entrypoint, and the CI workflows that would have provided one are largely inert (`.github/workflows/test.yml` is entirely commented out; `deploy-testnet.yml` validates the build but has its actual deploy steps commented out). `testnet/config/expiry-ledger-testing.md` mentions `soroban sandbox` as an option, but the repo provides no way to run it.

If a local sandbox is ever added, most of these tools get substantially less useful — instant ledgers remove the need for careful expiry arithmetic, and a local deployment removes most of the latency. **Re-evaluate these specs against a sandbox before implementing them.** Building a nonce inspector to smooth over live-testnet races is a workaround for a missing sandbox, not a permanent need.

This is a real risk to the whole batch, and it is another reason not to treat these documents as settled designs.

---

## Related Documentation

- `testnet/tooling/README.md` — index of the specs in this directory
- `testnet/tooling/nonce-inspector-spec.md` — spec; supports `testnet/config/nonce-tracking.md`
- `testnet/tooling/faucet-helper-spec.md` — spec; builds on `testnet/config/friendbot.md`
- `tools/sweep-signer/` — the one real signing tool that does exist
- `scripts/` — repo build/CI scripts; a different category of thing
- `testnet/config/README.md` — the same read-only/no-secrets discipline applied to `testnet/config/`

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-26 | Initial decision log for `testnet/tooling/` |
