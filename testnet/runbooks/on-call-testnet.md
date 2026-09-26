# Testnet On-Call and Support Expectations

## Current support posture

Bridgelet testnet support is **best effort**. The current repository does not
define:

- an active on-call rotation;
- a paging destination;
- 24/7 coverage;
- an availability SLO;
- a guaranteed acknowledgement or response time; or
- active automated testnet monitoring.

A maintainer may investigate when available. This runbook is an operational
guide, not a declaration that someone is always on the hook.

The existing [`../docs/support.md`](../docs/support.md) likewise states that no
response time is guaranteed. Integrators must not use testnet as a
business-critical service with an assumed SLA.

## Repository evidence

| Evidence | What it establishes |
|---|---|
| [`../docs/support.md`](../docs/support.md) | maintainer support is best effort |
| [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md) | health checks are manual, not automated monitoring |
| [`.github/workflows/test.yml`](../../.github/workflows/test.yml) | source build/test validation; no testnet event monitor |
| [`.github/workflows/deploy-testnet.yml`](../../.github/workflows/deploy-testnet.yml) | manual deployment trigger; no scheduled health check |

Deployment documents also disagree about which contracts are live. Treat a
stored contract ID as unverified until a read-only call succeeds on the
intended testnet.

## Before an incident

Capture and make available to responders:

- network passphrase, Soroban RPC URL, and Horizon URL from
  [`../docs/network-config.md`](../docs/network-config.md);
- exact contract IDs and, when known, deployed code/WASM hashes;
- UTC time and ledger of the last successful endpoint/contract check;
- last durable Horizon cursor and Soroban event cursor, if one exists;
- in-flight transaction hashes and current `SweepController::get_nonce` value;
- pause controls for payment recorders, sweep/claim workers, expiry/recovery
  workers, and deployment processes; and
- a private location for logs that must not be posted publicly.

Never store secret keys, seed phrases, or unredacted `.env` files in this
runbook, a public issue, or a postmortem.

## Severity and ownership

No formal rotation is active. Use these planning labels during an incident:

| Severity | Example | Current expectation | Proposed owner after activation |
|---|---|---|---|
| `security` | unauthorized transfer, signature bypass, state corruption | stop automation and use private security reporting | `<SECURITY_OWNER>` and backup |
| `critical` | shared deployment unusable or integrity mismatch | preserve evidence and pause dependent writes when possible | `<PRIMARY_OWNER>` and backup |
| `degraded` | relayer, event cursor, or one integration path impaired | document workaround and affected scope | `<PRIMARY_OWNER>` |
| `single-user` | one integration/config/account issue | normal best-effort issue triage | maintainer |

The placeholders are planning fields, not staffed assignments. The project
must not advertise response targets until they are replaced with named,
acknowledged owners.

## During an incident

1. Record the first observation in UTC.
2. Open [`incident-postmortem-template.md`](incident-postmortem-template.md).
3. Classify the symptom as endpoint availability, deployment/configuration
   drift, contract behavior, automation, data integrity, security, or unknown.
4. Pause dependent **mutating** automation when continued writes could
   duplicate a payment, consume a stale signature, or hide evidence.
5. Preserve read-only logs, cursors, queue contents, and transaction hashes.
6. For an endpoint incident, follow [`rpc-outage.md`](rpc-outage.md).
7. Never blindly retry a transaction with an unknown submission result.
8. Do not deploy or upgrade merely to hide an endpoint or configuration error.
   Any emergency mutation requires separate authorization and a recorded
   rationale.
9. For ordinary testnet problems, use the `[testnet]` issue process in
   `support.md`. Priority does not imply a response-time guarantee.
10. For possible vulnerabilities, do not open a public issue. Use GitHub's
    private vulnerability reporting channel.

## Evidence to preserve

At minimum:

- endpoint errors and UTC timestamps;
- latest successful ledger and Horizon cursor;
- contract IDs, interfaces, and code/WASM identity;
- transaction hashes, final results, and diagnostic events;
- account/controller state and relevant balances;
- worker queue and pause status; and
- the exact operation that first exposed the issue.

Redact credentials, auth signatures, private authorization frames, and
unnecessary user data.

## Recovery checklist

Before resuming automation:

1. Confirm the intended endpoint is reachable and the ledger is advancing.
2. Confirm the network passphrase and contract IDs match configuration.
3. Reconcile every in-flight transaction by hash.
4. Reconcile real payment observations before replaying `record_payment`.
5. Re-read `SweepController::get_nonce` before creating a new signature.
6. Resume one workflow class at a time and stop on the first mismatch.
7. Verify expected events, state, and per-asset balances.
8. Update the registry with verified evidence.
9. Complete the incident postmortem and assign follow-up actions.

## Activation requirements for real on-call coverage

Before claiming an active on-call service, commit and test:

- a named primary and secondary maintainer/service owner;
- a monitored alert route with an acknowledged test page;
- endpoint and contract health checks with defined freshness;
- escalation and update expectations for each severity;
- a backup responder and handoff procedure;
- a tested credential-rotation and private-report process; and
- a reviewed fallback-provider policy.

Until these are in place, the operational status is:

```text
testnet_on_call: not_staffed
testnet_monitoring: not_configured
```

## Related documents

- [`incident-postmortem-template.md`](incident-postmortem-template.md)
- [`rpc-outage.md`](rpc-outage.md)
- [`reentrancy-suspicion.md`](reentrancy-suspicion.md)
- [`../docs/support.md`](../docs/support.md)
- [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md)
