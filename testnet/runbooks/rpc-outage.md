# Runbook: Stellar Testnet RPC or Horizon Outage

## Scope

Use this runbook when public testnet infrastructure is degraded or down:

- Soroban RPC: `https://soroban-testnet.stellar.org`
- Horizon: `https://horizon-testnet.stellar.org`

These services are outside Bridgelet’s control. Support for testnet automation
is best-effort; this runbook is about **containing impact**, not restoring
Stellar’s endpoints.

## Immediate actions (checklist)

1. **Check Stellar’s status page** — [https://status.stellar.org/](https://status.stellar.org/)  
   Note any active incidents for Testnet / Soroban / Horizon.
2. **Confirm locally** with the read-only probes below (rule out laptop DNS/TLS).
3. **Switch clients to the fallback RPC** documented in
   [`testnet/config/rpc-fallback.md`](../config/rpc-fallback.md) when the primary
   Soroban RPC is the failing dependency.
4. **Pause automated testnet-dependent CI and integration jobs** that need live
   RPC/Horizon (keep pure unit/lint/build jobs running).
5. **Preserve evidence** — do not clear queues, nonces, or pending
   `record_payment` / sweep intents until outcomes are known.

## Endpoint impact

| Failure | Affected operations | Safe continuation |
|---|---|---|
| Soroban RPC down | simulate, submit, poll, contract state, Soroban events | Horizon may still observe classic payments into a **durable** queue; do **not** submit contract writes from stale state |
| Horizon down | payment discovery, classic balances/history | Soroban read diagnostics only; do not invent payments from cached metadata |
| Both down | all documented public endpoints | pause testnet mutation |
| Local DNS/TLS only | appears down for one operator | fix local network; do not pause org-wide CI |

Soroban RPC and Horizon are **not** interchangeable.

## 1. Record before acting

Capture UTC time, observer, endpoint, exact error, last good ledger/cursor,
in-flight tx hashes, pending payment intents, and current sweep nonce if
relevant. Do not delete evidence while scoping.

## 2. Read-only health probes

### Soroban RPC

```bash
curl --fail-with-body --silent --show-error \
  -H 'Content-Type: application/json' \
  --data '{"jsonrpc":"2.0","id":1,"method":"getLatestLedger"}' \
  "${STELLAR_SOROBAN_RPC_URL:-https://soroban-testnet.stellar.org}"
```

### Horizon

```bash
curl --fail-with-body --silent --show-error \
  "${STELLAR_HORIZON_URL:-https://horizon-testnet.stellar.org}/payments?order=desc&limit=1"
```

Empty payment lists are not an outage. Compare ledger progress to a previous
successful observation.

### Stellar status

Cross-check [status.stellar.org](https://status.stellar.org/). A single-region
failure is a **suspected** incident until confirmed more broadly.

## 3. Fallback RPC

Follow [`testnet/config/rpc-fallback.md`](../config/rpc-fallback.md):

- Point `STELLAR_SOROBAN_RPC_URL` (and CLI network config) at the documented
  fallback.
- Re-run the `getLatestLedger` probe against the fallback before resuming
  submits.
- Record which endpoint was used for any transactions submitted during the
  incident.

If the fallback is also unhealthy, treat as dual-path outage and stay paused.

## 4. Pause dependent automation

### CI / integration tests

Pause jobs that:

- deploy or invoke contracts on testnet;
- assert against live RPC/Horizon responses;
- consume testnet faucets or shared ephemeral accounts.

Do **not** pause source-only build, unit tests, fmt, or lint. Document skipped
jobs in the incident note.

### Runtime workers

| Dependency down | Pause |
|---|---|
| Soroban RPC | `record_payment`, sweep/claim submitters, expiry/recovery workers, testnet deploys |
| Horizon | auto payment discovery and balance-dependent sweeps |
| Both | all testnet contract mutation |

Horizon watchers may continue **enqueue-only** collection if the queue will not
auto-submit during the outage.

## 5. In-flight transactions

A timeout is not a confirmed failure.

1. Keep the hash and intended operation.  
2. Do not resubmit until final status is known.  
3. After recovery, query the original hash first.  
4. Reconcile state, nonces, events, and balances.  
5. Only then decide on a new transaction.

Never log secret keys or reusable pre-image auth material.

## 6. Resume criteria

Resume automated submits only when:

- primary or fallback RPC passes health probes; and
- status page does not contradict recovery; and
- one successful dry-run read (and a low-risk test invoke, if policy allows)
  has been recorded.

## Related documentation

- [`testnet/config/rpc-fallback.md`](../config/rpc-fallback.md)
- [`on-call-testnet.md`](on-call-testnet.md)
- [`incident-postmortem-template.md`](incident-postmortem-template.md)
- [`../docs/support.md`](../docs/support.md)
