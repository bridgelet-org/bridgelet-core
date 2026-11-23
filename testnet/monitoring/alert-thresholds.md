# Testnet Alert Policy Proposal: Sustained Rise in Account Expirations

## Status

> **Planning Document Only:** This document proposes a concrete alerting policy for testnet monitoring. No automated alert evaluator, pager, or collector is currently active in this repository. This specification defines the operational thresholds, time windows, and triage procedures for a future monitoring implementation.

---

## Purpose & Problem Statement

In a healthy Bridgelet integration on testnet, ephemeral accounts receive payments and are subsequently swept by an off-chain relayer or automated worker before reaching their configured `expiry_ledger`. When an account is swept successfully, the `SweepController` contract emits a `SweepCompleted` event.

Conversely, if an account passes its `expiry_ledger` without being swept, an operator or automated fallback must trigger `expire()` or `recover()`, which transitions the account to `Expired` and emits an `AccountExpired` event.

### Why a Sustained Shift Matters

While occasional expirations are normal (e.g. during manual testing or abandoned payment flows), a **sustained shift toward more accounts expiring unswept than being successfully swept** is a critical operational signal indicating:

1. **Integration-Side Bug or Relayer Outage:**
   - A partner relayer or sweeper daemon has crashed, hung, or lost network connectivity.
   - The relayer account has run out of testnet XLM to pay transaction fees.
   - The off-chain signer's nonce tracking has fallen out of sync with `SweepController` (see [`../runbooks/nonce-desync.md`](../runbooks/nonce-desync.md)), causing all sweep signatures to fail.
   - Signing keys were rotated incorrectly.
2. **Contract Regression:**
   - A newly deployed WASM contract version introduced a revert inside `execute_sweep()`, `sweep()`, or token transfer logic.
   - Authorization gates (`require_auth`) are rejecting valid controller invocations.
   - Resource limits (CPU instructions or memory bytes) are exceeded during sweep execution.
3. **Network Congestion or Fee Spikes:**
   - Relayer fee bids are insufficient to land transactions within the ledger window before `expiry_ledger`.

Detecting this shift early prevents customer funds from remaining stuck or falling back to the recovery address unexpectedly.

---

## Monitored Signals & Topics

The policy monitors two primary Soroban event topics from finalized ledgers:

| Signal | Source Contract | Event Topic | Event Payload Struct | Meaning |
|---|---|---|---|---|
| **$E$ (Expirations)** | `EphemeralAccount` | `expired` | `AccountExpired` | An account expired and executed the recovery fallback path via `expire()` or `recover()`. |
| **$S$ (Sweeps)** | `SweepController` | `sweep` | `SweepCompleted` | A sweep was successfully verified and executed via `execute_sweep()` (or claimed via `claim()`). |

### Event Collection & Deduplication Invariants

- **Deduplication Key:** Events must be uniquely identified by the tuple:
  ```text
  (contract_id, ledger_sequence, transaction_hash, event_index)
  ```
- **Scope Isolation:** Only count events originating from verified, active contract deployments (see [`../registry/contract-status.md`](../registry/contract-status.md)).
- **Do Not Double Count:** The child `EphemeralAccount` emits a `swept_mul` event (topic `swept_mul`) during a sweep. Do **not** count `swept_mul` as a separate sweep event; monitor only the controller's `sweep` topic for $S$.

---

## Proposed Ratio-Based Alert Thresholds

### Primary Alert: `sustained_expiry_to_sweep_breach`

Trigger an alert when the ratio of expirations to sweeps exceeds the threshold continuously over the specified evaluation window.

```text
E >= 2.0 * S
AND
E + S >= 10
CONDITION SUSTAINED FOR: 15 consecutive minutes
```

### Parameter Specification

| Parameter | Specification | Operational Justification |
|---|---:|---|
| **Lookback Window** | **30 minutes** (rolling) | Sufficient sample window to absorb short-term traffic spikes or bursty deposit activity. |
| **Evaluation Interval** | **5 minutes** | Evaluated every 5 minutes against finalized ledger ranges. |
| **Ratio Threshold ($R$)** | **$\ge 2.0$** ($E \ge 2 \times S$) | Flags when at least 2 accounts expire for every 1 successful sweep. Under normal operations, $E / S$ is typically $< 0.1$. |
| **Minimum Sample Volume** | **$\ge 10$ total events** ($E + S \ge 10$) | Prevents false positives during quiet testnet periods (e.g. 1 expiration and 0 sweeps during an idle hour). |
| **Sustained Duration** | **15 consecutive minutes** | Requires the condition to be true across 3 consecutive 5-minute evaluations, filtering transient test batches or one-off manual tests. |
| **Resolve Condition** | **15 consecutive minutes** below threshold | Alert auto-resolves only when the ratio remains $< 2.0$ for 3 consecutive evaluation cycles. |

---

## Edge Case: Zero-Sweep Silence (`S == 0`)

When no sweeps occur during the window ($S = 0$), the ratio $\frac{E}{S}$ is mathematically undefined. This must **not** be treated as a null value or divide-by-zero error.

A dedicated silence alert must be evaluated:

```text
S == 0
AND
E >= 10
CONDITION SUSTAINED FOR: 15 consecutive minutes
```

- **Alert Name:** `sweep_completion_silence`
- **Severity:** High / Pager
- **Operational Meaning:** Accounts are actively timing out and expiring, but **zero sweeps** are succeeding. This strongly points to a complete relayer failure, an authentication mismatch, or a broken sweep contract method.

If both $E == 0$ and $S == 0$, the system is considered idle; no alert is generated.

---

## Event Ingestion & Querying

A future collector or monitoring daemon can query finalized event streams via Soroban RPC:

```bash
# Query AccountExpired events (topic: "expired")
stellar contract events \
  --contract-id <EPHEMERAL_ACCOUNT_ID> \
  --start-ledger <START_LEDGER> \
  --filter '{"topics":[["expired"]]}' \
  --network testnet

# Query SweepCompleted events (topic: "sweep")
stellar contract events \
  --contract-id <SWEEP_CONTROLLER_ID> \
  --start-ledger <START_LEDGER> \
  --filter '{"topics":[["sweep"]]}' \
  --network testnet
```

---

## Alert Payload Specification

When an alert triggers, the monitoring webhook or notification payload should contain:

```json
{
  "alert_name": "sustained_expiry_to_sweep_breach",
  "severity": "CRITICAL",
  "environment": "testnet",
  "timestamp_utc": "2026-11-23T20:00:00Z",
  "window": {
    "lookback_minutes": 30,
    "start_ledger": 1245600,
    "end_ledger": 1245960
  },
  "metrics": {
    "account_expired_count": 18,
    "sweep_completed_count": 4,
    "ratio": 4.5,
    "threshold": 2.0,
    "sustained_minutes": 15
  },
  "contracts": {
    "sweep_controller": "CAJB4RRM5JRPXLITC5JUA7Q3LU2WQJDRPXVKMORXWHJJBPZRYWDJ5V4S",
    "ephemeral_account_template": "CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3"
  },
  "runbook_links": [
    "testnet/runbooks/nonce-desync.md",
    "testnet/runbooks/failed-sweep-signature.md",
    "testnet/runbooks/expired-account-recovery-check.md"
  ]
}
```

---

## Incident Response & Triage Workflow

When `sustained_expiry_to_sweep_breach` or `sweep_completion_silence` fires, the on-call engineer should follow this checklist:

```text
[Alert Fired: Expiry-to-Sweep Breach]
                 │
                 ▼
1. Check Relayer Health
   ├── Is the relayer process running?
   ├── Is the relayer wallet funded with XLM?
   └── Are relayer logs showing submission rejections?
                 │
                 ▼
2. Check for Nonce Desync
   ├── Query SweepController::get_nonce() directly.
   ├── Compare with relayer's cached nonce.
   └── If drifted, apply testnet/runbooks/nonce-desync.md.
                 │
                 ▼
3. Check Soroban RPC & Network
   ├── Is testnet experiencing elevated fee surges?
   └── Are transactions timing out before ledger close?
                 │
                 ▼
4. Check Contract Integrity
   ├── Review testnet/registry/upgrade-history.md for recent deployments.
   ├── Verify WASM hash on-chain against expected build.
   └── Check for recent contract reverts or bug reports.
```

---

## Baseline Calibration & Rollout Plan

Before enabling paging alerts based on this specification:

1. **Deploy Collector in Report-Only Mode:** Run the event collector for at least **7 consecutive days** without paging.
2. **Establish Normal Ratios:** Determine baseline $E / S$ ratios during normal automated integration tests and partner sandbox runs.
3. **Simulate Failure Injection:**
   - Stop a test relayer and verify that `sweep_completion_silence` fires after 15 minutes.
   - Submit batch expirations to verify that isolated batches ($< 15$ min) do not cause alert flapping.
4. **Sign-off:** Obtain approval from core maintainers and integration partners before configuring PagerDuty routes.

---

## Related Documentation

- [`README.md`](README.md) — Monitoring overview and observability architecture
- [`explorer-links.md`](explorer-links.md) — Pre-filtered explorer links for testnet contracts
- [`../runbooks/nonce-desync.md`](../runbooks/nonce-desync.md) — Remediation for relayer signature and nonce drift
- [`../runbooks/expired-account-recovery-check.md`](../runbooks/expired-account-recovery-check.md) — Verifying recovery address fallback behavior
- [`../runbooks/failed-sweep-signature.md`](../runbooks/failed-sweep-signature.md) — Triage for rejected sweep signatures
- [`../registry/event-topics.md`](../registry/event-topics.md) — Event topics reference for Soroban contracts
