# Testnet Monitoring

## Current status

> **No active testnet monitoring is implemented in this repository.**

There is no scheduled event collector, durable ledger cursor, metrics
exporter, dashboard, alert evaluator, or notification integration in the
checked-in project. The documents in this directory are procedures and plans;
they must not be interpreted as evidence that monitoring is running.

The root README contains stale claims about disabled CI. The current workflow
files do define contract build/test jobs and a manually dispatched deployment
job. That distinction does not change the operational status: none of those
jobs polls testnet contract events, evaluates health, or delivers an alert.

## Inventory

### Capabilities that exist

| Capability | Status | Boundary |
|---|---|---|
| Contract build/test workflows | Workflow configuration exists | Source validation, not testnet monitoring |
| Manual deployment workflow | `workflow_dispatch` configuration exists | A deployment job is not a health monitor |
| Manual contract checks | Procedure in [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md) | Operator-run and read-only unless a command is changed |
| Event topic reference | [`../registry/event-topics.md`](../registry/event-topics.md) | Documentation only; it does not consume events |
| Manual upgrade log | [`../registry/upgrade-history.md`](../registry/upgrade-history.md) | Operator-maintained; no watcher populates it |
| Manual runbooks | [`../runbooks/`](../runbooks/) | Procedures are not automated execution |

### Capabilities that do not exist

- Scheduled Soroban RPC polling
- Durable event/transaction cursors and deduplication
- An off-chain payment watcher or relayer monitor
- Account-expiry and overdue-work detection
- Prometheus-compatible metrics or a deployed dashboard
- Alert evaluation or delivery
- Automated code/WASM hash drift detection
- Automated `AccountExpired` and SEP-41 balance reconciliation
- A documented on-call rotation or monitoring SLO

Architecture documents may describe a payment watcher, signer, SDK, or relayer
as an integration responsibility. Those descriptions are not evidence that
such a service is deployed by this repository.

## Deployment-record warning

The testnet registry is internally inconsistent. Older deployment artifacts
and network configuration contain contract IDs, while
[`../registry/contract-status.md`](../registry/contract-status.md) reports that
the contracts have not been deployed or verified. The timestamps in
`last-verified.json` do not include status, ledger, transaction, or code-hash
evidence.

Treat every stored ID as unverified until an operator performs the read-only
procedure in `verify-contract-live.md` and records the evidence.

## Proposed target state

```text
Soroban RPC (read-only)
        |
        v
durable event/transaction collector
        |
        v
decoder + deduplication + contract allowlist
        |
        +--> account lifecycle projection
        +--> finalized sweep metrics
        +--> expiry/recovery reconciliation
        +--> code/WASM drift checks
        |
        v
reviewed alert evaluator and notifier
        |
        v
runbook-driven human investigation
```

This architecture is a proposal, not a description of current infrastructure.

## Collector requirements

A future implementation should:

- use Stellar Testnet and verify the exact network passphrase;
- read only an explicit allowlist of verified contract IDs;
- persist the last finalized ledger processed;
- handle duplicate pages and a detected testnet reset;
- decode the deployed ABI rather than assuming current source is live;
- correlate events by contract ID, ledger, transaction hash, and event index;
- distinguish transport failure, incomplete ingestion, and true zero events;
- keep transaction hashes, account IDs, and nonces in detailed records rather
  than metric labels; and
- exclude secret keys, signing seeds, and auth payloads.

## Initial signals

The first implementation can remain small:

| Signal | Definition |
|---|---|
| Sweep outcome rate | finalized successful and failed `execute_sweep` attempts |
| Account creation rate | unique `created` events over time |
| Time to sweep | first recorded payment to successful sweep |
| Expiry-to-sweep ratio | terminal `expired` versus terminal swept accounts |
| Event lag | latest RPC ledger minus last durably processed ledger |
| Recovery integrity | event/state/balance reconciliation for expired accounts |
| Code identity | configured versus observed contract code hash |

The proposed starting ratio policy is in
[`alert-thresholds.md`](alert-thresholds.md). It must remain labeled as a
proposal until a real collector, evaluator, and notifier are running.

## Manual baseline available now

Until the proposed collector exists, an operator can:

1. Verify contract IDs and read-only methods with
   [`verify-contract-live.md`](../registry/verify-contract-live.md).
2. Choose a bounded, finalized ledger range.
3. Query the expected event topics using
   [`event-topics.md`](../registry/event-topics.md).
4. Save the raw event output, contract IDs, ledger range, and conclusion.
5. Reconcile contract state and relevant balances.
6. Update the registry only after verification.

These are manual observations. They do not constitute continuous monitoring.

## Data-quality rules

A future alert must distinguish:

- `rpc_unavailable`
- `rpc_timeout`
- `cursor_lag`
- `incomplete_ledger_range`
- `decode_error`
- `contract_not_found`
- `code_identity_mismatch`
- `testnet_reset`
- an actual lifecycle or sweep condition

A failed data source is not a healthy zero and is not a contract outage until
contract reachability is tested independently.

## Activation checklist

Monitoring must not be marked active until all applicable items are complete:

- [ ] Deployment records are reconciled and live IDs are verified.
- [ ] The deployed event ABI and code identity are recorded.
- [ ] A primary and backup owner are assigned.
- [ ] Durable cursor, deduplication, and reset handling are exercised.
- [ ] RPC outage and skipped-ledger behavior produce data-quality alerts.
- [ ] Metric definitions and units are reviewed.
- [ ] Ratio thresholds are baselined and approved.
- [ ] Recovery balance reconciliation is exercised on a disposable account.
- [ ] Upgrade verification identifies changed behavior and state preservation.
- [ ] A test notification reaches a non-production route.
- [ ] Credentials are excluded from repository configuration.

Until then, the status is:

```text
testnet_monitoring: not_configured
```

## Related documents

- [`alert-thresholds.md`](alert-thresholds.md)
- [`../runbooks/upgrade-verification.md`](../runbooks/upgrade-verification.md)
- [`../runbooks/expired-account-recovery-check.md`](../runbooks/expired-account-recovery-check.md)
- [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md)
- [`../registry/upgrade-history.md`](../registry/upgrade-history.md)
