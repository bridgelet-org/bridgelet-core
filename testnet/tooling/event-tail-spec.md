# Spec: Event Tail — Stream Live Contract Events to a Terminal

## Status

**Spec only. Nothing in this document is implemented.** See `testnet/tooling/decision-log.md` for why this directory holds specifications rather than runnable scripts.

**And, unusually for this directory: the thing this tool exists to watch does
not currently exist.** As of the last review, **no Bridgelet contracts are
deployed to Stellar testnet.**
[`../registry/contract-status.md`](../registry/contract-status.md) records all
four as undeployed with IDs `TBD` and Last Verified `Never`;
[`../security/README.md`](../security/README.md) says to treat the IDs in
`deployments/testnet.json` as unverified until someone confirms them on-chain
per [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md).
There is therefore no live deployment to tail today, and this specification
**cannot currently be exercised**. That is not a reason to drop it — the
ergonomics problem below is real and specified — but it is exactly the
situation `decision-log.md` warns about, and it is why this document should be
read as a proposal, not as a description of something in progress.

## Problem

Watching a contract during a manual test or a demo means correlating events
across the run by hand, and today that is done by re-issuing the same query:

```bash
stellar contract events \
  --contract-id "$NEW_ACCOUNT_ID" \
  --start-ledger "$START_LEDGER" \
  --filter '{"topics":[["created"],["payment"],["multi_pay"],["swept_mul"]]}' \
  --network "$NETWORK"
```

`--start-ledger` is the problem. It is chosen once, before the run, and
remembered. Every subsequent query is a fresh invocation against a
hand-tracked ledger number, and during a live demo the questions are always the
same: *did it just happen?* *which account was that?* *did the controller
sweep, or only the child account?*

This is tedious in a way that is hard to describe to someone who has not done
it, and it is easy to get wrong: too early a start ledger returns a wall of
unrelated output from other integrators using the same shared deployment, and
too late a one silently misses the event you wanted to see. Both mistakes look
identical afterwards, because "no events" is the answer either way.

A `tail -f` over one contract, with a cursor the tool maintains, removes the
hand-tracked ledger entirely.

> This tool is an **ergonomics layer** over the `stellar contract events` shape
> of call documented in [`../examples/README.md`](../examples/README.md) and
> [`../registry/event-topics.md`](../registry/event-topics.md). It is not a new
> mechanism, and it should not invent one.

---

## Scope

**In scope:** read-only streaming. Show events for one contract, in ledger
order, as they are finalized, with a maintained cursor and human-readable
summaries.

**Explicitly out of scope:**

- Writing anything, ever. No submission, no contract invocation, no state
  change.
- Tailing more than one contract at a time in the first version.
- Alerting, thresholds, or notification delivery. That is
  [`../monitoring/`](../monitoring/README.md), and it is a different, larger
  problem.
- Being an indexer. It has no durable storage and no query interface.
- Any claim that absence of an event means anything. See "Absence is not
  evidence" below.

### Read-only, and provably so

This is a design constraint, not a preference, for the same reason it is one in
[`nonce-inspector-spec.md`](nonce-inspector-spec.md). The testnet deployment is
shared, and a tool that *can* mutate a shared deployment is a new abuse
surface. A tailer that watches other people's activity and can act on it would
be exactly that.

So the design must make read-only **provable**, not merely documented:

- The only RPC method in scope is event retrieval (`getEvents` and its
  equivalents), plus a ledger-head read.
- No transaction is ever built, signed, or submitted. Not behind a flag, not in
  a future version, not in `--dry-run`.
- The tool takes no key, no seed, and no secret. If it cannot hold a credential
  it cannot spend one.
- No file in the repository is written. The cursor lives in memory or in a
  caller-supplied path the tool never creates on its own.
- The contract ID it reads is not a target; a filter that does not match is
  information, not an invitation to try another.

The read-only posture in [`../monitoring/uptime-check-spec.md`](../monitoring/uptime-check-spec.md)
— simulate, never submit, including in health probes — is the same rule
applied for the same reason.

---

## Interface

### Invocation

```bash
bridgelet-events tail <contract> [options]
```

`<contract>` is a `C...` contract ID, or a name resolved by
[`contract-id-lookup-spec.md`](contract-id-lookup-spec.md). If a name is given
and resolution yields an **unverified** ID, the tool must say so before
streaming, and must exit `1` unless the operator passes `--allow-unverified`.
Health-check the ID per
[`../registry/verify-contract-live.md`](../registry/verify-contract-live.md)
first; a contract that does not answer is `contract_not_found`, not an empty
event stream.

### Options

| Option | Default | Purpose |
|---|---|---|
| `--event <symbol>` | all | Repeatable filter on `topics[0]`. See "Event symbols" below. |
| `--since <ledger>` | current head | Start ledger. Accepts a ledger sequence, `now`, or a saved cursor. |
| `--cursor-file <path>` | none | Read a cursor from, and write one to, a caller-supplied file. The tool does not create this file's parent. |
| `--duration <n>` | unbounded | Stop after `n` events or after a wall-clock limit. Bounded runs are what belong in CI. |
| `--json` | off | One JSON object per line (`--json` is a stream format, not a single document). |
| `--format <text\|json\|compact>` | `text` | Human output style. |
| `--no-color` | auto | Disable colour. Auto-detects a non-tty, which CI always is. |
| `--follow` / `--no-follow` | `--follow` | Stream, or print a bounded range and exit. |
| `--rpc-url <URL>` | `https://soroban-testnet.stellar.org` | RPC endpoint. |
| `--poll-ms <n>` | one ledger | Poll interval. See the timing note below. |

The RPC default is `https://soroban-testnet.stellar.org`, as in every other
document in this tree.

### Output — human

```text
ledger 17894512  tx 3f2a…c41d  idx 0  created
  creator        GABC…
  expiry_ledger  17895232

ledger 17894513  tx 91be…7a02  idx 0  payment
  amount         1000000   (base units)
  asset          CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC

ledger 17894518  tx c07d…1188  idx 0  sweep
  ephemeral_account  CDEF…
  destination       GABC…
  amount            1000000
```

Three lines of context per event, then the decoded payload, indented. The
header carries ledger, transaction hash, and event index, because those three
are what you need to correlate with anything else and none of them is in the
payload.

An unknown payload field must be **shown, not skipped**. A deployed WASM whose
interface has drifted will carry fields this tool does not know, and a decoder
that silently drops them makes a version change look like a quiet system.

### Output — JSON

One object per line:

```json
{"ledger":17894512,"tx_hash":"3f2a...c41d","event_index":0,"contract_id":"CDEF...","topic_0":"created","symbol":"created","payload":{"creator":"GABC...","expiry_ledger":17895232},"decode":"full","source":"network"}
```

`decode` is one of `full`, `partial`, or `raw`, so a consumer can tell a
complete decode from a degraded one. `source` is `network` or `replay`, and
says whether the record arrived from a live read or from a cursor replay —
which matters for exactly-once reasoning downstream.

### Exit codes

| Code | Meaning | Typical CI reaction |
|---|---|---|
| `0` | Completed cleanly (or `--duration` reached) | proceed |
| `1` | Contract unverified / not found / wrong filter set | fail; do not proceed |
| `2` | Usage error | fail |
| `3` | RPC unreachable, or the connection dropped and could not resume | **retry** |
| `4` | Start ledger older than the RPC retention window | fail; needs a newer start, not a retry |
| `5` | Cursor could not be read, or points into a different reset epoch | fail; needs operator action |

Distinct codes matter here for exactly the reason they matter in
[`nonce-inspector-spec.md`](nonce-inspector-spec.md): **CI must be able to tell
"no events yet" from "the RPC is down."** A tool that exits `0` on a dropped
connection and a tool that exits non-zero on an empty stream teach opposite
habits, and the second one teaches people to ignore non-zero exits. "Zero
events in the window" is a legitimate `0`, and must be visibly different from
"we never reached the network."

---

## Behaviour

### Event symbols — get these exactly right

Topic symbols and payload structs are **not** derived here. The source of
truth is [`../registry/event-topics.md`](../registry/event-topics.md), backed by
`contracts/ephemeral_account/src/events.rs`, the event helpers in
`contracts/sweep_controller/src/lib.rs`, and
`contracts/reserve_contract/src/events.rs`. This section only records what
those say, and flags one documented inconsistency.

Soroban contract events carry an array of `ScVal` **topics** whose **first
element is the event-name symbol**; that first topic is the primary filter. The
payload is a contracttype-serialized `ScVal` in `data`. The Rust struct name is
**not** the on-chain symbol: `PaymentReceived` is published as `payment`, and
`SweepExecutedMulti` as `swept_mul`.

| Contract | `topics[0]` symbols | Payload struct |
|---|---|---|
| `EphemeralAccount` | `created`, `payment`, `multi_pay`, `swept_mul`, `expired`, `reserve` | `AccountCreated`, `PaymentReceived`, `MultiPaymentReceived`, `SweepExecutedMulti`, `AccountExpired`, `ReserveReclaimed` |
| `SweepController` | `sweep`, `dest_auth`, `dest_upd` | `SweepCompleted`, `DestinationAuthorized`, `DestinationUpdated` |
| `AccountFactory` | *none* — the current source emits no events directly | — |

> **Inconsistency to carry, not resolve.** For `ReserveContract`,
> [`../registry/event-topics.md`](../registry/event-topics.md) lists
> `initialized` and `base_reserve_updated`, but the current source in
> `contracts/reserve_contract/src/events.rs` publishes **`init`** and
> **`reserve`**.
> [`../monitoring/event-watchlist.md`](../monitoring/event-watchlist.md)
> records the same discrepancy and says some registry prose predates those
> symbols. For this tool, the **source is the authority** for the default
> symbol list, and `testnet/registry/event-topics.md` should be corrected
> separately. `ReserveContract` is optional and not wired into
> `EphemeralAccount` by the checked-in deployment tooling, so this is not on
> the critical path. A filter naming a symbol the deployed WASM does not emit
> matches nothing, silently — which is the next section.

### "No output" must never be ambiguous

This is the failure mode that makes a tailer worse than a manual command, and
it deserves its own section because it is a design requirement, not a nicety.

A wrong symbol, a filter that excludes everything, a contract that was never
deployed, an RPC that is unreachable, and a genuinely quiet contract all
present as **an empty terminal**. A tailer that just sits there is
indistinguishable from a broken one, and the operator's only way to tell them
apart is to stop it and check something else — at which point the tool has added
a step instead of removing one.

So the tool must distinguish them, and say which one it is:

| State | How the tool reports it |
|---|---|
| Connected, filter valid, no matching events | Explicit heartbeat line: last ledger seen, events matched `0`. Silence is never ambiguous |
| Filter names a symbol the source does not define | Warn **before** streaming, listing the known symbols, and exit `1` |
| Filter is valid but matches no known contract's event set | Warn, do not silently stream nothing |
| RPC unreachable or dropped | Say so, immediately, and exit `3` — never as a quiet stall |
| Contract not found / unverified | Say so, and point at [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md) |
| Connected but not yet finalized | Say "waiting for ledger N", not nothing |

The heartbeat is the key mechanism. **A connected tool must produce output on a
fixed interval even when no event matches.** For a demo operator, the absence
of a heartbeat is itself the signal, and it is the only signal that reliably
distinguishes "quiet" from "broken" at a glance.

The tool must also **validate filters against the source symbol list at
startup**, not discover the mismatch by watching nothing happen.

### Cursor and paging

This is the interesting design problem, and getting it wrong produces the two
failure modes that are hardest to notice: a gap (an event never shown) and a
duplicate (an event shown twice, which looks like a double sweep).

**The cursor is a ledger sequence, and it must be.** On Stellar, a ledger
sequence is a total order over finalized state; it is the only cursor that is
both gapless and unambiguous. A timestamp is not a safe cursor here:

- `Payment.timestamp` and the ledger close time are not the same thing, and
  neither is guaranteed monotonic in the way a cursor must be.
- Testnet ledgers close roughly every five seconds, but the sequence returned
  by the RPC is authoritative — never derive a cursor from a clock. The same
  rule that governs `expiry_ledger` arithmetic, for the same reason.
- A testnet reset **decreases** the ledger sequence. A timestamp-derived cursor
  would sail straight past a reset and show nothing at all.

So: the cursor is an integer ledger sequence, plus enough identity to notice a
reset. Following
[`../monitoring/event-watchlist.md`](../monitoring/event-watchlist.md), a
durable record should retain at least network, reset epoch, contract ID,
`topics[0]`, ledger, transaction hash, event index, and payload.

**Resume semantics.** Resume from the last **finalized** ledger the tool has
fully processed, plus one. Never from the last event seen: if the process dies
mid-ledger, the remainder of that ledger is silently lost.

**Duplicates across a reconnect.** A reconnect re-requests the last ledger
range, and events at the boundary will arrive twice. Deduplicate on
`(reset_epoch, ledger, tx_hash, contract_id, event_index)` — the same tuple
`event-watchlist.md` specifies. This makes the tool's behaviour
**at-least-once on the wire, exactly-once on screen**, and the tool should
state that rather than implying stronger delivery than it has.

**Retention.** If `--since` is older than the RPC's history window, the range
is gone. That is exit `4`, with a message naming the oldest ledger still
available. It must not degrade silently into "no events" — that is precisely
the ambiguity this tool exists to remove.

**Testnet reset.** If the observed head ledger is **lower** than the cursor,
that is a reset, not a lag. Declare a new reset epoch, quarantine the old
cursor, and require re-verification of the contract ID. Exit `5`. See
[`../monitoring/uptime-check-spec.md`](../monitoring/uptime-check-spec.md) for
the reset-handling sequence, which this tool should follow rather than invent.

**Paging limits.** A bounded ledger range per request, with the cursor
advanced only after the full range has been processed. Poll interval should be
one ledger; polling faster re-reads the same finalized ledger and produces
false reassurance, which is the streaming equivalent of the nonce-inspector's
"too short a sample interval" problem.

### Absence is not evidence

A tailer watching a shared deployment is a tool for watching **other people's
activity** as well as your own. `event-watchlist.md` is unambiguous that
absence is meaningful only when a specific committed operation should have
produced the event, and that an idle window's honest baseline is zero events
without that implying a fault.

The tool must not print anything resembling "all good" on an empty window, and
must not colour output by "expected" versus "unexpected" — it does not have
the information to know. It reports what it saw and nothing more.

---

## A shared deployment, honestly

The testnet deployment is **shared**. Read
[`../security/README.md`](../security/README.md) and
[`../security/known-testnet-abuse-patterns.md`](../security/known-testnet-abuse-patterns.md)
first; they are the references, and this section only applies them to a
streamer.

Someone else's activity on the same contracts is a **normal operating
condition**, not an anomaly. During a demo, the accounts in the stream will
usually not be yours.

Two requirements follow:

1. **Attribute output to the emitting contract, and show the contract ID
   prominently.** If the tool can tail more than one source over its life, a
   bare `sweep` line means nothing. Every rendered event carries the emitting
   contract ID.
2. **Never imply causation.** The tool observes; it does not know who submitted
   a transaction, and on a permissionless network it often cannot tell. Do not
   label events as "mine" or "someone else's", do not infer a party from an
   address, and do not describe a transition as something the operator caused.

Keep the blameless tone the rest of the tree uses: observations, not
accusations. A recorded observation should be checkable by someone else from
the ledger, transaction hash, and event index printed alongside it.

Also relevant: a stream of someone else's sweeps is exactly the signal that
explains a sudden nonce jump. The tool should make that connection available —
printing the current ledger and the last `sweep` event — without diagnosing the
operator's signature failure for them. See
[`../config/nonce-tracking.md`](../config/nonce-tracking.md) and
[`../runbooks/nonce-desync.md`](../runbooks/nonce-desync.md).

---

## Demo and session ergonomics

This tool exists for a terminal in front of people, which the other specs in
this directory are not designed for.

| Need | Behaviour |
|---|---|
| Clear screen on start | Yes by default, so a demo does not open on stale scrollback. `--no-clear` to disable |
| Bounded scrollback | Keep the last N lines (default 200) in the human view. A tail that scrolls a terminal into uselessness is worse than none |
| Colour | Auto-disabled when not a tty, so CI logs stay readable. `--no-color` to force |
| One-line summaries | The default human format is one line per event with a compact payload, plus indented detail. Both, because the summary is for the demo and the detail is for the person who missed something |
| `--json` | One object per line, for piping into `jq` or a file. Machine consumption must never require parsing the pretty format |
| Timestamps | Show ledger sequence first and wall time second. **The ledger is the identity; the clock is decoration** |
| Filter recap | Print the resolved filter and start ledger once at startup, then stop talking |

A demo operator should be able to start the tailer, run the walkthrough, and
read the stream without touching the tool again. That is the whole bar.

---

## Non-Goals

| Not doing | Why |
|---|---|
| Any write path, or holding a credential | Keeps the tool structurally incapable of affecting the shared deployment |
| Multi-contract tailing in v1 | Multi-source streaming makes attribution ambiguous, and attribution is the point |
| Durable storage, indexing, or querying | That is an indexer, and a different piece of work |
| Alerting, thresholds, or notifications | [`../monitoring/`](../monitoring/README.md) territory. Mixing a demo tool with an alerting path produces a tool nobody trusts for either |
| Inferring meaning from absence | See "Absence is not evidence". Event-watchlist owns that policy |
| Re-implementing event decoding from the contract source | [`../registry/event-topics.md`](../registry/event-topics.md) is the source of truth; the deployed ABI is the final authority |
| Any on-chain claim | Nothing is deployed. This tool cannot presently be run |

---

## Integration

### With the example workflows

The end-to-end flows in [`../examples/README.md`](../examples/README.md) already
choose a `START_LEDGER` before the run. With this tool, the operator starts the
tailer first, with `--since now`, and the hand-tracked start ledger stops being
a step that can be got wrong:

```bash
bridgelet-events tail "$NEW_ACCOUNT_ID" \
  --event created --event payment --event multi_pay --event swept_mul &

bridgelet-events tail "$SWEEP_CONTROLLER_ID" --event sweep &
```

The walkthrough's own verification steps — final transaction status,
`get_status() == 2`, `get_info().swept_to`, `can_sweep()` now false, the nonce
advanced by one, and destination balances changed — remain **the** check. A
tailer shows what the contracts said; it does not confirm that a balance
moved. [`../examples/README.md`](../examples/README.md) is explicit that a
successful submission, a contract event, and a balance change are three
separate facts.

### In CI

Bounded, read-only, with a distinct exit code for "no events" versus "RPC
down":

```bash
bridgelet-events tail "$SWEEP_CONTROLLER_ID" --event sweep \
  --since "$START_LEDGER" --json --duration 50 --no-follow --rpc-url "$RPC_URL"
```

Exit `0` with zero records is a legitimate result and must be reported as one,
not as a failure and not as a pass on the sweep. Exit `3` is the retry case.

### How this differs from a monitoring watchlist

[`../monitoring/event-watchlist.md`](../monitoring/event-watchlist.md) is not a
tool and does not consume events; it is a **policy reference** — which symbols
exist, when their absence is meaningful, and what evidence an alert would need.
[`../monitoring/README.md`](../monitoring/README.md) records that no collector,
cursor, or alert path exists in this repository.

| | This tool | [`../monitoring/`](../monitoring/README.md) |
|---|---|---|
| Purpose | Watch one contract during a session | Reconcile the deployment and detect conditions |
| Lifetime | One demo or test run | Continuous |
| Storage | Bounded in-memory scrollback, optional caller-supplied cursor | Durable collector, retention policy |
| Absence | Never interpreted | Meaningful only under declared preconditions |
| Operators | A developer or a demo audience | An on-call engineer |
| State today | Not implemented | Not implemented |

If a monitoring collector is ever built, it should consume the same
primitives — ledger-sequence cursor, the same dedup tuple, the same symbol
list — rather than growing a second, subtly different decoder.

### With name resolution

Accepting a contract name and resolving it is
[`contract-id-lookup-spec.md`](contract-id-lookup-spec.md)'s job, including
reporting that the resolved ID is unverified. Do not reimplement resolution
here, and do not silently accept an unverified ID.

---

## Placement

**This tool must not be placed in `scripts/` by this batch.** The batch that
produced these specs was explicitly scoped to avoid `scripts/`, and
`testnet/tooling/` is where the specification lives. Where an implementation
would eventually be built is undecided and recorded in
[`decision-log.md`](decision-log.md).

---

## Open Questions

- **Would a local sandbox remove the need for this tool?** Largely, yes. A
  sandbox is the natural home for demo and tailing workflows: instant ledgers,
  no shared-deployment noise, no retention window, and a contract you can
  deploy yourself. Its absence is a substantial part of why this tool is being
  proposed at all. `decision-log.md` argues this at length, and the same
  argument applies here with more force than to the other specs.
- **How much scrollback, and should it persist across runs?** A cursor file
  would make a long demo resumable, but it also means the tool writes files,
  which brushes against the read-only posture. A caller-supplied path only is
  the current proposal; whether an explicit opt-in to a default cache location
  is acceptable is not decided.
- **Should the tool decode payloads, or pass them through?** Decoding is much
  more useful in a demo and much more likely to be wrong across an interface
  change. The `full` / `partial` / `raw` marker in the JSON output is a
  proposed compromise; whether the human view should ever fall back to `raw` is
  open.
- **Poll interval.** One ledger is the natural default, but testnet ledger
  timing is not contractual and the RPC is rate-limited. A fixed interval, a
  server-suggested one, or an adaptive one is undecided.
- **Does it need a bounded-mode event count for demos specifically?** A
  `--duration` that stops on a quiet window would be useful for a scripted
  demo, but it risks being read as an absence check. Left open on purpose.

---

## Related Documentation

- [`decision-log.md`](decision-log.md) — why this is a spec, and the sandbox dependency
- [`../registry/event-topics.md`](../registry/event-topics.md) — the topic symbols and payload structs; the source of truth here
- [`../monitoring/event-watchlist.md`](../monitoring/event-watchlist.md) — event/absence policy; not a tool
- [`../monitoring/README.md`](../monitoring/README.md) — what monitoring does and does not do here
- [`../monitoring/uptime-check-spec.md`](../monitoring/uptime-check-spec.md) — read-only probe design and reset handling
- [`../examples/README.md`](../examples/README.md) — the workflows this sits beside, and the start-ledger pattern it replaces
- [`contract-id-lookup-spec.md`](contract-id-lookup-spec.md) — name → ID, with provenance
- [`../registry/contract-status.md`](../registry/contract-status.md) — the reason nothing can be tailed today
- [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md) — health checks, of which this is one
- [`../security/README.md`](../security/README.md) — the shared-deployment framing
- [`../security/known-testnet-abuse-patterns.md`](../security/known-testnet-abuse-patterns.md) — what interference looks like
- [`../config/nonce-tracking.md`](../config/nonce-tracking.md) — why a `sweep` you did not submit may explain a nonce jump
- [`../runbooks/nonce-desync.md`](../runbooks/nonce-desync.md) — recovery when a nonce moved under you

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-26 | Initial event tail spec |
