# Testnet Alert Threshold: Expiry-to-Sweep Ratio

## Status

> **Proposed threshold only.** No collector, evaluator, dashboard, or alert
> destination is active in this repository. This document defines a concrete
> starting policy for a future implementation.

## Signal

Track two event types in finalized ledgers:

| Symbol | Contract | Meaning |
|---|---|---|
| `AccountExpired` / topic `expired` | `EphemeralAccount` | A successful `expire()` or `recover()` call |
| `SweepCompleted` / topic `sweep` | `SweepController` | A controller `execute_sweep()` or `claim()` completion event |

For a configured scope, let:

```text
E = unique AccountExpired events
S = unique SweepCompleted events
```

The two counts are intentionally not the same metric:

- `expired` is not emitted automatically when the ledger reaches
  `expiry_ledger`; an operator or relayer must successfully call `expire()` or
  `recover()`.
- `sweep` is a controller completion event, not by itself proof of a token
  balance change. The current source also emits it from `claim()`.
- Always filter by the expected account and controller contract IDs. Do not
  count the account-side `swept_mul` event as a second `SweepCompleted` event.

## Proposed starting threshold

Page for a sustained rise when:

```text
S > 0
E >= 2 * S
E + S >= 10
```

The condition must remain true for 15 consecutive minutes.

| Parameter | Starting value |
|---|---:|
| Trailing lookback | 30 minutes |
| Evaluation interval | Every 5 minutes |
| Ratio | `E / S >= 2.0` |
| Sustained duration | 15 minutes |
| Minimum sample | 10 total relevant events |
| Resolve after | Condition false for 15 consecutive minutes |

The 30-minute window filters short bursts. The 15-minute sustain requirement
prevents a single batch from paging. The minimum sample prevents one event in
an otherwise quiet testnet from creating a noisy ratio.

This is an initial operating threshold, not a statistically validated SLO.
Calibrate it after at least seven days of representative data.

## Zero-sweep case

The ratio is undefined when `S == 0`. Do not represent that case as an
infinite ratio or as a 100% failure rate.

Use a separate diagnostic condition:

```text
S == 0
E >= 10
condition persists for 15 minutes
```

Label it `sweep_completion_silence`, not `expiry_to_sweep_breach`. It can
indicate a stopped relayer, an event-indexing problem, a low-traffic period, or
a contract regression; it does not identify the cause by itself.

If both counts are zero, the standing-state health check must determine whether
the deployment is idle or unavailable. Event absence alone is not an outage.

## Collection requirements

A future collector must:

1. Read finalized Soroban ledger ranges and persist a durable cursor.
2. Resume after the last processed ledger without reusing a stale cursor.
3. Deduplicate by contract ID, ledger, transaction hash, and event index.
4. Decode the deployed interface; testnet WASM may differ from current source.
5. Keep the network/reset epoch and contract allowlist with the metrics.
6. Record RPC errors, skipped ledgers, and cursor lag as data-quality failures.
7. Never convert an RPC error or incomplete page into zero events.

Example bounded queries:

```bash
stellar contract events \
  --contract-id <EA_CONTRACT_ID> \
  --start-ledger <START_LEDGER> \
  --filter '{"topics":[["expired"]]}' \
  --network testnet

stellar contract events \
  --contract-id <SWEEP_CONTROLLER_ID> \
  --start-ledger <START_LEDGER> \
  --filter '{"topics":[["sweep"]]}' \
  --network testnet
```

Replace every placeholder and retain the raw results and finalized ledger range.

## Alert payload

A future alert should include:

- environment and testnet reset epoch;
- `window_start` and `window_end` ledger/time;
- collector cursor and lag;
- `account_expired_count` (`E`);
- `sweep_completed_count` (`S`);
- calculated ratio or the zero-sweep diagnostic reason;
- minimum-sample and sustained-duration state;
- contract IDs included in the calculation;
- RPC/indexer health; and
- links to this policy and the recovery investigation procedures.

The alert must not call `expire()`, `recover()`, `sweep()`, `claim()`, or any
other state-changing method.

## Initial investigation

When the threshold is breached:

1. Confirm the collector cursor covers the complete window and the contract
   allowlist is correct.
2. Confirm delayed event ingestion is not inflating `E` or suppressing `S`.
3. Compare `created`, `payment`, and `multi_pay` activity with expirations.
4. Find accounts whose `expiry_ledger` has passed but which have no `expired`
   event; these may indicate a stopped expiry worker.
5. Inspect finalized `SweepController::execute_sweep` transactions, relayer
   logs, and signature failures.
6. Re-read the current `get_nonce` and reconcile the last successful signed
   operation. Do not infer the nonce solely from event counts.
7. Check `upgrade-history.md` and the deployed WASM identity for a recent
   behavior or interface change.
8. For an expiry discrepancy, reconcile `recovery_address` and each asset's
   balance independently.

## Baseline and activation

Before enabling a page:

- [ ] Reconcile the current deployment records and verify live contract IDs.
- [ ] Record the deployed event ABI and code/WASM identity.
- [ ] Run the collector in report-only mode for at least seven days.
- [ ] Confirm how `claim()` should be counted for the intended metric.
- [ ] Label the initial signal as reported controller completions, not funds
      received.
- [ ] Test a ratio breach, zero-sweep case, RPC outage, skipped ledger, and
      recovery below threshold.
- [ ] Assign a primary and backup owner and a non-production alert route.
- [ ] Obtain maintainer approval for the threshold, window, and sample size.

Until all items are complete, the operational status remains:

```text
expiry_to_sweep_alert: not_configured
```

## Related documents

- [`README.md`](README.md)
- [`../registry/event-topics.md`](../registry/event-topics.md)
- [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md)
- [`../registry/upgrade-history.md`](../registry/upgrade-history.md)
- [`../runbooks/expired-account-recovery-check.md`](../runbooks/expired-account-recovery-check.md)
