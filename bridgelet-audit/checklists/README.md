<!--
Purpose: Index and directory guide for the bridgelet-audit/checklists/ folder.
Owner: @Superray23
Status: Documentation-only. No contracts, docs, scripts, or tools changes are introduced by this file.
-->

# Checklists Index

| Field | Value |
| :--- | :--- |
| **Category** | Operations & Security Review Checklists |
| **Owner / reviewer** | `@Superray23` |
| **Last reviewed** | `2026-08-25` |

---

## Overview

This directory provides structured, binary checklists for operators, security reviewers, and deployment engineers. Checklists ensure that critical deployment decisions, custody parameters, and contract state configurations are explicitly verified and recorded before contracts go live on public networks.

---

## Checklists Grouped by Frequency

### 1. One-Time (Pre-Deployment & Go-Live) Checklists

These checklists are executed before a contract deployment, initialization, or mainnet upgrade. Every item must be marked complete and signed off before production traffic is permitted.

| Checklist | Component | "Use this when..." Summary |
| :--- | :--- | :--- |
| [`mainnet-readiness-ephemeral-account.md`](mainnet-readiness-ephemeral-account.md) | `EphemeralAccount` | Preparing to deploy or upgrade the `EphemeralAccount` contract on Stellar mainnet; validates signature verification, reserve handling, state transitions, access control, and event emission. |
| [`sweep-controller-initialization-checklist.md`](sweep-controller-initialization-checklist.md) | `SweepController` | Deploying and initializing a new `SweepController` instance; confirms destination mode (locked vs. flexible), signer custody, and authorized-controller mapping before go-live. |

---

### 2. Recurring (Periodic Review & Operational) Checklists

These checklists are executed on a recurring cadence (e.g., weekly, monthly, or quarterly) by operators maintaining live protocol infrastructure.

| Review Area | Cadence | "Use this when..." Summary |
| :--- | :--- | :--- |
| **Storage TTL & Footprint Audit** | Weekly / Monthly | Monitoring the Time-To-Live (TTL) of active contract instances and persistent storage entries to ensure automated bump operations prevent ledger archival. |
| **Authorized Signer Key Custody Audit** | Quarterly | Reviewing HSM/KMS access control logs, IAM role policies, and key separation for off-chain Ed25519 sweep signers. |
| **Testnet Dependency & Version Sync** | Per Release | Verifying Soroban SDK dependency pins and network protocol version compatibility across all workspace crates before staging deployments. |

---

## Authoring Convention for Checklist Files

All documents in this folder must adhere to the **literal-checkbox formatting standard**:

1. **Checkbox Syntax**: Use markdown task list items verbatim:
   ```markdown
   - [ ] **Action title in bold** — Description of the exact on-chain property or operational state to verify.
   ```
2. **Binary Verification**: Each checklist item must be strictly binary (a definitive `yes` or `no`). If a requirement has sequenced sub-steps, author it as an operational procedure in `bridgelet-audit/runbooks/` instead.
3. **No Unactionable Commentary**: Keep each item direct, specifying what command to run, what value to inspect, or what condition blocks go-live.
4. **Sign-Off Table**: Pre-deployment checklists must conclude with a two-person sign-off table (Operator and Reviewer) with timestamp and commit SHA fields.

---

## Related References

- [`bridgelet-audit/README.md`](../README.md) — Main index for the `bridgelet-audit/` knowledge base.
- [`bridgelet-audit/runbooks/README.md`](../runbooks/validate-batch-initialize-salt-uniqueness.md) — Step-by-step operational runbooks.
- [`bridgelet-audit/threat-models/README.md`](../threat-models/README.md) — Threat models and attack vector analyses.
- [`docs/security.md`](../../docs/security.md) — Baseline protocol security model.
