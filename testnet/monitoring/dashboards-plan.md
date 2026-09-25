# Bridgelet Testnet Dashboard Plan

## Status

> **Future plan only.** The repository has no deployed Grafana instance,
> Prometheus datasource, event indexer, canary runner, or alert route. No panel
> in this document should be presented as an active service today.

The future dashboard will show testnet contract lifecycle health and the
end-to-end synthetic canary described in
[`synthetic-transaction-plan.md`](synthetic-transaction-plan.md). Grafana is the
visualization layer; Soroban RPC, Horizon, and canary result records are the
proposed data sources.

Do not scrape Stellar Expert or Lumenscan for metrics. They are drill-down and
corroboration surfaces, not authoritative telemetry backends.

## Objectives

The first dashboard version must make these signals visible:

1. finalized sweep success and failure rates;
2. ephemeral-account creation rate;
3. average time from recorded payment to successful sweep;
4. the expiry-to-sweep terminal-outcome ratio; and
5. data freshness, testnet-reset state, and synthetic-canary integrity.

Every displayed value must be traceable to a finalized ledger range, a
normalized event, or an immutable canary result.

## Proposed data flow

```text
Soroban RPC                         Horizon
- getEvents                         - payments/history
- getTransaction                    - balance corroboration
- getLatestLedger                         |
      |                                  |
      +-----------------+----------------+
                        v
                Normalized ingestion
      - durable ledger cursor and reset epoch
      - event/transaction deduplication
      - account lifecycle projection
      - finalized canary result records
                        |
              metrics / structured logs
                        |
                      Grafana
                        |
                reviewed alert rules
```

## Source-of-truth rules

1. Contract state and contract events come from Soroban RPC.
2. Transaction inclusion and failure come from finalized transaction results.
3. Classic payment and balance observations come from Horizon.
4. The canary runner records attempted operations and destination-balance
   evidence that successful events alone cannot reconstruct.
5. Explorers provide operator links only.
6. A missing event is not classified as a failed operation without transaction
   or state evidence.
7. RPC/indexer errors are data-quality failures, not zero-valued metrics.

## Event and state model

Use the current source event names and scope every event by contract ID:

| Contract | Topic | Meaning |
|---|---|---|
| `EphemeralAccount` | `created` | `initialize` succeeded |
| `EphemeralAccount` | `payment` / `multi_pay` | first / subsequent payment was recorded |
| `EphemeralAccount` | `swept_mul` | account-side sweep state transition |
| `EphemeralAccount` | `expired` | `expire` or `recover` state transition |
| `EphemeralAccount` | `reserve` | internal reserve bookkeeping changed |
| `SweepController` | `sweep` | controller completion event |
| `SweepController` | `dest_auth` / `dest_upd` | destination configuration changed |
| `ReserveContract` | `init` / `reserve` | initialization / base-reserve update |
| `AccountFactory` | none | child accounts emit their own `created` events |

Important interpretation rules:

- `record_payment` proves metadata was recorded, not that an asset transfer
  occurred.
- `SweepCompleted.amount` sums raw amounts from different asset base units and
  must not be treated as one monetary value.
- A `sweep` event is not a substitute for final transaction status and balance
  reconciliation.
- The deployed WASM may differ from current source, so record the interface and
  code identity used by the collector.

## Bounded labels

Keep Prometheus/Grafana labels low-cardinality. Good labels include bounded
contract profiles, event topic, outcome, failure class, and source
(`synthetic`, `application`, or `operator`).

Do not use these as metric labels:

- ephemeral-account IDs;
- transaction hashes;
- destinations;
- nonces; or
- arbitrary RPC error messages.

Store those values in detailed run/event records and link from dashboard tables.

## Core metrics

### Finalized sweep success and failure

Use finalized attempts as the denominator, not submissions or successful
events.

```text
sweep_success_rate =
  strictly_verified_successful_attempts / finalized_attempts

sweep_failure_rate =
  finalized_failed_attempts / finalized_attempts
```

A strict success requires:

- final transaction status is successful;
- the matching controller `sweep` event exists;
- account status is `Swept`;
- `swept_to` matches the requested destination; and
- destination balance changed by the expected amount for each asset.

Show counts beside percentages. Suppress rate alerts when the sample is too
small to interpret.

### Account creation rate

```text
account_creation_rate =
  unique AccountCreated events / elapsed time
```

Count unique `created` events, not factory transactions. One factory batch can
create multiple child accounts.

### Average time to sweep

```text
time_to_sweep =
  successful_sweep_event_time
  - first_recorded_payment_event_time
```

Use ledger close timestamps from normalized events, not an assumed five-second
ledger duration. Display the sample count, median, p90, and p95 alongside the
average. Exclude expired accounts, pending attempts, and accounts without a
recorded payment.

### Expiry-to-sweep ratio

Use matured account cohorts and terminal outcomes:

```text
expiry_to_sweep_ratio =
  accounts_with_expired_terminal_outcome
  /
  (expired_terminal_accounts + verified_swept_terminal_accounts)
```

Assign each account to one creation cohort and count it only once. Display
terminal counts and open-account counts beside the ratio. This is a lifecycle
ratio, not proof of fund recovery.

### Data freshness

| Metric | Purpose |
|---|---|
| `rpc_latest_ledger` | Latest ledger observed from RPC |
| `indexer_last_finalized_ledger` | Last durably ingested ledger |
| `indexer_ledger_lag` | Difference between the two |
| `last_event_age_seconds` | Time since the last accepted event |
| `canary_last_success_timestamp` | Last strict end-to-end success |
| `canary_last_result_timestamp` | Last terminal canary result |
| `configured_code_hash_match` | Expected versus observed code identity |
| `network_reset_epoch` | Current testnet-reset generation |

## Grafana layout

### Row 1: Service and deployment status

- network passphrase fingerprint and testnet reset epoch;
- configured contract IDs and verification timestamps;
- current code/WASM identity;
- Soroban and Horizon reachability;
- last finalized ledger and collector lag;
- last canary result and age.

Use `unknown` or `stale`, never green, when required data is missing.

### Row 2: Synthetic canary

- outcome by controlled failure class;
- time for each workflow phase;
- nonce before/after for the latest run;
- expected versus observed destination balance;
- transaction and event correlation status;
- link to the transaction on Stellar Expert.

### Row 3: Sweep outcomes

- finalized attempts over time;
- success and failure rates;
- stacked failures by class;
- synthetic versus application traffic;
- pending-attempt age;
- recent failed transactions with drill-down links.

### Row 4: Account lifecycle

- account creation rate and cumulative count;
- open, swept, and expired cohorts;
- expiry-to-sweep ratio;
- terminal counts beside the ratio;
- number of accounts whose `expiry_ledger` has passed but which remain open.

### Row 5: Time to sweep

- average payment-to-sweep duration;
- median, p90, and p95;
- create-to-sweep duration;
- comparison with a trailing same-window baseline.

### Row 6: Data integrity

- indexer lag;
- duplicate event count;
- event/transaction reconciliation failures;
- state projection conflicts;
- code-hash mismatches;
- testnet-reset count.

## Alert inputs

Thresholds require a measured baseline. The future alert review should start
with:

| Condition | Initial proposal |
|---|---|
| Strict canary balance/state mismatch | Immediate investigation |
| Two consecutive finalized canary failures | Page after owner approval |
| No canary success for two intervals plus timeout | Page after owner approval |
| Sweep success below 99% over 30 minutes with at least 10 attempts | Warning |
| Indexer more than three closed ledgers behind RPC | Warning |
| Matured cohort expiry-to-sweep ratio above 25% with at least 20 accounts | Warning |
| Configured and observed code identity differ | Immediate investigation |
| New testnet reset epoch | Informational or warning |

These are proposals, not active SLOs. An empty window must not be reported as a
100% failure rate.

## Explorer drill-down

Use the links in [`explorer-links.md`](explorer-links.md) for bounded contract
IDs. For individual transactions, use the exact hash returned by the submitter:

```text
https://stellar.expert/explorer/testnet/tx/<TRANSACTION_HASH>
```

If an explorer page is missing, link to the retained RPC/Horizon evidence as
the fallback.

## Security and reliability requirements

- Never ingest or display secret keys, signing seeds, or auth payloads.
- Deduplicate before incrementing metrics.
- Preserve ledger, transaction, contract ID, and event index for every event.
- Treat pending transactions as unknown, not successful or failed.
- Do not sum heterogeneous asset amounts into a monetary total.
- Keep metric labels bounded.
- Segment all comparisons by testnet reset epoch.
- Display sample counts and data freshness with every rate.
- Make no alert claim until a real collector, notifier, and acknowledged test
  alert are running.

## Rollout

1. Implement and backfill a durable event/transaction cursor.
2. Reconcile normalized state with direct contract reads.
3. Activate immutable synthetic-canary result records.
4. Build rows 1-2 and validate empty, stale, and reset behavior.
5. Add lifecycle and latency cohorts.
6. Add reviewed alert rules and test every notification path.

Until those steps are complete, the correct dashboard status is:

```text
testnet_monitoring: not_configured
```

## Related documents

- [`README.md`](README.md)
- [`synthetic-transaction-plan.md`](synthetic-transaction-plan.md)
- [`explorer-links.md`](explorer-links.md)
- [`../registry/event-topics.md`](../registry/event-topics.md)
- [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md)
