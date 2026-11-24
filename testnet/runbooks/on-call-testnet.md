# Testnet On-Call and Support Expectations

## Current support posture

Bridgelet testnet support is **strictly best effort**. Even though real
integrators and partners rely on Stellar Testnet for testing and integration,
the Bridgelet project is an open-source MVP and does not provide
production-grade operational guarantees.

Specifically, the project does **not** maintain:

- **No dedicated on-call team:** No person or team is on the hook or paged for
  testnet outages.
- **No active automated monitoring:** No 24/7 synthetic canary, health monitor,
  or automated pager alerts maintainers to down contracts or RPC failures.
- **No availability SLA or SLO:** There is no commitment to uptime, error
  rates, or deployment continuity.
- **No guaranteed response time:** There is no binding timeframe for issue
  acknowledgment, triage, or remediation.

Maintainers investigate issues voluntarily when available during normal
working hours. Integrators must calibrate their architecture, test suites, and
project milestones to this reality.

```text
testnet_support_tier: best_effort
testnet_on_call: not_staffed
testnet_sla: none
testnet_active_monitoring: false
```

## Expectations for integrators and partners

Because testnet has external users depending on it staying reasonably usable,
integrators should observe the following guidance to calibrate their reliance:

### 1. Do not rely on testnet for mission-critical workflows
- Testnet is an ephemeral sandbox intended for integration experiments and
  verification, not for production traffic, critical customer demos without
  fallbacks, or SLA-backed partner showcases.
- Expect unannounced redeployments, contract upgrades, and periodic platform
  resets by the Stellar Development Foundation (SDF).

### 2. Outage awareness relies on community reports
- Because there is no active 24/7 monitoring daemon paging maintainers,
  maintainers are generally unaware of testnet incidents until an integrator
  or community member files a report.
- If you observe an endpoint failure, contract revert, or invalid state,
  report it promptly via the `[testnet]` GitHub issue workflow described in
  [`../docs/support.md`](../docs/support.md).

### 3. Expected response and triage times
While no binding SLA exists, maintainers observe the following informal
prioritization during triage:

| Issue scope | Example | Informal triage expectation |
|---|---|---|
| **Ecosystem-blocking outage** | Complete deployment failure, missing contracts following an SDF reset, or invalid contract IDs affecting all users | Best-effort maintainer triage as priority, typically within 1–2 business days |
| **Suspected security vulnerability** | Signature bypass, unauthorized transfer, state corruption | Acknowledged via private security reporting as maintainer availability permits |
| **Degraded service / single-user issue** | Nonce desync, transient RPC error, or local integration query | Handled as general issue backlog during regular maintenance |

### 4. Resiliency recommendations for client integrations
- **Handle periodic SDF resets:** The SDF periodically resets testnet, wiping
  all accounts, contract storage, and ledgers. Automate your deployment and
  initialization fixtures so you can rebuild state after a reset.
- **Implement graceful retry and timeout logic:** Public testnet RPC nodes can
  experience rate-limiting, lag, or downtime. Build client-side exponential
  backoff and fallback RPC configurations.
- **Pin contract IDs:** Verify contract IDs against
  [`../registry/contract-status.md`](../registry/contract-status.md) and
  [`../docs/changelog.md`](../docs/changelog.md) before testing runs.
- **Use local sandboxes for automated CI:** For deterministic CI/CD pipelines
  and regression tests, prefer a local standalone Soroban sandbox or mock
  environment over the shared public testnet.

## Repository evidence

| Evidence | What it establishes |
|---|---|
| [`../docs/support.md`](../docs/support.md) | Maintainer support is best effort with no response guarantee |
| [`../docs/contact-and-ownership.md`](../docs/contact-and-ownership.md) | No named owner, `CODEOWNERS`, or on-call contacts are staffed |
| [`../monitoring/README.md`](../monitoring/README.md) | No active monitoring is running (`testnet_monitoring: not_configured`) |
| [`../monitoring/uptime-check-spec.md`](../monitoring/uptime-check-spec.md) | Health checks and uptime monitoring are future specifications only |
| [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md) | Contract health checks are manual operator procedures, not automated monitors |
| [`.github/workflows/test.yml`](../../.github/workflows/test.yml) | Source build/test validation; no testnet event monitor |
| [`.github/workflows/deploy-testnet.yml`](../../.github/workflows/deploy-testnet.yml) | Manual deployment trigger; no scheduled health check |

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
   [`../docs/support.md`](../docs/support.md). Priority does not imply a
   response-time guarantee.
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
- [`../docs/contact-and-ownership.md`](../docs/contact-and-ownership.md)
- [`../monitoring/uptime-check-spec.md`](../monitoring/uptime-check-spec.md)
- [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md)
