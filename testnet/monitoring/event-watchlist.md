# Testnet Event Watchlist

## Status

> **Proposed event reference and absence policy.** No event collector or
> absence alert is active in this repository, and no honest positive event-rate
> baseline has been measured.

This document identifies the current source event symbols, the conditions under
which their absence is meaningful, and the evidence required before an alert.

## Source-of-truth rules

The current source files are:

- `contracts/ephemeral_account/src/events.rs`
- `contracts/sweep_controller/src/lib.rs`
- `contracts/reserve_contract/src/events.rs`

The deployed testnet WASM may differ. Record the contract ID, code/WASM hash,
and deployed interface before enabling a baseline. A method or payload mismatch
is a version signal, not automatically a network outage.

A testnet reset can remove contracts and historical events. Segment every
baseline by reset epoch.

## Event encoding

Soroban contract events contain an event-name symbol in `topics[0]`, optional
indexed values, and a contract-type payload in data. Indexers must filter by
both contract ID and `topics[0]`.

The Rust struct name is not always the on-chain symbol. For example,
`PaymentReceived` is published as `payment`, and `SweepExecutedMulti` as
`swept_mul`.

## EphemeralAccount events

| Topic `0` | Payload | Emitted by | Absence meaning |
|---|---|---|---|
| `created` | `AccountCreated { creator, expiry_ledger }` | successful `initialize` | Meaningful after a known initialization should have committed |
| `payment` | `PaymentReceived { amount, asset }` | first successful `record_payment` | Meaningful after a known first payment record |
| `multi_pay` | `MultiPaymentReceived { asset, amount }` | later distinct-asset `record_payment` | Meaningful after a known later record; not every account emits it |
| `swept_mul` | `SweepExecutedMulti { destination, payments }` | account-side `sweep` or `sweep_claim` | Meaningful after a known sweep/claim operation |
| `expired` | `AccountExpired` | successful `expire` or `recover` | Meaningful after a known terminal recovery action; not automatic at expiry ledger |
| `reserve` | `ReserveReclaimed` | reserve reclaim bookkeeping | Meaningful after a known sweep/expiry/reclaim action |

Important payload rules:

- Current source names the `AccountExpired` aggregate field
  `amount_returned`; older documents may call it `total_amount`.
- `Payment.amount` and `PaymentReceived.amount` use the asset's base units.
- `Payment.timestamp` is the ledger timestamp captured by `record_payment`.
- `expiry_ledger` and `ReserveReclaimed.sweep_id` are ledger values, not wall
  clock timestamps.
- A `payment`/`multi_pay` event proves metadata was recorded, not that a real
  asset transfer occurred.
- A `swept_mul` event proves the account-side state path ran, not by itself that
  the destination balance changed.

## SweepController events

| Topic `0` | Payload | Emitted by | Absence meaning |
|---|---|---|---|
| `sweep` | `SweepCompleted { ephemeral_account, destination, amount }` | current `execute_sweep` or `claim` | Meaningful after a known controller operation |
| `dest_auth` | `DestinationAuthorized { destination }` | initialization with a locked destination | Expected once for a known locked-controller initialization |
| `dest_upd` | `DestinationUpdated { old_destination, new_destination }` | successful authorized destination update | Meaningful after a known destination update |

`SweepCompleted.amount` is a raw sum of recorded amounts. It can combine
different asset base units and must not be presented as one monetary total.

A `sweep` event is not a balance receipt. Verify final transaction status and
per-asset destination balance when the metric means funds received.

## ReserveContract events

| Topic `0` | Payload | Emitted by | Absence meaning |
|---|---|---|---|
| `init` | `ContractInitialized { admin }` | successful initialization | Expected once for a known initialized contract |
| `reserve` | `BaseReserveUpdated { old_value, new_value, admin }` | successful base-reserve change | Meaningful after a known configuration update |

The current source publishes `init`, not `initialized`, and `reserve`, not
`base_reserve_updated`. Some existing registry prose predates those symbols.
Use the deployed interface as the final authority.

`ReserveContract` is not currently wired into `EphemeralAccount` by the
checked-in deployment tooling. Its events are optional for the core profile.

## AccountFactory events

The current `AccountFactory` source emits no direct events. A successfully
initialized child account emits its own `created` event. Do not alert on the
absence of a factory-specific event unless a future deployed WASM explicitly
adds one.

## Current minimum baseline

There is no defensible positive frequency baseline today because:

- the known-account registry contains placeholders;
- no synthetic workload is active;
- no historical event cursor is committed;
- deployment records disagree; and
- testnet resets can erase history.

The only honest standing minimum for new events in an arbitrary idle window
is:

```text
0 new events per event type
```

This does not mean an idle deployment is unhealthy. Event absence is meaningful
only when a specific committed or expected operation should have produced it.

## Conditional expectations

| Event | Expected baseline | Alert precondition |
|---|---|---|
| `created` | one per registered initialized child account | known successful initialization and cursor covers its ledger |
| `payment` | one for a fixture recorded for its first asset | known first `record_payment` succeeded |
| `multi_pay` | one per known later distinct asset | later record was expected |
| `swept_mul` | one for each successful account-side sweep/claim | operation and account are known |
| `expired` | one for each explicit expiry/recovery action | terminal action was expected; time eligibility alone is insufficient |
| ephemeral `reserve` | one per known reclaim attempt | reclaim was expected |
| `sweep` | one per known controller completion | controller operation and account are known |
| `dest_auth` | one historical event for a locked controller | initialization with a destination is known |
| `dest_upd` | one per authorized destination update | update is known |
| Reserve `init` | one historical event when initialized | initialization is known |
| Reserve `reserve` | one per base-reserve change | configuration change is known |
| AccountFactory event | not applicable | never alert with the current source |

A fixture must not be required to emit both `payment` and `multi_pay`. Declare
which operation is expected from its purpose and state.

## Absence-alert policy

Do not alert merely because:

- no event appeared in the last five minutes;
- no sweep was attempted;
- no payment was recorded;
- no account reached an explicit `expire`/`recover` action;
- no destination update was requested;
- no reclaim was attempted; or
- an optional contract is not configured.

An event-overdue alert is valid only when:

1. the triggering operation was expected to succeed;
2. the transaction/ledger range is known;
3. the contract ID and deployed interface are current;
4. the collector cursor covers the range and is not lagging; and
5. the event symbol is appropriate for that operation.

A practical initial indexing deadline is two collector poll cycles, with a
five-minute maximum. It must be tuned to the actual collector and is not a
contract SLA.

## State versus event failures

- RPC timeout, contract-not-found, or a non-decoding read: investigate RPC,
  deployment, and interface state first.
- State changed successfully but the event is absent: investigate the event
  cursor, decoder, retention, and deployed ABI.
- Event exists but state is inconsistent: investigate the deployed version,
  transaction correlation, or state projection.
- All old IDs disappear after a ledger decrease: declare the baseline stale and
  require a new reset epoch and deployment verification.

Do not infer failure solely from a missing event.

## Collector record

A normalized event should retain at least:

```json
{
  "network": "testnet",
  "reset_epoch": "<epoch-id>",
  "contract_id": "<CONTRACT_ID>",
  "topic_0": "payment",
  "ledger": 0,
  "transaction_hash": "<TX_HASH>",
  "event_index": 0,
  "data": {}
}
```

Deduplicate by reset epoch, ledger, transaction hash, contract ID, and event
index. Preserve raw XDR for incident analysis according to a documented
retention policy, but never store credentials or unrelated auth payloads.

## Units

| Field | Unit |
|---|---|
| `expiry_ledger`, `sweep_id` | ledger sequence |
| `Payment.timestamp` | ledger timestamp seconds |
| payment/sweep amounts | asset base units (`i128`) |
| `AccountExpired.amount_returned` | raw aggregate of recorded amounts; not cross-asset money |
| reserve amounts | stroops |

Native XLM uses 10,000,000 stroops per XLM. Other SEP-41 assets can use
different base units. Never add heterogeneous amounts and label the result a
balance.

## Activation checklist

- [ ] Reconcile and verify contract IDs after the latest reset.
- [ ] Record deployed interfaces and current code identity.
- [ ] Configure an explicit contract/topic allowlist.
- [ ] Persist and test the ledger cursor and deduplication.
- [ ] Populate known account/operation expectations.
- [ ] Define pending-transaction and pending-expiry handling.
- [ ] Test missing-event, delayed-indexer, and RPC-outage cases.
- [ ] Calibrate positive frequency only after a declared workload.
- [ ] Approve owners and non-production notification routes.

Until then, the status is:

```text
testnet_event_watchlist: not_configured
```

## Related documents

- [`../registry/event-topics.md`](../registry/event-topics.md)
- [`../registry/known-test-accounts.md`](../registry/known-test-accounts.md)
- [`../registry/contract-status.md`](../registry/contract-status.md)
- [`../config/nonce-tracking.md`](../config/nonce-tracking.md)
- [`../runbooks/failed-sweep-signature.md`](../runbooks/failed-sweep-signature.md)
- [`uptime-check-spec.md`](uptime-check-spec.md)
