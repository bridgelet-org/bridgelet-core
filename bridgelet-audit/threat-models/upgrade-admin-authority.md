# Threat Model: Upgrade & Admin Authority

**Path:** `bridgelet-audit/threat-models/upgrade-admin-authority.md`  
**Target Repository:** `bridgelet-org/bridgelet-core`  
**Audience:** Security Auditors, Smart Contract Engineers, Protocol Governance  

---

## 🎯 Purpose

This document provides a systematic threat model evaluating administrative authority across all contracts in the `bridgelet-core` workspace. It documents the exact privileged entrypoints exposed by each contract, notes where upgrade capabilities do or do not exist, and analyzes the blast radius if an admin private key is compromised.

---

## 🏛️ Administrative Capability Summary Across Workspace

| Contract | Admin Role Present? | Upgrade Function (`upgrade()`) | Administrative Operations | Blast Radius Severity |
| :--- | :--- | :--- | :--- | :--- |
| **`EphemeralAccountContract`** | Yes | **Yes** (`upgrade()`) | WASM bytecode upgrade | 🚨 **Critical** |
| **`ReserveContract`** | Yes | **No** | Parametric adjustment (`set_base_reserve()`) | ⚠️ **Medium** |
| **`AccountFactory`** | No / Minimal | **No** | Factory registry / WASM deployment | ℹ️ **Low** |
| **`SweepController`** | No / Minimal | **No** | Permissionless batch sweeps | ℹ️ **Low** |

---

## 🔍 Detailed Contract-by-Contract Breakdown

### 1. `EphemeralAccountContract`

#### Privileged Entrypoints
* **`upgrade(new_wasm_hash: BytesN<32>)`**: Admin-gated function allowing the administrative address to replace the running WASM executable bytecode with a new compiled WASM binary.

#### Key Compromise Impact Analysis

* **What a compromised admin CAN affect:**
  * **Arbitrary Code Execution**: An attacker with administrative credentials can call `upgrade()` with a malicious WASM hash.
  * **Fund Exfiltration**: The malicious upgraded WASM code can bypass authorization checks in `sweep()`, `recover()`, or `reclaim_reserve()`, directly draining all escrowed or active funds held across all account instances sharing the upgraded executable code.
  * **State Invalidation / Denial of Service**: The attacker can brick accounts by deploying a failing/panicking WASM binary, locking user assets indefinitely.

* **What a compromised admin CANNOT affect:**
  * Past transaction history recorded on the Stellar ledger (on-chain immutable history).
  * Storage data structures, unless the newly deployed malicious code explicitly overwrites or clears instance storage key-values.

---

### 2. `ReserveContract`

#### Privileged Entrypoints
* **`set_base_reserve(new_reserve: i128)`**: Admin-gated function allowing the protocol admin to modify the minimum reserve requirement enforced during account creation and settlement.

#### Key Compromise Impact Analysis

* **What a compromised admin CAN affect:**
  * **Griefing / Denial of Service via Reserve Price**: Setting `new_reserve` to an arbitrarily high value (e.g., `i128::MAX`) will cause subsequent account creation attempts to fail due to insufficient user balance.
  * **Economic Friction / Dust Generation**: Setting `new_reserve` to `0` may allow creation of undercollateralized accounts, potentially exhausting storage limits or leaving uncollected dust entries in state.

* **What a compromised admin CANNOT affect:**
  * **Direct Asset Theft**: Modifying `base_reserve` does not give the admin direct access to transfer existing reserve tokens out of the contract storage instance.
  * **Existing Account Execution**: Already funded accounts operate under previously committed ledger reserves unless explicitly queried against updated parameters.
  * **WASM Upgrades**: `ReserveContract` does not expose an `upgrade()` entrypoint; code logic remains immutable unless re-deployed at a new contract address.

---

### 3. `AccountFactory` & `SweepController`

#### Fact Observation
* Factually, neither **`AccountFactory`** nor **`SweepController`** exposes an `upgrade()` path or administrative code migration interface.

#### Security & Operational Implications

* **Immutability Assurance**: The logic governing batch processing (`SweepController`) and instance creation (`AccountFactory`) is fully immutable upon deployment.
* **Blast Radius Reduction**: An admin key leak elsewhere in the system cannot alter the logic of `SweepController` or `AccountFactory`.
* **Migration Strategy**: If a vulnerability or optimization requires updating `AccountFactory` or `SweepController`, operators must deploy a completely new contract instance and migrate off-chain indexing or routing configurations to the new contract address.

---

## 🛡️ Risk Mitigation Recommendations

1. **Multi-Sig / Timelock Governance**:
   - The admin key for `EphemeralAccountContract` must be held by a multi-signature threshold wallet (e.g., 3-of-5) combined with an on-chain timelock to allow users time to exit before code changes take effect.
2. **Key Separation**:
   - Separate the administrative key for `ReserveContract` parameter adjustments from the high-privilege WASM upgrade key for `EphemeralAccountContract`.
3. **Immutable Deployment Considerations**:
   - Evaluate whether `EphemeralAccountContract::upgrade()` should eventually be disabled (renouncing ownership) once the protocol reaches full operational stability.
