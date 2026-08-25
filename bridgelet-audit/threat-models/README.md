<!--
Purpose: Index and directory guide for the bridgelet-audit/threat-models/ folder.
Owner: @chinweobtagaz
Status: Documentation-only. No contracts, docs, scripts, or tools changes are introduced by this file.
-->

# Threat Models Index

| Field | Value |
| :--- | :--- |
| **Category** | Security Threat Models Index |
| **Baseline Document** | [`docs/security.md`](../../docs/security.md) |
| **Owner / reviewer** | `@chinweobtagaz` |
| **Last reviewed** | `2026-08-25` |

---

## Overview

This directory contains in-depth threat models and attack vector analyses for each smart contract and cross-contract subsystem in the Bridgelet Core protocol. 

These documents extend the architectural security guarantees established in [`docs/security.md`](../../docs/security.md). While [`docs/security.md`](../../docs/security.md) provides the primary specification of system security assumptions, the documents in this folder dive deep into specific execution flows, trust boundaries, failure modes, and mitigation strategies.

This index routes security researchers, auditors, and protocol engineers to the relevant threat model document.

---

## Threat Models by Component

### 1. `ephemeral_account`

| Document | Focus & Scope Summary |
| :--- | :--- |
| [`ephemeral-account-lifecycle.md`](ephemeral-account-lifecycle.md) | Evaluates state machine transitions (`Uninitialized -> Active -> Swept`), initialization frontrunning, post-sweep deposits, and controller spoofing. |
| [`upgrade-admin-authority.md`](upgrade-admin-authority.md) | Evaluates administrative authority, the `upgrade()` entrypoint, and the blast radius of an admin key compromise on ephemeral accounts. |

---

### 2. `sweep_controller`

| Document | Focus & Scope Summary |
| :--- | :--- |
| [`sweep-controller-signature-flow.md`](sweep-controller-signature-flow.md) | Analyzes Ed25519 signature payload hashing, replay attacks, nonce sequencing, and destination tampering in `execute_sweep()`. |
| [`sweep-controller-claim-flow.md`](sweep-controller-claim-flow.md) | Evaluates Soroban native authorization (`require_auth()`), relayer submission, fee-bumping, and argument binding in `claim()`. |
| [`frontrunning-risk.md`](frontrunning-risk.md) | Analyzes mempool frontrunning vectors on `execute_sweep()` and `claim()`, contrasting locked mode vs. flexible mode security. |
| [`token-transfer-trust-assumptions.md`](token-transfer-trust-assumptions.md) | Evaluates trust assumptions and risk vectors (gas griefing, panics, reentrancy) when interacting with external SEP-41 token contracts during sweeps. |

---

### 3. `account_factory`

| Document | Focus & Scope Summary |
| :--- | :--- |
| [`account-factory-deployment-flow.md`](account-factory-deployment-flow.md) | Analyzes batch deployment trust assumptions, unauthenticated WASM hash initialization, and deterministic salt derivation collision risks. |

---

### 4. `reserve_contract`

| Document | Focus & Scope Summary |
| :--- | :--- |
| [`reserve-contract-config-flow.md`](reserve-contract-config-flow.md) | Analyzes admin access control, parameter validation bounds, and downstream system risks when configuring the system-wide base reserve. |

---

### 5. Cross-Contract & System-Wide

| Document | Focus & Scope Summary |
| :--- | :--- |
| [`replay-nonce-protections.md`](replay-nonce-protections.md) | Provides a comprehensive matrix and coverage analysis of multi-layered replay protections (sweep nonces, `AlreadySwept` state guard, deployment salts) across all contracts. |
| [`storage-expiry-risk.md`](storage-expiry-risk.md) | Analyzes Soroban TTL expiration and state archival risks across persistent, temporary, and instance storage. |
| [`cross-contract-trust-boundaries.md`](cross-contract-trust-boundaries.md) | Maps cross-contract call graphs and trust boundaries between factory, controller, ephemeral account, and external token contracts. |

---

## Related References

- [`docs/security.md`](../../docs/security.md) — Baseline security model, authorization flows, and core protocol invariants.
- [`docs/reentrancy-analysis.md`](../../docs/reentrancy-analysis.md) — Reentrancy protection patterns and Soroban runtime execution model.
- [`docs/cross-contract-safety.md`](../../docs/cross-contract-safety.md) — Cross-contract call security patterns and error propagation.
- [`bridgelet-audit/README.md`](../README.md) — Main index for the `bridgelet-audit/` knowledge base.
