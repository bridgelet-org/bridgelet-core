# Testnet Monitoring: Baseline & Architecture

## Current Status: Honest Baseline

> **Current Operational Status: No active testnet monitoring is implemented in this repository.**
>
> There is currently no scheduled event collector, durable ledger cursor, Prometheus
> metrics exporter, deployed Grafana dashboard, automated alert evaluator, or notification
> webhook running in this repository or associated infrastructure.
>
> All documents in this directory represent **specifications, operational runbooks, and
> architectural proposals**. They must not be presented to developers, auditors, or users as
> evidence of active, running background services.

### Avoiding "Phantom Pipeline" Documentation

In smart-contract and blockchain infrastructure, documentation frequently succumbs to
the anti-pattern where documents describe pipelines, watchers, and monitors as if they
are operational realities when no code actually executes. The root [`README.md`](../../README.md)
explicitly calls out this discrepancy for CI/CD:

> *"Despite earlier documentation claiming automated test and deploy pipelines, both GitHub
> Actions workflows are non-functional as committed..."*

This monitoring directory establishes its baseline from that same transparent principle:
**we document exactly what exists today, categorize what is aspirational, and define the
concrete engineering milestones required before any capability can be marked active.**

---

## Directory Inventory: Active vs. Aspirational Tracking

The table below tracks the status of every component and document within `testnet/monitoring/`:

| Document | Purpose | Operational status | Backing implementation | Immediate capability today | Transition to active requires |
|---|---|---|---|---|---|
| [`README.md`](README.md) | Directory index, honest baseline, and capability inventory | **Active Specification** | Markdown reference | Authoritative tracking of monitoring state | Continuous maintenance against actual code changes |
| [`event-watchlist.md`](event-watchlist.md) | On-chain signals of life, topic encodings, baseline frequencies, and dead-man alerting criteria | **Proposed Specification** | None (spec only) | Reference for manual event inspection via CLI | Deployment of durable event indexer & silence timer |
| [`alert-thresholds.md`](alert-thresholds.md) | Expiry-to-sweep ratio ($E \ge 2S$) and zero-sweep silence detection logic | **Proposed Policy** | None (policy only) | Manual post-hoc calculation over queried ledgers | Automated metrics evaluator reading finalized ledgers |
| [`synthetic-transaction-plan.md`](synthetic-transaction-plan.md) | Scheduled 30-minute create-pay-sweep canary workflow using disposable accounts | **Proposed Plan** | None (spec only) | Step-by-step procedure for manual canary execution | Dedicated signing service, cron runner, and funded fee payer |
| [`balance-thresholds.md`](balance-thresholds.md) | Minimum funding floors, re-fund triggers, and Friendbot runbooks for test accounts | **Active Operational Runbook** | Manual CLI / curl scripts | Operator runbook to inspect balances and fund accounts | Scheduled balance scraper and automated alert route |
| [`uptime-check-spec.md`](uptime-check-spec.md) | Soroban RPC reachability, ledger progression, and read-only contract probing | **Proposed Specification** | Manual CLI commands | Manual health check procedure (`verify-contract-live.md`) | Scheduled HTTP/JSON-RPC synthetic probe daemon |
| [`dashboards-plan.md`](dashboards-plan.md) | Proposed Grafana dashboard panels, PromQL queries, and metric source schemas | **Future Plan** | None (mockup only) | Blueprint for building visualization panels | Deployed Prometheus server, scraper, and Grafana host |
| [`weekly-health-summary-template.md`](weekly-health-summary-template.md) | Standardized markdown template for reporting testnet operational health | **Active Template** | Markdown template | Operator template for compiling manual weekly reports | Automated script aggregating weekly metrics |
| [`third-party-dependency-status.md`](third-party-dependency-status.md) | External dependency tracking (Stellar SDF Testnet, RPC, Friendbot, Horizon) | **Active Reference** | Public endpoint URLs | Directory of official network status and endpoints | Automated heartbeat probe against upstream dependencies |
| [`false-positive-log.md`](false-positive-log.md) | Incident record for tuning thresholds and logging false alerts | **Active Log Template** | Markdown log (0 entries) | Template for recording alerting mistakes | Running alerting system generating alerts to review |
| [`explorer-links.md`](explorer-links.md) | Curated links to Stellar Expert and Lumenscan for deployed contracts | **Active Reference** | Public explorer URLs | Manual operator drill-down and visual ledger inspection | Automated explorer link generation in incident alerts |

---

## Capabilities Breakdown

### Capabilities that exist today (Manual & Specification layer)

1. **Manual Contract Health Probing**:
   - Operators can execute read-only Soroban RPC checks (`get_status`, `get_info`, `get_nonce`)
     following [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md).
2. **Authoritative Event Topic Encoding**:
   - Precise `ScVal` symbol representations for topics and contract structs documented in
     [`../registry/event-topics.md`](../registry/event-topics.md) and [`event-watchlist.md`](event-watchlist.md).
3. **Manual Balance Maintenance Runbooks**:
   - Clear protocol reserve calculations, hard floors, and Friendbot funding commands documented
     in [`balance-thresholds.md`](balance-thresholds.md) for proposed demo accounts.
4. **Diagnostic Triage Runbooks**:
   - Comprehensive incident handling guides in [`../runbooks/`](../runbooks/) covering signature
     rejections, nonce desynchronization, relayer submission failures, and RPC outages.
5. **Contract Build & Unit Testing Workflows**:
   - Offline contract validation running in GitHub Actions (`cargo test` on contract logic).

### Capabilities that do not exist today (Aspirational layer)

- **No Continuous Daemon**: No long-running background process polling Soroban RPC or Horizon.
- **No Durable Ledger Cursor**: No persistent database tracking `last_processed_ledger`,
  resulting in zero automated event ingestion.
- **No Event Deduplication Pipeline**: No database enforcing `(reset_epoch, ledger, tx_hash, contract_id, event_index)` uniqueness.
- **No Automated Dead-Man Switch**: No alerting engine evaluating absence of `SweepCompleted` or `PaymentReceived` events.
- **No Automated Out-of-Gas Alarms**: No continuous monitoring notifying operators when relayer or demo account balances cross below their hard floors.
- **No Prometheus Metrics Exporter**: No `/metrics` HTTP endpoint exposing testnet counters or gauge metrics.
- **No Live Dashboards**: No active Grafana instance rendering testnet contract states.
- **No Paging or Notification Route**: No integration with PagerDuty, Opsgenie, Discord, or Slack webhooks.
- **No Automated Recovery Reconciliation**: No background job verifying that expired ephemeral accounts have returned their reserves.

---

## Deployment Records & Registry State Warning

A critical prerequisite to any monitoring system is knowing the exact contract addresses to watch.
The testnet registry contains historical inconsistencies:

- Older deployment files and network TOMLs record legacy contract IDs from prior testnet epochs.
- [`../registry/contract-status.md`](../registry/contract-status.md) currently documents that contracts
  must be verified or redeployed following testnet resets.
- Timestamps in historical JSON files do not provide cryptographic ledger proofs or WASM hash bindings.

> [!WARNING]
> Treat every stored contract ID as **unverified** until an operator performs the manual
> verification procedure in [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md).
> An event watcher or uptime probe configured with obsolete contract IDs will report false silence
> or false outages.

---

## Proposed Target Architecture

The target architecture represents the end state to be implemented across future milestones:

```text
Stellar Testnet Horizon                    Soroban RPC (JSON-RPC)
(Classic balances & transfers)             (getEvents, getLatestLedger, getTransaction)
          |                                                    |
          +-------------------------+--------------------------+
                                    |
                                    v
                     Durable Ingestion Collector
                     - Network passphrase validation
                     - Durable ledger cursor persistence
                     - 5-tuple event deduplication
                     - Reset-epoch regression detector
                                    |
          +-------------------------+--------------------------+
          |                         |                          |
          v                         v                          v
  Account Lifecycle         Finalized Sweep            Data Pipeline Health
  - State projection        - Outcome counter          - Cursor lag (<120s)
  - Expiry tracking         - Ratio calculator         - RPC reachability
          |                         |                          |
          +-------------------------+--------------------------+
                                    |
                                    v
                        Prometheus Metrics Exporter
                                    |
                 +------------------+------------------+
                 |                                     |
                 v                                     v
         Grafana Dashboards                   Alert Evaluator Engine
         - Contract lifecycle                 - Silence alerts (Dead-man switch)
         - Canary execution                   - Expiry ratio breaches (E >= 2S)
         - Ingestion health                   - Out-of-gas warnings (< hard floor)
                                                       |
                                                       v
                                            On-Call Triage Runbooks
                                            (Investigate via testnet/runbooks/)
```

---

## Implementation Roadmap: From Aspirational to Active

Transitioning testnet monitoring from specifications into active operations follows a 5-phase plan:

### Phase 0: Specifications, Baselines, and Runbooks (Current State - Complete)
- [x] Authoritative directory index and honest baseline established ([`README.md`](README.md)).
- [x] On-chain signals-of-life and absence-of-activity criteria defined ([`event-watchlist.md`](event-watchlist.md)).
- [x] Expiry-to-sweep ratio and zero-sweep diagnostic thresholds formulated ([`alert-thresholds.md`](alert-thresholds.md)).
- [x] Canary end-to-end operational specification defined ([`synthetic-transaction-plan.md`](synthetic-transaction-plan.md)).
- [x] Demo and test account funding floors and Friendbot runbooks documented ([`balance-thresholds.md`](balance-thresholds.md)).
- [x] Triage runbooks for signature failures, nonce desync, and RPC outages authored in [`../runbooks/`](../runbooks/).

### Phase 1: Read-Only Health Probing & CLI Validation (Immediate Next Step)
- [ ] Build a lightweight CLI probe command (`bridgelet-tail` or `testnet-probe`) based on [`../tooling/event-tail-spec.md`](../tooling/event-tail-spec.md).
- [ ] Verify live contract IDs, interfaces, and WASM hashes on testnet using [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md).
- [ ] Implement manual balance verification scripts against public test accounts.

### Phase 2: Ingestion Pipeline & Durable Ledger Cursor
- [ ] Build a durable event ingestion worker storing cursor sequences in persistent storage.
- [ ] Implement event deduplication using the tuple `(reset_epoch, ledger, tx_hash, contract_id, event_index)`.
- [ ] Implement automated testnet reset detection (head ledger regression trigger).

### Phase 3: Scheduled Synthetic Canary Execution
- [ ] Deploy a dedicated, isolated canary runner executing the workflow in [`synthetic-transaction-plan.md`](synthetic-transaction-plan.md) every 30 minutes.
- [ ] Configure isolated, non-committed signing credentials and dedicated canary destination addresses.
- [ ] Fund canary payer accounts above the working target (50 XLM) with Friendbot auto-refunding.

### Phase 4: Telemetry Exporter, Visualization & Alerting
- [ ] Implement Prometheus metrics exporter exposing event counters, timestamps, and balance gauges.
- [ ] Deploy Grafana dashboards according to the layout in [`dashboards-plan.md`](dashboards-plan.md).
- [ ] Configure Prometheus Alertmanager rules for `TestnetCanarySweepSilence` and `TestnetCanaryPaymentSilence`.
- [ ] Validate alert delivery to non-production notification channels (Slack / Discord).

---

## Independent Audit Procedure: Proving No Monitoring Runs

Any developer, contributor, or external auditor can independently verify that no phantom monitoring
daemons exist in this repository by running the following checks:

1. **Inspect GitHub Actions Workflows**:
   ```bash
   # Check active scheduled workflow triggers (cron)
   grep -rn "schedule:" .github/workflows/
   ```
   *Expected result*: No matches found. All workflows are triggered exclusively by `push`, `pull_request`, or manual `workflow_dispatch`.

2. **Inspect Process and Deployment Tooling**:
   ```bash
   # Search for background daemon configurations or Docker compose services
   ls scripts/
   ```
   *Expected result*: Scripts consist exclusively of build (`build.sh`), test (`test.sh`), link validation (`check-testnet-links.py`), and manual deployment (`deploy-testnet.sh`). No daemon, systemd unit, or container compose file exists.

3. **Inspect Repository Secrets & Configurations**:
   *Result*: No external monitoring tokens, Prometheus pushgateway credentials, or PagerDuty integration keys are committed or configured.

---

## Data-Quality & Anti-Deception Guardrails

To prevent the documentation from drifting back into aspirational claims:

1. **Never Label a Metric "Observed" Without a Ledger Hash**:
   Any documented metric value must link to a finalized transaction hash or ledger sequence.
2. **Never Treat RPC Ingestion Outages as "Zero Events"**:
   Ingestion failure (`rpc_timeout`, `cursor_lag`, `network_reset`) must be reported as a **data-quality failure**, never as contract silence.
3. **Suppress Silence Alerts During Known Outages**:
   Dead-man switch alerts must automatically be suppressed if RPC reachability is false or cursor lag exceeds 120 seconds.
4. **Mark Status Explicitly in Every File**:
   Every file in this directory must maintain a prominent `## Status` callout at the top explicitly stating whether it is an *Active Specification*, *Proposed Policy*, or *Future Plan*.

---

## Activation Checklist

The overall operational status in this directory must remain `not_configured` until all of the following
items are completed and verified:

- [ ] Contract IDs in [`../registry/contract-status.md`](../registry/contract-status.md) verified live on testnet.
- [ ] Ingestion worker deployed with durable ledger cursor and deduplication database.
- [ ] Testnet reset detection and cursor invalidation verified in staging.
- [ ] Synthetic canary runner executing every 30 minutes on testnet.
- [ ] Prometheus metrics endpoint active and scraped by monitoring server.
- [ ] Expiry ratio ($E \ge 2S$) and silence alerts tested with simulated failures.
- [ ] Data-quality suppression guards verified against simulated RPC timeouts.
- [ ] Notification route verified with a non-production test alert.

Until then, the status across this directory is:

```text
testnet_monitoring: not_configured
operational_readiness: specifications_and_runbooks_only
```

---

## Related Documents

- [`event-watchlist.md`](event-watchlist.md) — Signals of life, topic symbols, and absence-of-activity alerting
- [`alert-thresholds.md`](alert-thresholds.md) — Expiry-to-sweep ratio and silence alert criteria
- [`synthetic-transaction-plan.md`](synthetic-transaction-plan.md) — 30-minute end-to-end synthetic canary specification
- [`balance-thresholds.md`](balance-thresholds.md) — Fee-payer funding floors and Friendbot runbooks
- [`uptime-check-spec.md`](uptime-check-spec.md) — RPC liveness and read-only contract health checks
- [`dashboards-plan.md`](dashboards-plan.md) — Visualization panels and metric source plans
- [`weekly-health-summary-template.md`](weekly-health-summary-template.md) — Weekly reporting structure
- [`third-party-dependency-status.md`](third-party-dependency-status.md) — External network status and endpoints
- [`false-positive-log.md`](false-positive-log.md) — Alert tuning and review log
- [`explorer-links.md`](explorer-links.md) — Public block explorer references
- [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md) — Manual contract verification procedure
- [`../registry/contract-status.md`](../registry/contract-status.md) — Current deployment status
- [`../runbooks/rpc-outage.md`](../runbooks/rpc-outage.md) — RPC connectivity triage
- [`../runbooks/upgrade-verification.md`](../runbooks/upgrade-verification.md) — Post-upgrade verification runbook
- [`../../README.md`](../../README.md) — Main repository overview and MVP status
