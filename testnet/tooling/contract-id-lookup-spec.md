# Spec: Contract-ID Lookup — Resolve a Contract Name to Its Current Testnet ID

## Status

**Spec only. Nothing in this document is implemented.** See `testnet/tooling/decision-log.md` for why this directory holds specifications rather than runnable scripts.

## Problem

Contract IDs are **hand-copied**. The same four IDs appear in
`deployments/testnet.json`, in `deployment-artifacts/contract-ids.txt`, in
`testnet/docs/network-config.md`, and in the body of at least three example and
runbook documents, and nothing guarantees they still agree — or that any of
them is live.

Meanwhile the tree already refers to this tool as if it exists.
[`nonce-inspector-spec.md`](nonce-inspector-spec.md) offers
`--controller <C...>` and says, in its options table, *"Resolve names via the
contract-ID lookup tool spec."* That is a dangling reference waiting to be
either honoured or removed. This spec is the attempt to honour it.

An integration test suite that hard-codes a `C...` string cannot tell the
difference between "the contract is at this address" and "a file in this
repository once said the contract was at this address." A lookup tool that
resolves a **name** to an **ID plus its provenance** is only worth building if
it closes that gap rather than hiding it.

---

## The hard problem: there is no single trustworthy source of truth today

This is the design constraint. Everything below follows from it.

As of the last review, the repository's four candidate sources disagree, and
none of them is evidence of liveness:

| Source | What it says | What it is |
|---|---|---|
| [`../registry/contract-status.md`](../registry/contract-status.md) | All four contracts **undeployed** (❌ No), IDs `TBD` or `N/A`, Last Verified **Never** | The canonical human registry |
| [`../registry/last-verified.json`](../registry/last-verified.json) | `2026-09-24T00:00:00Z` for `ephemeral_account`, `sweep_controller`, `reserve_contract`, `account_factory` | A **staleness** mechanism, not a liveness record. It carries no status, ledger, transaction, or code-hash evidence |
| `deployments/testnet.json` | Concrete IDs for all four, `deployedAt: 2026-07-12T10:29:25Z` | Machine-readable deployment output the registry does not reflect |
| `deployment-artifacts/contract-ids.txt` | The same four IDs, flat `KEY=VALUE` | A CI artifact duplicating the above |

[`../security/README.md`](../security/README.md) states the rule plainly: treat
those IDs as **unverified** until someone confirms them on-chain per
[`../registry/verify-contract-live.md`](../registry/verify-contract-live.md).
[`../examples/README.md`](../examples/README.md) says the same thing from the
integrator's side: *"the canonical status registry currently conflicts with
older deployment artifacts, so a copied ID is not proof of availability."*

The two registries do not even agree on who is authoritative:
[`../registry/README.md`](../registry/README.md) calls `contract-status.md` the
canonical table, while [`../config/README.md`](../config/README.md) calls
`deployments/testnet.json` and `deployment-artifacts/contract-ids.txt` the
source of truth. A tool cannot resolve a conflict it has not been told how to
resolve, so this spec does not pretend to. It reports the conflict.

> **The governing design constraint: a lookup tool that reads a stale file and
> returns a confident answer is worse than no tool at all.** Without this tool,
> a developer copies an ID out of a file and can see that is what they did.
> With a badly designed tool, they run a command, get clean output, and stop
> asking. The tool must make the answer's weakness more visible than a bare
> string ever was, or it must refuse to answer.

So the tool's output is **provenance and freshness, plus a string** — never
just a string.

---

## Scope

**In scope:** read-only. Resolve a contract name to an ID, and report where
that ID came from, how old the source is, and whether it has been verified.

**Explicitly out of scope:**

- Any write, to any file.
- Submitting transactions or invoking contract methods.
- Caching an ID between runs.
- Deploying, upgrading, or repairing anything.
- Deciding which source is authoritative. That is a human decision, recorded
  in a document, not something a CLI should settle at runtime.

---

## Interface

### Invocation

```bash
bridgelet-contract-id <name> [options]
```

### Options

| Option | Default | Purpose |
|---|---|---|
| `<name>` positional | required | Contract name. See "Name resolution" below. |
| `--json` | off | Machine-readable output, with the same fields as the human form. |
| `--require-verified` | off | Exit non-zero unless the result is verified. Intended for CI. |
| `--probe` | off | Perform a bounded reachability check (see "Sources of truth"). |
| `--rpc-url <URL>` | `https://soroban-testnet.stellar.org` | RPC endpoint, used only by `--probe`. |
| `--max-age <duration>` | `30d` | Age at which a source is reported as stale. Mirrors the staleness threshold in [`../registry/README.md`](../registry/README.md). |
| `--source <name>` | all | Restrict the search to one source, for diagnosis. Never for producing a result to submit with. |

### Output — human

```text
sweep_controller
  id           CBEU…                    (unverified)
  source       deployments/testnet.json (contracts.sweepController)
  deployed_at  2026-07-12T10:29:25Z
  file_age     76d  (stale: exceeds --max-age 30d)
  verified     no   — no on-chain confirmation recorded
  registry     ../registry/contract-status.md says: not deployed, ID TBD,
               Last Verified "Never"
  action       verify per ../registry/verify-contract-live.md before use

  warning: sources disagree. 1 of 2 returned an ID; 0 agree on a verified one.
```

The `registry` and `action` lines are not decoration. They are what makes this
tool better than `grep deployments/testnet.json`, and they are the entire
reason the tool is worth building while the registry conflict is unresolved.

### Output — JSON

```json
{
  "name": "sweep_controller",
  "id": "CBEU...",
  "verified": false,
  "verification": "unconfirmed",
  "sources": [
    {
      "source": "registry/contract-status.md",
      "id": null,
      "status": "undeployed",
      "age_days": 2
    },
    {
      "source": "deployments/testnet.json",
      "field": "contracts.sweepController",
      "id": "CBEU...",
      "deployed_at": "2026-07-12T10:29:25Z",
      "age_days": 76,
      "stale": true
    }
  ],
  "agreement": "conflict",
  "probed": false,
  "probe_result": null,
  "resolve": "no-single-source-of-truth",
  "message": "registry reports not deployed; deployment artefact reports an unverified ID"
}
```

`verified: false` must be **visibly** false in all three surfaces — plain text,
JSON, and exit code. A consumer that only looks at the ID must not be able to
mistake an unverified one for a confirmed one without also seeing the caveat.

### Exit codes

| Code | Meaning | Typical CI reaction |
|---|---|---|
| `0` | Resolved, and **verified** | proceed |
| `1` | Resolved, but **unverified or stale** | proceed only with `--require-verified` off, and log it |
| `2` | Unknown contract name | fail; the name is wrong, or the tool predates a new contract |
| `3` | **Ambiguous or conflicting sources** | fail; a human must reconcile the registry |
| `4` | Source data unreadable or absent | fail; the repository is in an unexpected state |
| `5` | `--probe` could not reach the network | distinct from `1`, so "retry" is distinguishable from "wrong data" |

This split is the same reasoning
[`nonce-inspector-spec.md`](nonce-inspector-spec.md) gives for its own codes:
**CI must tell "the data is stale" from "the network is down" from "the name is
wrong."** Collapsing them into one non-zero exit teaches people to ignore the
failure, and then a genuinely stale ID passes.

**"Refuse to answer" is a first-class outcome, not an error condition.** The
tool must be willing to print no ID at all — with a code of `3` or `4` — and
say why. A tool that never refuses is a tool that is guessing.

---

## Behaviour

### Name resolution

The four contracts are keyed inconsistently across the tree:

| Where | Keys |
|---|---|
| [`../registry/last-verified.json`](../registry/last-verified.json) | `ephemeral_account`, `sweep_controller`, `reserve_contract`, `account_factory` |
| `deployments/testnet.json` | `ephemeralAccount`, `sweepController`, `reserveContract`, `accountFactory` |
| [`../registry/contract-status.md`](../registry/contract-status.md) and [`../registry/event-topics.md`](../registry/event-topics.md) | Title Case, prose |

Specification:

1. **Normalise** the input: lowercase, and treat `_`, `-`, and `.` as
   equivalent separators. So `sweep_controller`, `sweep-controller`,
   `SweepController`, and `sweepcontroller` all normalise to the same token.
2. **Exact match first.** If the input matches a registry key exactly after
   normalisation, use it. Never fuzzy-match.
3. **The canonical name is the registry key.** The `snake_case` form is what
   the tool documents, and it is what `last-verified.json` already uses. The
   camelCase form is accepted on input as a compatibility alias only, because
   that is the spelling `deployments/testnet.json` uses.
4. **Ambiguity gets a defined answer, not a best guess.** If an input
   normalises to more than one known contract — for instance if a future
   contract is named such that `foo` and `foo_bar` collide under aggressive
   normalisation — the tool must exit `3` and list the candidates. It must not
   pick the longest or the first. Guessing which contract someone meant is how
   the wrong ID reaches a transaction.
5. **An unknown name exits `2`** and prints the four canonical names. This is
   the common typo case and the cheapest possible feedback.

### Sources of truth, as an ordered precedence

The precedence below is a **proposal**, and the open question of whether the
registry or the deployment artefact should be authoritative is deliberately
left open (see "Open Questions"). What is not open is the conflict rule.

| Rank | Source | Role |
|---|---|---|
| 1 | [`../registry/contract-status.md`](../registry/contract-status.md) + [`../registry/last-verified.json`](../registry/last-verified.json) | Decides **status** and **verification**. This is the only place a result can become verified. |
| 2 | `deployments/testnet.json`, then `deployment-artifacts/contract-ids.txt` | Supplies a **candidate ID** when the registry has none. Never sufficient to mark anything verified. |
| 3 | Bounded live probe (opt-in, `--probe`) | **Tiebreaker only.** A contract that does not answer is evidence the candidate is wrong; a contract that does answer is evidence it is reachable, not that it is the right contract. |

**The conflict rule, which is the part that must not be left to taste:**

- The tool **must never invent an ID**, and must never construct one, infer
  one, or fall back to a value from a different contract.
- The tool **must never silently prefer one source when two disagree.** If a
  registry status of "not deployed" and a deployment artefact carrying an ID
  disagree, that is `agreement: "conflict"` and exit `3` — not a quiet choice of
  the artefact.
- A single agreeing source yields a result with `verified` set from the
  registry's own record, **not** from the mere presence of an ID.
- Any time the tool cannot state a provenance, it refuses. An answer with no
  provenance is the failure this tool exists to prevent.

### Reporting age

Every result reports the age of the file it came from, computed from that
file's own date field where one exists (`deployedAt` for
`deployments/testnet.json`, the registry's Last Verified column otherwise) and
from the file's modification time only as a fallback. Note plainly that
`last-verified.json`'s timestamps are a *staleness* mechanism: a recent
timestamp means someone looked recently, not that the contract is live, and
the output must not imply otherwise.

---

## Non-Goals

| Not doing | Why |
|---|---|
| Any write path | Keeps the tool incapable of damaging the shared deployment, and incapable of "fixing" the registry by accident. |
| Submitting transactions or invoking methods | It is a lookup. Reachability, if probed at all, is read-only. |
| **Caching an ID** | **A cached ID is the exact failure being fixed.** A cache would make a stale answer look fresh, and would outlive the file it came from. |
| Network calls beyond a bounded health check | Every network call is a latency and availability dependency, and an extra reason for CI to fail for reasons unrelated to the deployment. |
| Being a deployment tool | It resolves names. It does not deploy, upgrade, initialize, or repair. |
| Reconciling the registry automatically | Writing a status table from a tool's own reading of files would launder an unverified ID into an apparently verified one. That reconciliation is a human step, per [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md). |
| Choosing an authoritative source | See "Open Questions". The tool reports the conflict; it does not end it. |

---

## Integration

### In a test harness

The point is a **stable interface** so that test code depends on a name rather
than a copied string:

```bash
export SWEEP_CONTROLLER_ID="$(bridgelet-contract-id sweep_controller --json | jq -r '.id')"
```

The harness should also assert on `verified`, not just on `id`. An ID that
resolves but is unverified is the case that needs a human.

### In CI, fail rather than proceed

```bash
bridgelet-contract-id sweep_controller --require-verified --json
# non-zero exit = the ID is unresolved, unverified, or conflicting; do not proceed
```

This is the mode that earns the tool its place. A CI run that proceeds on an
unverified ID reproduces the hand-copying problem with extra steps; a CI run
that stops makes the registry conflict visible at the moment it matters.

### Alongside the nonce inspector

[`nonce-inspector-spec.md`](nonce-inspector-spec.md) takes a `--controller
<C...>` and points here for name resolution. If both exist, the intended
composition is:

```bash
NONCE=$(bridgelet-nonce \
  --controller "$(bridgelet-contract-id sweep_controller --json | jq -r .id)" \
  --json | jq -r .nonce)
```

The chain only works if the ID it resolves is right, which is why exit code `1`
versus `3` matters upstream: a stale ID produces a plausible nonce read from
the wrong contract, and nothing downstream can detect it.

### With the health check

`--probe` should be built on the same bounded, read-only idea as
[`../registry/verify-contract-live.md`](../registry/verify-contract-live.md) and
the operator procedure in that document. It is a liveness hint for one address,
**not** a substitute for the per-contract method checks that document
prescribes. See [`../monitoring/uptime-check-spec.md`](../monitoring/uptime-check-spec.md)
for the fuller version of the same idea.

### Network values

Endpoints, passphrase, and the RPC URL default come from
[`../docs/network-config.md`](../docs/network-config.md). The passphrase must
match exactly, including the spaces around `;`.

---

## Placement

**This tool must not be placed in `scripts/` by this batch.** The batch that
produced these specs was explicitly scoped to avoid `scripts/`, and
`testnet/tooling/` is where the specification lives. Where a future
implementation would be *built* is one of the open questions in
[`decision-log.md`](decision-log.md) — undecided, and to be decided there
before any code is written.

---

## Open Questions

- **Which source becomes authoritative?** The registry says
  `contract-status.md` is canonical; `testnet/config/README.md` says the
  deployment artefacts are the source of truth. This is the question that
  would resolve most of the tool's difficulty, and it is a documentation
  decision, not a code one.
- **Should the tool ever go to the network at all?** `--probe` is proposed as a
  tiebreaker only. An alternative is to drop network access entirely and be
  purely a provenance reporter, which is simpler and has no availability
  dependency. Not decided.
- **Who updates the registry after a redeploy?** The registry's own update
  process is manual and operator-driven. Whether the deploy flow should write
  `last-verified.json` automatically, and whether a redeploy is enough to
  invalidate a previously verified ID, are undecided. The reset/redeploy
  communication path is in [`../reset/communication-template.md`](../reset/communication-template.md);
  visible changes belong in [`../docs/changelog.md`](../docs/changelog.md).
- **What is the stability promise?** Whether a consuming project's CI may
  depend on the JSON schema — and therefore whether it needs a schema version
  field — has not been discussed. It is the same question
  [`decision-log.md`](decision-log.md) lists for the whole directory.
- **Would a local sandbox collapse this?** Partly. A local deployment with its
  own known, locally-generated IDs removes the "which file is current" problem
  for local work, though not for integrators pointing at shared testnet.
  `decision-log.md` argues this at length; re-evaluate before implementing.

---

## Related Documentation

- [`decision-log.md`](decision-log.md) — why this is a spec and not a script, and the sandbox dependency
- [`nonce-inspector-spec.md`](nonce-inspector-spec.md) — the tool that already points here for name resolution
- [`../registry/contract-status.md`](../registry/contract-status.md) — the canonical status table, currently all-`TBD`
- [`../registry/last-verified.json`](../registry/last-verified.json) — the staleness timestamps
- [`../registry/README.md`](../registry/README.md) — who updates what, and when
- [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md) — the procedure that would produce verification
- [`../security/README.md`](../security/README.md) — the rule that stored IDs are unverified until confirmed
- [`../config/README.md`](../config/README.md) — the competing "source of truth" claim, and the no-secrets rule
- [`../examples/README.md`](../examples/README.md) — the integrator-side statement of the same problem
- [`../docs/network-config.md`](../docs/network-config.md) — endpoints, passphrase, and the published ID list
- [`../monitoring/uptime-check-spec.md`](../monitoring/uptime-check-spec.md) — the broader version of the probe idea
- [`../docs/changelog.md`](../docs/changelog.md) — where a redeploy must be recorded

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-26 | Initial contract-ID lookup spec |
