# Threat Model: Ephemeral Account Lifecycle

**Path:** `bridgelet-audit/threat-models/ephemeral-account-lifecycle.md`  
**Component:** `EphemeralAccount`  
**Target Operations:** Initialization, Token Receiving, Claiming/Sweeping, Archival

---

## Executive Summary

The `EphemeralAccount` contract represents a temporary custody container for bridging tokens into a user's wallet via a specific Bridgelet controller. It operates strictly on a state machine lifecycle: `Uninitialized -> Active -> Swept`.

This threat model evaluates vulnerabilities associated with state transitions in the ephemeral account, focusing on frontrunning initialization, unauthorized sweeping, locked funds, and replay vulnerabilities.

---

## Detailed Threat Scenario & Vulnerability Analysis

### 1. Initialization Frontrunning (Hijacking)
- **Scenario**: The `AccountFactory` deploys an `EphemeralAccount` but does not atomically initialize it.
- **Threat**: An attacker observes the uninitialized contract in the mempool and calls `initialize()` with their own parameters (e.g., setting themselves as the `controller`).
- **Analysis**: **BLOCKED**. Bridgelet utilizes atomic cross-contract deployment and initialization. The `AccountFactory` deploys the Wasm instance and immediately invokes `initialize()` within the exact same transaction. There is no window for an attacker to hijack the initialization state.

### 2. Post-Sweep Deposit (Fund Locking)
- **Scenario**: An `EphemeralAccount` successfully executes `sweep_claim()` and transitions its state to `Swept`. Later, a user or external system deposits more tokens into the contract's address.
- **Threat**: Tokens become permanently locked because the contract prevents sweeps when the status is `Swept`.
- **Analysis**: **MODERATE RISK**. The `sweep_claim()` function strictly asserts `status == Active`. If tokens arrive after the state transitions to `Swept`, they cannot be claimed via the standard path. 
- **Mitigation**: Implement a `retry_sweep()` or `recover_funds()` function that allows the controller to extract late-arriving tokens even after the primary `Swept` lifecycle event has occurred.

### 3. Controller Spoofing
- **Scenario**: An attacker attempts to call `sweep_claim()` directly on the `EphemeralAccount`, bypassing the `SweepController`.
- **Threat**: Unauthorized draining of funds to an attacker-controlled destination.
- **Analysis**: **BLOCKED**. `sweep_claim()` calls `controller.require_auth()`. Because the `controller` address is immutably set during atomic initialization, only the designated `SweepController` can successfully authorize the execution of `sweep_claim()`.

### 4. Bricking via Token Allowances
- **Scenario**: The `EphemeralAccount` approves the `SweepController` to move its funds, but an attacker drains the allowance or modifies the trustline.
- **Threat**: The sweep fails during execution.
- **Analysis**: **NOT APPLICABLE**. The `EphemeralAccount` natively transfers funds from its own balance to the destination via `token.transfer(env.current_contract_address(), destination, amount)`. It does not rely on allowances, preventing approval-based griefing.

---

## Recommended Mitigations

### 1. Late-Arrival Token Recovery
Add a mechanism to `EphemeralAccount` to either reject incoming transfers when `status == Swept` (difficult on Stellar without trustline removal) or permit the authenticated `controller` to sweep the account multiple times to recover late arrivals.

### 2. State Machine Immutability
Ensure the `status` enum explicitly prevents transitions backwards (e.g., `Swept -> Active`). This is currently enforced, but must be strictly preserved in future contract upgrades to prevent double-spending semantics in external tracking systems.
