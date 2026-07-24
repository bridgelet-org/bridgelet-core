# Threat Model: Replay and Nonce Protections Across Bridgelet Core

**Path:** `bridgelet-audit/threat-models/replay-nonce-protections.md`  
**Component:** System-wide (`SweepController`, `EphemeralAccount`, `AccountFactory`)  
**Cross-References:**
- [reserve-reclaim.md](file:///c:/Users/g-obiagazie/Desktop/bridgelet-core/bridgelet-audit/glossary/reserve-reclaim.md)
- [sweep-nonce.md](file:///c:/Users/g-obiagazie/Desktop/bridgelet-core/bridgelet-audit/glossary/sweep-nonce.md)
- [sweep-vs-sweep-claim.md](file:///c:/Users/g-obiagazie/Desktop/bridgelet-core/bridgelet-audit/glossary/sweep-vs-sweep-claim.md)

---

## Executive Summary

Replay attacks represent a primary vector for smart contract exploits, wherein an attacker captures a valid payload or signature and re-submits it to execute unauthorized operations or double-spend assets. Bridgelet Core implements a multi-layered defense using three distinct replay-prevention mechanisms operating at different layers of the stack:

1. **Sweep Nonce** (`SweepController`): Monotonically increasing on-chain sequence counter protecting off-chain Ed25519 signature authorizations.
2. **`AlreadySwept` State Guard** (`EphemeralAccount`): On-chain state machine enforcing single-use execution per ephemeral account instance.
3. **Deployment Salt** (`AccountFactory`): Deterministic salt derivation used for WASM contract address creation.

---

## Mechanism Breakdown & Coverage Matrix

### 1. Sweep Nonce Mechanism
- **Scope & Storage**: Stored as `u64` in `SweepController` instance storage (`init_sweep_nonce`, `get_sweep_nonce`).
- **Operation**: Included in SHA256 digest: `SHA256(destination || nonce || controller_id)`. Upon successful signature verification in `execute_sweep()`, `nonce` increments by 1.
- **Threat Mitigated**: Prevents an attacker from capturing an Ed25519 signature payload submitted in transaction $T_1$ and replaying it in transaction $T_2$ to sweep another account or re-sweep the same controller.

### 2. `AlreadySwept` Status Guard Mechanism
- **Scope & Storage**: Stored as `AccountStatus` enum in `EphemeralAccount` contract storage.
- **Operation**: Initialized to `Active` (0), transitions to `PaymentReceived` (1) on first payment, and explicitly transitions to `Swept` (2) or `Expired` (3) before token transfers or reserve reclaims execute.
- **Threat Mitigated**: Prevents double-sweeping of an `EphemeralAccount`. Any subsequent attempt to invoke `sweep()` or `sweep_claim()` on an already-swept account immediately fails with `Error::AlreadySwept`.

### 3. Deployment Salt Mechanism
- **Scope & Storage**: Derived in `AccountFactory::batch_initialize` via `env.deployer().with_current_contract(salt).deploy_v2(...)`.
- **Operation**: Constructing 32-byte salt `salt_bytes[28..32] = index.to_be_bytes()` to deterministically calculate new contract addresses.
- **Threat Mitigated**: Prevents address collision within a single batch deployment call.

---

## Complete Matrix: Coverage by System Flow

| Flow / System Operation | Sweep Nonce Covered? | `AlreadySwept` Guard Covered? | Deployment Salt Covered? | Status & Vulnerability Analysis |
| :--- | :--- | :--- | :--- | :--- |
| **`execute_sweep()`** | **Yes** (Verifies & Increments) | **Yes** (`EphemeralAccount` checks state) | N/A | **Fully Covered**: Signature replay impossible; double-sweep blocked. |
| **`claim()` (Gas-Free Path)** | **No** (Nonce is not incremented) | **Yes** (`EphemeralAccount` checks state) | N/A | **Partially Covered**: State guard prevents double-sweep, but un-incremented nonce leaves `update_authorized_destination` check unlocked. |
| **`record_payment()`** | N/A | N/A | N/A | **Partially Covered**: Guarded against duplicate assets (max 10), but accepts arbitrary inbound caller deposits. |
| **`expire()` / `recover()`** | N/A | **Yes** (Fails if `Swept` or `Expired`) | N/A | **Fully Covered**: Immutable state transition prevents re-expiration or double reserve reclaim. |
| **`reclaim_reserve()`** | N/A | **Yes** (State machine + `remaining_reserve` tracking) | N/A | **Fully Covered**: Idempotent payout formula $\min(\text{available}, \text{remaining})$ prevents double-payout. |
| **`batch_initialize()`** | N/A | N/A | **Partial** (Index-scoped only) | **Gap Identified**: Index-based salt `0..N` repeats across separate batch transactions, causing salt collision if factory is invoked multiple times. |

---

## Identified Coverage Gaps & Architectural Recommendations

### Gap 1: `claim()` Path Does Not Increment Sweep Nonce
- **Analysis**: `SweepController::claim()` invokes `authorize_claim()` and calls `EphemeralAccount::sweep_claim()`. It skips `authorization::increment_nonce()`.
- **Impact**: `update_authorized_destination()` checks `nonce > 0` to verify if sweeps have occurred. If all sweeps were executed via `claim()`, `nonce` remains `0`, allowing the creator to mutate `authorized_destination` after funds have already been claimed.
- **Mitigation**: Update `claim()` to increment `sweep_nonce` or implement a dedicated `swept_count` flag on `SweepController`.

### Gap 2: Cross-Transaction Salt Collision in `AccountFactory`
- **Analysis**: `batch_initialize()` constructs `salt_bytes[28..32] = (index as u32).to_be_bytes()`. On a subsequent call to `batch_initialize()`, `index` resets to `0`, generating identical salts.
- **Impact**: Deploying contracts with identical salts under the same factory address causes transaction failure or deployment collisions on ledger.
- **Mitigation**: Mix a global factory deployment counter or unique request hash into `salt_bytes`:
  $$\text{salt} = \text{SHA256}(\text{factory\_nonce} \mathbin{\Vert} \text{request\_hash} \mathbin{\Vert} \text{index})$$

---

## Summary & Cross-References

For deep dives into individual lifecycle components, refer to:
- [sweep-nonce.md](file:///c:/Users/g-obiagazie/Desktop/bridgelet-core/bridgelet-audit/glossary/sweep-nonce.md) for full specification of Ed25519 signature payload hashing.
- [sweep-vs-sweep-claim.md](file:///c:/Users/g-obiagazie/Desktop/bridgelet-core/bridgelet-audit/glossary/sweep-vs-sweep-claim.md) for structural differences between signed sweeps and gas-free claims.
- [reserve-reclaim.md](file:///c:/Users/g-obiagazie/Desktop/bridgelet-core/bridgelet-audit/glossary/reserve-reclaim.md) for the reserve state machine.
