# Runbook: Stellar Testnet RPC or Horizon Outage

## Scope

Use this runbook when either documented endpoint is unavailable, returns
errors, stops advancing, or behaves inconsistently:

- Soroban RPC: `https://soroban-testnet.stellar.org`
- Horizon: `https://horizon-testnet.stellar.org`

Bridgelet testnet support remains best effort. This runbook does not promise
active on-call coverage, restoration time, or uninterrupted service.

## Endpoint impact

| Failure | Affected operations | Safe continuation |
|---|---|---|
| Soroban RPC unavailable | contract simulation, submission, polling, state reads, and RPC event queries | Horizon may collect payment evidence into a durable queue, but do not trigger contract writes from stale observations |
| Horizon unavailable | payment discovery, classic history/balance reads, and workflows depending on them | Soroban read diagnostics may continue; do not infer a new payment from stored metadata |
| both unavailable | all reads/writes through the documented endpoints | pause dependent testnet mutation |
| local DNS/TLS/client failure | unknown until independently checked | preserve local evidence before declaring a public outage |

Soroban RPC and Horizon are not interchangeable. Contract state/events require
Soroban RPC; classic payment and account observation uses Horizon.

## 1. Record before acting

Capture:

- UTC start time and observer;
- endpoint and exact error;
- DNS, TLS, timeout, HTTP, JSON-RPC, or ledger-progress symptom;
- affected workflows;
- last successful Soroban ledger;
- last durable Horizon/event cursor;
- in-flight transaction hashes and pending record-payment intents;
- current `SweepController::get_nonce`; and
- whether any signed authorization was submitted elsewhere.

Do not clear queues or delete evidence while determining scope.

## 2. Run read-only checks

### Soroban RPC

```bash
curl --fail-with-body --silent --show-error \
  -H 'Content-Type: application/json' \
  --data '{"jsonrpc":"2.0","id":1,"method":"getLatestLedger"}' \
  "$STELLAR_SOROBAN_RPC_URL"
```

Compare the returned sequence with an earlier successful check. A successful
response does not prove transaction submission or event history is healthy.

### Horizon

```bash
curl --fail-with-body --silent --show-error \
  "$STELLAR_HORIZON_URL/payments?order=desc&limit=1"
```

Record the HTTP result and paging/ledger information. An empty result set is not
an outage.

### Stellar status

Check the official [Stellar status page](https://status.stellar.org/). Record
both its advisory state and direct endpoint evidence. A failure from one network
location is a suspected incident until broader reachability is established.

## 3. Pause dependent automation

Use a durable pause when available. Preserve read-only logs and cursors.

### Soroban RPC unavailable

Pause:

- `record_payment` submission;
- `execute_sweep` and `claim` submission;
- expiry, recovery, and reserve-reclaim submission;
- relayers/workers that retry or submit stale work; and
- testnet deployment jobs.

A Horizon watcher may continue collecting inbound-payment evidence only if its
queue is durable and will not release contract writes during the outage.

### Horizon unavailable

Pause every workflow that requires current classic payment or balance data,
including automatic payment recording and balance-dependent sweep/claim work.
Soroban-only read diagnostics may continue.

Do not infer a real inbound transfer from `get_info.payments`; that list is
recorded metadata.

### Both unavailable

Pause all dependent testnet contract mutation and deployment activity. Preserve
the last known state and pending work for reconciliation.

Pause creation of new sweep signatures unless signing is formally separated
from submission and stale-nonce risk is controlled.

### CI and integration jobs

Pause only jobs that access the unavailable testnet endpoint or depend on fresh
testnet state. Do not disable source-only build, formatting, lint, or local
contract tests merely because a public endpoint is down. Record which jobs were
skipped and why.

## 4. Handle in-flight transactions

A timeout is not proof of failure.

For every in-flight transaction:

1. preserve its hash and intended operation;
2. do not resubmit until the final on-chain outcome is known;
3. after recovery, query the original hash first;
4. reconcile state, nonce, events, and relevant balances; and
5. decide explicitly whether a new operation is required.

Do not log secret keys or reusable pre-submission authorization material.

## 5. Fallback policy

The current main branch does not contain a reviewed fallback-provider policy.
The separately tracked target `testnet/config/rpc-fallback.md` must not be
treated as approved merely because its path is referenced.

A Soroban RPC fallback may be used only after a maintainer verifies:

- exact Stellar Testnet passphrase and network identity;
- required methods for ledger reads, simulation, submission, transaction lookup,
  and event retrieval;
- ledger lag, history retention, pagination, and rate limits;
- stable transaction hashes and contract IDs on the same network; and
- failover tested before an incident, without automatic transaction replay.

A Horizon fallback must independently verify:

- the same testnet ledger history;
- account, payment, balance, and pagination operations;
- cursor/stream ordering and retention; and
- compatibility with durable watcher state.

A fallback is not a recovery guarantee. If none is approved, pause, preserve
evidence, wait, and reconcile. Never invent a provider during the incident.

## 6. Detect a testnet reset during recovery

Treat the network as reset if the latest ledger unexpectedly decreases, old
account/contract IDs disappear, or deployment records no longer resolve.

After a reset:

- do not replay transactions from pre-reset history;
- do not assume prior contract storage or nonce values exist;
- reconcile current deployment records and the testnet changelog;
- treat redeployment as a separate manual operation, not an outage workaround.

## 7. Recovery sequence

1. Confirm the intended endpoint is reachable and the ledger is advancing.
2. Confirm the exact testnet passphrase.
3. Verify contract IDs with
   [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md).
4. Reconcile every in-flight transaction by hash.
5. Resume Horizon from the last durable cursor, if Horizon recovered.
6. Release queued `record_payment` work one account/payment at a time only
   after confirming the real payment, asset, amount, and current state.
7. Re-read the controller nonce and create fresh sweep signatures.
8. Resume expiry/recovery and reserve-reclaim work separately.
9. Resume testnet-dependent CI/integration jobs one at a time.
10. Watch for duplicate events, stale IDs, nonce changes, and unexpected state.
11. Update verified registry timestamps and complete
    [`incident-postmortem-template.md`](incident-postmortem-template.md).

If recovery cannot be verified, keep dependent automation paused and record
the evidence gaps.

## Related documents

- [`../docs/network-config.md`](../docs/network-config.md)
- [`../docs/support.md`](../docs/support.md)
- [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md)
- [`on-call-testnet.md`](on-call-testnet.md)
- [`incident-postmortem-template.md`](incident-postmortem-template.md)
