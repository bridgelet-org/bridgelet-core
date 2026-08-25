<!--
Purpose: Index and directory guide for the bridgelet-audit/postmortems/ folder.
Owner: @Superray23
Status: Documentation-only. No contracts, docs, scripts, or tools changes are introduced by this file.
-->

# Postmortems Index

| Field | Value |
| :--- | :--- |
| **Category** | Retrospective Security & Engineering Postmortems |
| **Owner / reviewer** | `@Superray23` |
| **Last reviewed** | `2026-08-25` |

---

## Overview

This directory houses retrospective engineering and security postmortems for the Bridgelet Core protocol. 

These documents are **retrospective knowledge-base writeups**, independent of any linked bug-tracking issue or PR. They capture root-cause analyses, worked exploit scenarios, blast radius evaluations, and institutional lessons learned to prevent regressions across the codebase.

This index is designed to be scannable for engineers, auditors, and contributors conducting broader architectural retrospectives across multiple findings.

---

## Postmortems Grouped by Theme

### 1. Replay, Nonce & State Collision

| Document | Severity | Theme | Summary |
| :--- | :--- | :--- | :--- |
| [`account-factory-salt-collision.md`](account-factory-salt-collision.md) | **High** | Salt & Address Collision | Loop index-based salt generation in `AccountFactory::batch_initialize` causes deterministic address collisions across distinct batch deployment transactions. |

---

### 2. Access Control & Authorization

| Focus Area | Theme | Retrospective Key Takeaway |
| :--- | :--- | :--- |
| **Contract Initialization & Roles** | Access Control | Unauthenticated contract initializers allow frontrunning in the mempool; deployment and initialization must be atomic within a single transaction frame. |
| **Controller & Admin Separation** | Access Control | Shared creator/controller/admin roles create single points of failure; high-privilege operations (e.g., WASM upgrades) must be gated separately from operational sweep actions. |

---

### 3. Documentation & Specification Drift

| Focus Area | Theme | Retrospective Key Takeaway |
| :--- | :--- | :--- |
| **Protocol Constants vs State** | Documentation Drift | Discrepancies between hardcoded contract constants (e.g., `BASE_RESERVE_STROOPS`) and dynamic network parameters cause accounting confusion and require explicit documentation sync. |
| **Error Code Numbering** | Documentation Drift | Uncoordinated error discriminant numbering leads to collisions across contracts unless explicitly namespaced and tracked. |

---

### 4. Code Duplication & Consistency

| Focus Area | Theme | Retrospective Key Takeaway |
| :--- | :--- | :--- |
| **Error Enums & Shared Logic** | Code Duplication | Duplicating error codes across individual contract crates causes fragmentation; common error variants must be consolidated in `shared::errors::SharedError`. |
| **Token Transfer Handling** | Code Duplication | Ad-hoc token invocation wrappers risk inconsistent reentrancy guards; standardizing on a shared transfer utility enforces uniform Checks-Effects-Interactions (CEI). |

---

### 5. Dead Code & Storage Invariants

| Focus Area | Theme | Retrospective Key Takeaway |
| :--- | :--- | :--- |
| **Unused State Keys** | Dead Code | Unused storage keys left in instance storage increase byte serialization overhead and contract footprint without providing functional utility. |
| **Orphaned Entrypoints** | Dead Code | Retaining legacy or uncalled helper functions expands contract WASM size and increases the potential audit attack surface. |

---

## Authoring Guidelines for New Postmortems

When adding a new postmortem document to this folder:
1. **Focus on Root Cause**: Document the fundamental design flaw or assumption mismatch, not just the symptom.
2. **Include a Worked Example**: Step through a concrete scenario demonstrating how the issue manifests on-chain.
3. **Generalize Lessons**: Provide actionable rules and preventive guidance applicable to future smart contract development.
4. **Update This Index**: Add a row to the corresponding theme section above.

---

## Related References

- [`bridgelet-audit/README.md`](../README.md) — Main directory index for the `bridgelet-audit/` knowledge base.
- [`bridgelet-audit/threat-models/README.md`](../threat-models/README.md) — Threat model index across contracts and execution flows.
- [`bridgelet-audit/checklists/README.md`](../checklists/README.md) — Go-live and review checklists.
- [`docs/security.md`](../../docs/security.md) — Core security model and protocol invariants.
