# Threat Model: Token Transfer Trust Assumptions

**Path:** `bridgelet-audit/threat-models/token-transfer-trust-assumptions.md`  
**Target Repository:** `bridgelet-org/bridgelet-core`  
**Audience:** Security Auditors, Smart Contract Engineers  

---

## 🎯 Purpose

This document analyzes the trust assumptions made by `execute_transfers` when interacting with external token contracts. It evaluates risks associated with non-standard or malicious token contract implementations during automated sweep operations and details how Soroban host-level mechanics and contract invariants mitigate reentrancy and unexpected behaviors.

---

## 🔍 Core Trust Assumptions in `execute_transfers`

When `execute_transfers` iterates over transfer instructions, it makes several implicit trust assumptions regarding the target token address configured in `payment.asset`:

### 1. Standard Compliance (SEP-41 / SAC Interface)
* **Assumption:** The target address at `payment.asset` implements the standard Stellar Asset Contract (SAC) or [SEP-41 Token Standard](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0041.md) interface (`transfer(from, to, amount)`).
* **Reality:** Soroban dynamic invocations do not enforce static type guarantees on external targets. If `payment.asset` points to an un-permissioned custom contract, arbitrary code will execute in the child call frame upon invocation.

### 2. Predictable State & Side Effects
* **Assumption:** Invoking `transfer` only mutates token balances corresponding to the `from` and `to` parameters and returns cleanly or panics on failure.
* **Reality:** A non-standard token implementation can inject arbitrary logic before, during, or after balance updates.

---

## 🚨 Attack Vectors & Non-Standard Token Behaviors

### 1. Gas Griefing & Resource Exhaustion
* **Mechanism:** A malicious token implementation can consume near-maximum CPU/Memory instructions or host resources within its `transfer` execution path.
* **Impact:** During batch sweeps, one malicious `payment.asset` can exhaust transaction resource limits, reverting the entire batch sweep transaction and denying service to legitimate accounts processed in the same invocation.

### 2. Malicious Panic / Selective Denial of Service
* **Mechanism:** The target token contract conditionally panics based on `msg.sender`, target address, or state conditions (e.g., reverting only when invoked by `SweepController`).
* **Impact:** Reverts the entire calling execution frame, preventing funds from being swept or expired.

### 3. Reentrancy & Control Flow Hijacking
* **Mechanism:** The external token contract attempts a reentrant callback into `SweepController` or `EphemeralAccount` prior to returning from `transfer()`.
* **Impact & Protection:** 
  * As documented in `docs/reentrancy-analysis.md`, Soroban's runtime architecture enforces strict reentrancy protections. 
  * Contracts in `bridgelet-core` utilize non-reentrant state transitions (updating internal lifecycle status to `Swept` or `Expired` before executing external asset calls).
  * Direct reentrant callbacks to mutate state on the calling contract instance will fail due to Soroban's call-stack isolation and internal state locks.

---

## 🔗 Cross-Reference: Reentrancy Protections (`docs/reentrancy-analysis.md`)

For full architectural details on Soroban-level reentrancy mitigation, refer to `docs/reentrancy-analysis.md`. The primary defensive layers protecting `execute_transfers` include:

1. **Checks-Effects-Interactions Pattern:** Internal status flags (e.g., `get_status()`) are updated *prior* to issuing external token `transfer` host calls.
2. **Soroban Invocation Isolation:** Reentrant calls attempting to re-enter state-changing functions on the parent contract fail when trying to re-acquire mutable storage entry locks.

---

## 🛡️ Risk Mitigation Recommendations

1. **Asset Whitelisting / Validation:**
   - Ensure `payment.asset` is validated against known, canonical Stellar Asset Contracts (SAC) or approved SEP-41 implementations during account creation.
2. **Isolated Batch Processing:**
   - Avoid bundling untrusted or arbitrary token transfers in a single atomic batch transaction without per-item error handling or bounded gas allocation.
3. **Strict Parameter Checks:**
   - Verify `amount > 0` before making cross-contract calls to avoid zero-value external invocations.
