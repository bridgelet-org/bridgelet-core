# Reentrancy Observation Log

## Status

**No entries. There is nothing to log yet, and that is the correct state of
this document.**

As of the last review, **no Bridgelet contracts are deployed to Stellar
testnet.** `testnet/registry/contract-status.md` records all four contracts as
undeployed, with contract IDs `TBD` and a Last Verified value of `Never`. There
is no live deployment, therefore no live transaction, therefore no
reentrancy-adjacent behaviour to have observed.

This document is written **ahead of** that first observation. It defines a
format and a procedure. It is not a record of anything, and nothing in it
should be read as a claim that a transaction was examined.

`deployments/testnet.json` and `deployment-artifacts/contract-ids.txt` do
contain contract IDs from a 2026-07-12 deployment, and
`testnet/docs/changelog.md` has a matching dated entry. Those are unverified
per `testnet/registry/contract-status.md` and
`testnet/registry/wasm-hash-reference.md`. Until someone confirms them on-chain
per `testnet/registry/verify-contract-live.md`, this log stays empty.

## Why a log rather than a runbook

[`testnet/runbooks/reentrancy-suspicion.md`](../runbooks/reentrancy-suspicion.md)
tells you how to investigate *one* transaction. It does not tell you what the
last twelve investigations concluded, which is the thing that makes a
classification interpretable.

A reentrancy conclusion is a statement about a *rate*: "this is not
reentrancy" only carries information if you also know how many times it
looked like reentrancy and was not. Without a log of negatives, every
investigation starts from zero institutional memory, re-derives the same
reasoning, and reaches a conclusion with no visible base rate.

So: **low bar for logging, high bar for concluding.**

## Append-only discipline

**Entries are never edited and never deleted.** Not to fix a typo, not to
redact, not to remove an embarrassing false alarm.

A wrong conclusion is corrected by **appending a new entry that supersedes
the old one and references it by entry ID.** Both entries stay.

This mirrors `testnet/registry/upgrade-history.md`, which is a log rather than
a state table for the same reason: it exists so a behaviour change can be
correlated to a moment. A log that can be rewritten is not evidence — it is a
narrative, and the value of a narrative is that it can be made to say whatever
is convenient later.

Practical consequences:

| Rule | Reason |
|---|---|
| Corrections are new entries | The wrong conclusion is data. "We were wrong, twice" is worth more than a clean-looking log. |
| False alarms are entries, not omissions | They set the base rate. See [What counts as an entry](#what-counts-as-an-entry). |
| Entry IDs are permanent and never reused | A superseding reference must always resolve. |
| Nothing is redacted after the fact | Redaction is deletion with extra steps. Redact *before* filing. |
| Timestamps are UTC and recorded at investigation time, not filing time | An investigation that took three days is a three-day-long fact. |

## Entry schema

One row per investigated transaction, under
[Observations](#observations). These fields are required; anything optional is
marked so.

| Field | Required | Must contain |
|---|---|---|
| **Entry ID** | yes | `RE-0001`, monotonically increasing, never reused. Also the `supersedes` target. |
| **Observed at (UTC)** | yes | `YYYY-MM-DDTHH:MM:SSZ`. When the transaction was seen, not when the entry was written. |
| **Ledger** | yes | The integer sequence number the transaction was included in. |
| **Contract ID** | yes | Full `C...` Strkey, verbatim. A name is not an ID. |
| **WASM hash** | yes | The contract code hash, or the literal `unknown`. Do not copy a hash from `deployments/testnet.json` and present it as observed — say `unverified` if that is what you have. |
| **Transaction hash** | yes | The `getTransaction` hash. For an observation with no submitted transaction, say `none (read-only probe)`. |
| **Function called** | yes | The exact entry point, e.g. `SweepController::execute_sweep`. `unknown` is an acceptable answer, not a good one. |
| **Why it looked reentrancy-adjacent** | yes | The specific observation that triggered the investigation, in one or two sentences. Not the conclusion. |
| **Classification** | yes | One value from [the vocabulary](#classification-vocabulary). No free text. |
| **Confidence** | yes | `high`, `medium`, or `low`. |
| **Evidence retained** | yes | Where the raw material lives: RPC `getTransaction` response, event query, log excerpt, or `none`. Say `none` plainly if nothing was kept. |
| **Investigated by** | yes | A GitHub handle, or `unassigned`. An unassigned entry is better than a fictional owner. |
| **Deployed build** | no | The commit SHA the investigator believes was live, or `unknown`. The runbook requires this to be recorded as unknown rather than assumed. |
| **Supersedes** | no | The entry ID this one corrects, or `none`. |
| **Links** | no | The runbook sections followed, the issue, the postmortem. |

That is thirteen required fields. It is deliberately more than feels
comfortable, because the runbook's own instruction is to record the deployed
identity as unknown rather than assert that current source behaviour was active
on testnet — and a schema that does not ask for that field will not get it.

## Classification vocabulary

A small, closed set. Five values, and they map onto the three conclusions
[`testnet/runbooks/reentrancy-suspicion.md`](../runbooks/reentrancy-suspicion.md)
prescribes at its "Classify and record" step, which is the authority here.
This log does not introduce a parallel scheme; it names the two sub-cases the
runbook already describes inside "not reentrancy" so they can be counted
separately.

| Value | Runbook term | Use when |
|---|---|---|
| `confirmed-reentrancy` | **Supported reentrancy** | One committed transaction demonstrably re-enters the target while the original invocation frame is still active, and causes an additional unauthorized committed effect. Both halves are required: re-entry alone, without the extra effect, is not this. |
| `not-reentrancy` | **Not reentrancy** | The evidence positively identifies a non-reentrancy cause, but it is not one of the two named below. Normal sequencing, a later direct call, a lifecycle issue. |
| `not-reentrancy:duplicate-or-replay` | **Not reentrancy** (sub-case) | Multiple client submissions with one successful effect, or sequential `AlreadySwept` diagnostics with no second committed sweep. Duplicate relayer submission and replay protection. |
| `not-reentrancy:misconfiguration` | **Not reentrancy** (sub-case) | The cause is a wrong destination, a mismatched `authorized_signer`, a wrong contract ID in a signature, or a different deployed build. `testnet/runbooks/failed-sweep-signature.md` is the first stop. |
| `inconclusive` | **Inconclusive** | Invocation, deployment identity, or state evidence is missing. This is the correct classification when a safe reproduction is not possible — the runbook says to use it. |

`inconclusive` is a real answer, not a failure. It is how a log records "we
could not tell", which is the single most useful thing for the next
investigator.

### The indicators that need more evidence

The runbook's "Indicators that require more evidence" table is the direct
input to the `why it looked reentrancy-adjacent` field. Condensed, so a filer
knows which observations are worth an entry at all:

| Observation | Initial interpretation |
|---|---|
| The target entered recursively in one active invocation tree, with extra committed effects | The only strong reentrancy indicator. `confirmed-reentrancy` is reachable. |
| More transfers than recorded assets | Duplicate transfer, recorded-data issue, loop, or callback. Investigate the invocation tree. |
| `payment` / `multi_pay` after `swept_mul` | A later direct call or a lifecycle issue. Look at the *separate* transaction. |
| Different destination | Authorization or configuration until reentrancy is proven. Almost never `confirmed-reentrancy`. |
| Multiple `reserve` events | `reclaim_reserve()` is repeatable by design. Not sufficient evidence of re-entry. |
| Multiple submissions, one successful hash | Relayer race. Almost never an entry beyond a line in `not-reentrancy:duplicate-or-replay`. |

Two things the runbook states that constrain every entry here: **event order
is supporting evidence, not the call tree**, and **calling a function twice
sequentially is not a reentrancy reproduction.**

## What counts as an entry

**Log the false alarms. All of them.**

The reason is the one from the top of this document: a positive is only
interpretable against a rate, and the rate comes from the negatives. A log
containing forty `not-reentrancy:duplicate-or-replay` entries and one
`confirmed-reentrancy` tells the next investigator something a log containing
only the positive does not. A log with only positives is indistinguishable
from a log written by someone who only writes up successes, which destroys its
own credibility.

Concretely, log an entry when:

- A transaction, event, or error **made you look** — you started a
  reentrancy investigation, however briefly.
- A runbook's indicator table fired, even if you dismissed it in a minute.
- Two parties' activity **interacted** in a way you had to reason about.
- You **were wrong** about something, and the reason you were wrong is worth
  recording.

Do not log an entry for:

- Reading the source. That is [`docs/reentrancy-analysis.md`](../../docs/reentrancy-analysis.md),
  and it is design-level reasoning, not an observation.
- A suspected reentrancy you have not investigated. File it as an issue; the
  entry is for investigated things.
- Routine `AlreadySwept` from your own double-submit during a walkthrough,
  unless it is part of a real race. The bar is low, not zero.

The asymmetry to hold onto: **anything that caused a pause goes in;
not everything that caused a pause deserves a conclusion.**

## Observations

| Entry ID | Observed at (UTC) | Ledger | Contract ID | WASM hash | Transaction hash | Function | Looked reentrancy-adjacent because | Classification | Confidence | Evidence | Investigated by | Build | Supersedes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| — | — | — | — | — | — | — | — | — | — | — | — | — | — |

**No entries. As of 2026-09-26 no Bridgelet contract is deployed to Stellar
testnet** (`testnet/registry/contract-status.md`), so there is no live
transaction to have investigated. The table above is the format, not a
result.

Add the first entry when the deployment is verified live and someone
investigates their first reentrancy-adjacent observation — including, per
[above](#what-counts-as-an-entry), the first false alarm. A first entry that
is a false alarm is a good first entry.

## Entry template

Copy this block verbatim into the table, or use it as the checklist. Every
`required` field from [the schema](#entry-schema) must be filled; `unknown` is
an acceptable value, silence is not.

```markdown
| RE-0000 | {{YYYY-MM-DDTHH:MM:SSZ}} | {{ledger sequence}} | {{full C... Strkey}} | {{code hash or `unknown`}} | {{tx hash or `none (read-only probe)`}} | {{exact entry point}} | {{the observation that triggered the investigation, observation only, no conclusion}} | {{one value from the classification table}} | {{high / medium / low}} | {{where the raw RPC/event/log material is, or `none`}} | {{GitHub handle or `unassigned`}} | {{commit SHA or `unknown`}} | {{entry ID or `none`}} |
```

## Worked example

> ### ⚠️ Example only — not a record
>
> Everything below is **synthetic**. The contract ID, the transaction hash, the
> ledger, the WASM hash, and the destination are all invented placeholders in
> the style of `testnet/examples/README.md`. No such observation has been made;
> none can have been, because no Bridgelet contract is deployed. This block
> exists to show the format, and **must never be copied into the Observations
> table as an entry.**

| Entry ID | Observed at (UTC) | Ledger | Contract ID | WASM hash | Transaction hash | Function | Looked reentrancy-adjacent because | Classification | Confidence | Evidence | Investigated by | Build | Supersedes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| RE-0000 *(example)* | `2026-11-04T14:22:07Z` | `5841201` | `CEXAMPLEEXAMPLEEXAMPLEEXAMPLEEXAMPLEEXAMPLEEXAMPLEEXAMPLEEXA` | `unknown` | `<synthetic-tx-hash-not-a-real-transaction>` | `SweepController::execute_sweep` | Two `token.transfer` operations for the same asset inside one transaction, while only one payment had been recorded. Invocation tree was not retrieved. | `inconclusive` | low | `none` — transaction was not preserved before the ledger was queried | `example-investigator` | `unknown` | `none` |

Three things that example is trying to demonstrate:

1. **`WASM hash: unknown` and `Build: unknown` are normal and correct.** The
   runbook requires the deployed identity to be recorded as unknown rather
   than assumed. Filling these in from `deployments/testnet.json` would be
   fabricating evidence.
2. **`Evidence retained: none` is recorded, not hidden.** An entry that says
   what is missing is worth more than one that implies completeness.
3. **The conclusion is `inconclusive`, not `confirmed-reentrancy`.** More
   transfers than recorded assets is a real indicator, but without the
   invocation tree it does not meet the bar. The `why it looked
   reentrancy-adjacent` field records the observation; the classification
   records only what the evidence supports.

## How to file an entry

**A pull request against `testnet/security/`, appending one row to the
[Observations](#observations) table.** Nothing else changes — the entry is the
whole change.

1. Fill the row from the [template](#entry-template).
2. Check the classification against
   [the vocabulary](#classification-vocabulary) table. It is a closed set.
3. Check the file against the [no-secrets rule](#privacy-and-no-secrets).
4. Open the PR describing the observation. If the observation is **actively
   harmful right now** — tokens moved, or the deployment being degraded — do
   not open a public PR until it has been reported privately. Follow
   [`disclosure-process.md`](disclosure-process.md) first; this log is
   downstream of that decision, not an alternative to it.
5. If the entry is a **superseding** one, reference the original entry ID in
   both the `Supersedes` field and the PR description, and say so in the row.
   Do not touch the original row.
6. If the investigation went past triage, also complete
   [`testnet/runbooks/incident-postmortem-template.md`](../runbooks/incident-postmortem-template.md)
   and link it. The log entry is one line; the postmortem is the narrative.

A PR that adds an entry is a low-barrier, low-risk change. Reviewers should
check the classification and the no-secrets rule, not re-litigate the
investigation.

## Privacy and no-secrets

**Entries are public information, and only public information.** A log that
cannot be pasted into a public PR without a redaction pass does not work.

Permitted, and sufficient for any entry:

- Contract IDs (`C...`), account addresses (`G...`), asset contract IDs
- Transaction hashes
- Ledger sequence numbers
- Decoded event data and its topic symbols
- The public `authorized_signer` value from `deployments/testnet.json`
  (`config.authorizedSigner`) — it is a public key
- Exact error values, including `Error(Contract, #N)`

Forbidden, without exception:

- Secret keys, any `S...` StrKey
- Ed25519 signing seeds, in hex or binary
- **Any signature**, including your own. An `auth_signature` is a live
  authorization artefact; publishing one hands a third party a usable sweep
  until the nonce moves. Refer to it as a hash of the bytes if correlation
  matters, not the bytes.
- `.env` contents, unredacted CLI argument dumps, or anything that brings
  either along
- Anything belonging to another party that is not on-chain public state

This is the same standing rule that governs `testnet/security/` as a whole and
is set out in [`README.md`](README.md) and
[`admin-key-hygiene.md`](admin-key-hygiene.md). It is not restated here as a
substitute for reading them; it is restated because a reentrancy entry is
exactly the kind of document where a raw `auth_signature` would be pasted in
"for completeness".

Blameless wording is the other standing rule. Record what the contracts did and
what the plausible cause was. An entry that names a party without evidence is
noise, and on a shared deployment it is also unfair.

## Retention and the audit log

**Entries are permanent. Do not prune, summarise away, or compress this
history when it gets long.**

The reason is that a reentrancy observation log is not only an operational
triage aid. It is potentially **the evidence base a future audit will be built
on**, and audits want negatives:

- "Reentrancy was investigated N times; here is every case and here is the
  evidence for each" is a materially stronger artefact than a summary.
- An audit that arrives after a real incident will ask what was known and when.
  A log that was pruned cannot answer that.
- A log whose *rate* of inconclusive outcomes is visible is itself a finding:
  it says the evidence-retention discipline is weak, which is a thing an audit
  should know.

Two adjacent documents, deliberately kept distinct:

| Concern | Document |
|---|---|
| Design-level reasoning: why reentrancy is not a viable vector here, and which tests check that | [`docs/reentrancy-analysis.md`](../../docs/reentrancy-analysis.md) |
| Operational observation: what actually happened on a real deployment, and how it was classified | **this file** |

That split is the same one [`testnet/security/README.md`](README.md) draws
between `docs/security.md` and this directory, and it is why a disagreement
between the two is not a contradiction. `docs/reentrancy-analysis.md` can be
right about the design and this log can record a false alarm on a live
transaction; both can be true, and that pair of facts is what makes the design
claim credible.

When an entry is the basis for something larger — a postmortem, a code fix, an
audit finding, a disclosure — **link to it, do not move it.** The entry stays
here, with its own entry ID, and the larger document points at it.

---

## Related Documentation

- [`testnet/runbooks/reentrancy-suspicion.md`](../runbooks/reentrancy-suspicion.md)
  — the investigation procedure and the classification this log records
- [`testnet/runbooks/incident-postmortem-template.md`](../runbooks/incident-postmortem-template.md)
  — the long-form record, for when triage is under way
- [`README.md`](README.md) — index of this directory; the
  public-information-only and blameless rules
- [`disclosure-process.md`](disclosure-process.md) — route a harmful finding
  privately *before* filing an entry for it
- [`known-testnet-abuse-patterns.md`](known-testnet-abuse-patterns.md) — the
  abuse surface these observations are judged against
- [`testnet/registry/event-topics.md`](../registry/event-topics.md) — the event
  symbols the runbook's queries filter on
- [`testnet/registry/upgrade-history.md`](../registry/upgrade-history.md) — the
  append-only model this log follows
- [`testnet/registry/verify-contract-live.md`](../registry/verify-contract-live.md)
  — the pre-flight that establishes a contract ID before it can appear here
- [`docs/reentrancy-analysis.md`](../../docs/reentrancy-analysis.md) —
  design-level reasoning; a different concern

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-26 | Initial reentrancy observation log format; no entries |
