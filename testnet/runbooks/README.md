# Testnet Runbooks Index

## Overview

This directory contains operational runbooks for diagnosing and resolving issues with Bridgelet contracts on Stellar Testnet. Each runbook covers a specific failure scenario with step-by-step investigation procedures.

## Runbook Index

| Runbook | Scenario | Related Contracts |
|---|---|---|
| [`account-factory-batch-failure.md`](account-factory-batch-failure.md) | `AccountFactory::batch_initialize` partial failure | AccountFactory, EphemeralAccount |
| [`multi-payment-edge-cases.md`](multi-payment-edge-cases.md) | Repeated `record_payment` for same asset | EphemeralAccount |
| [`failed-sweep-signature.md`](failed-sweep-signature.md) | `SweepController::execute_sweep` signature rejection | SweepController, EphemeralAccount |
| [`stuck-ephemeral-account.md`](stuck-ephemeral-account.md) | EphemeralAccount won't sweep or expire | EphemeralAccount, SweepController |

## Quick Start

1. **Identify the symptom** from the table above
2. **Open the corresponding runbook**
3. **Follow the investigation steps** in order
4. **Escalate** if root cause not found

## Common Prerequisites

All runbooks assume:
- Testnet identity with XLM for queries (`stellar keys generate --global testnet-investigator`)
- Stellar CLI configured for testnet
- Contract IDs from `testnet/registry/contract-status.md`
- Access to Soroban RPC: `https://soroban-testnet.stellar.org`

## Runbook Template

Each runbook follows this structure:
1. **Symptom** - What the user observes
2. **Prerequisites** - Required access/tools
3. **Investigation Steps** - Ordered diagnostic procedures
4. **Common Causes** - Known failure modes
5. **Resolution** - Fix procedures
6. **Escalation** - When to contact core team

## Contributing

To add a new runbook:
1. Create `testnet/runbooks/<name>.md` following the template
2. Add entry to this index table
3. Cross-reference in `testnet/registry/contract-status.md`

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-24 | Initial runbook index |

---

## Runbook directory (complete index)

`testnet/runbooks/` is the single place to start when something goes wrong on testnet. The table below lists every runbook file currently present in this directory, grouped by category. (`README.md` is this index page itself.) On testnet, only `ephemeral_account` and `sweep_controller` are supported; `reserve_contract` and `account_factory` are not deployed, so their runbooks are reference-only until that changes.

### Signature / sweep

| Runbook | Use this when |
|---|---|
| [`failed-sweep-signature.md`](failed-sweep-signature.md) | `execute_sweep` rejects a signature (`SignatureVerificationFailed`); check nonce, contract ID, and destination. |
| [`nonce-desync.md`](nonce-desync.md) | The signer's nonce has drifted from the controller's `get_nonce()` or concurrent sweeps caused a race. |
| [`relayer-submission-failures.md`](relayer-submission-failures.md) | A signed sweep or gas-free claim cannot be submitted by the relayer (fees, auth entry, or submission errors). |
| [`reentrancy-suspicion.md`](reentrancy-suspicion.md) | Sweep/transfer behavior suggests a possible reentrancy or status-ordering problem. |

### Account lifecycle

| Runbook | Use this when |
|---|---|
| [`stuck-ephemeral-account.md`](stuck-ephemeral-account.md) | A payment was received but sweep/claim fails, or `is_expired()` never becomes true. |
| [`expired-account-recovery-check.md`](expired-account-recovery-check.md) | An account has reached `expiry_ledger` and recovery to the fallback address must be verified. |
| [`multi-payment-edge-cases.md`](multi-payment-edge-cases.md) | `record_payment` is called repeatedly for the same asset or multiple assets are involved. |
| [`account-factory-batch-failure.md`](account-factory-batch-failure.md) | A `batch_initialize` returns partial failure with `error: None` (reference-only; not deployed on testnet). |
| [`insufficient-reserve-errors.md`](insufficient-reserve-errors.md) | Reserve/rent exemption errors appear during init, sweep, or reclaim (reference-only; reserve contract not deployed). |

### Infrastructure

| Runbook | Use this when |
|---|---|
| [`rpc-outage.md`](rpc-outage.md) | Soroban RPC or Horizon is unreachable or returning errors, blocking queries and submissions. |
| [`contract-not-found-error.md`](contract-not-found-error.md) | A contract ID is unknown on testnet, often because the contract is not deployed or the ID is from another network. |
| [`wrong-network-passphrase.md`](wrong-network-passphrase.md) | Calls fail or hit the wrong network because the passphrase/network setting is incorrect. |
| [`event-not-appearing-in-horizon.md`](event-not-appearing-in-horizon.md) | Expected contract events (for example `SweepCompleted`) are missing from event queries or indexers. |
| [`upgrade-verification.md`](upgrade-verification.md) | You need to confirm the deployed WASM hash matches the expected build/commit after an upgrade. |

### Process

| Runbook | Use this when |
|---|---|
| [`on-call-testnet.md`](on-call-testnet.md) | You are taking or handing off the testnet on-call shift and need escalation contacts and rotation steps. |
| [`incident-postmortem-template.md`](incident-postmortem-template.md) | An incident is resolved and you need to write it up using the standard postmortem template. |

### Start here: on-call flow

1. **Confirm the network** — check [`wrong-network-passphrase.md`](wrong-network-passphrase.md) and that you are using the testnet `sweepController`/`ephemeralAccount` IDs from `testnet/registry/contract-status.md`.
2. **Confirm basic connectivity** — if queries or submissions fail wholesale, go to [`rpc-outage.md`](rpc-outage.md) before debugging contract logic.
3. **Confirm the contract exists** — if a call reports no contract, use [`contract-not-found-error.md`](contract-not-found-error.md); remember `reserve_contract` and `account_factory` are not deployed on testnet.
4. **Match the symptom** — signature rejection → [`failed-sweep-signature.md`](failed-sweep-signature.md) (or [`nonce-desync.md`](nonce-desync.md)); account won't sweep/expire → [`stuck-ephemeral-account.md`](stuck-ephemeral-account.md); payment edge cases → [`multi-payment-edge-cases.md`](multi-payment-edge-cases.md).
5. **Check events** — if the transaction succeeded but nothing observable changed, use [`event-not-appearing-in-horizon.md`](event-not-appearing-in-horizon.md).
6. **Escalate and record** — follow the runbook's escalation step, log the shift in [`on-call-testnet.md`](on-call-testnet.md), and write up significant incidents with [`incident-postmortem-template.md`](incident-postmortem-template.md).
