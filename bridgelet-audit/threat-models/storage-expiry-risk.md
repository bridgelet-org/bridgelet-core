# Threat Model: Storage Expiry (TTL Archival) Risk

**Path:** `bridgelet-audit/threat-models/storage-expiry-risk.md`  
**Component:** `SweepController`, `EphemeralAccount`, `Factory`, `Reserve`  
**Target Operations:** Contract State Management, TTL Extensions

---

## Executive Summary

Soroban implements state expiration mechanisms (Time-To-Live or TTL) for both persistent and temporary storage. If a storage entry's TTL is not properly maintained, the entry is either archived (persistent storage) or permanently deleted (temporary storage). 

This analysis evaluates the risk of storage expiration across the four primary Bridgelet smart contracts, analyzing what happens when contract instances, admin configs, or state variables expire due to network inactivity.

---

## Detailed Threat Scenario & Vulnerability Analysis

### 1. Persistent Storage Expiry (Archival)
**Affected State**: Admin configurations, controller states, initialization flags.
- **Mechanism**: If `env.storage().persistent().extend_ttl()` is not called frequently enough, the network archives the persistent data.
- **Impact**: Any function relying on that data (e.g., `execute_sweep`, `claim`) will trap with a `StorageError` when attempting to read the archived state. The contract becomes temporarily frozen.
- **Recovery**: Anyone can submit a `RestoreFootprintOp` to restore the archived entries. No funds or state data are permanently lost, but availability is degraded.

### 2. Temporary Storage Expiry (Deletion)
**Affected State**: Nonces, ephemeral replay protections.
- **Mechanism**: Temporary storage in Soroban is permanently deleted when its TTL expires. 
- **Impact**: If replay nonces are stored in temporary storage and expire, an attacker could potentially replay old `execute_sweep` signatures.
- **Vulnerability**: If `SweepController` uses temporary storage for the Ed25519 `nonce`, a deleted nonce means the signature can be reused. Bridgelet core mitigates this by using **persistent storage** or **instance storage** for nonces and state tracking, ensuring replay protections are never permanently wiped.

### 3. Instance Expiry
**Affected State**: The WebAssembly contract code and its associated `instance()` storage.
- **Mechanism**: If the contract instance itself is not bumped, the entire contract becomes archived.
- **Impact**: Calls to `EphemeralAccount` or `SweepController` will fail at the network layer before execution begins.
- **Recovery**: A `RestoreFootprintOp` is required to bring the contract back online.

---

## Summary Matrix

| Storage Type | Expiry Consequence | Security Impact | Recovery |
| :--- | :--- | :--- | :--- |
| Persistent | Archival (Inaccessible) | Denial of Service (Low) | `RestoreFootprintOp` |
| Temporary | Permanent Deletion | Replay Attacks (Critical) | None |
| Instance | Archival (Inaccessible) | Denial of Service (Low) | `RestoreFootprintOp` |

---

## Recommended Mitigations

### 1. Aggressive TTL Bumping
Implement `extend_ttl()` calls on all critical read/write paths. For example, during `claim()` or `execute_sweep()`, explicitly bump the TTL of the controller configuration and the Ephemeral Account state to the maximum allowable network limit.

### 2. Avoid Temporary Storage for Security Critical State
Never use `env.storage().temporary()` for cryptographic nonces, replay protection mechanisms, or authorization states. All such data must reside in `persistent()` or `instance()` storage.

### 3. Off-chain Monitoring Watchtower
Deploy an off-chain watchtower service that monitors the TTL of all active `SweepController` and `EphemeralAccount` instances and automatically issues `BumpFootprintOp` transactions when TTL falls below a 30-day threshold.
