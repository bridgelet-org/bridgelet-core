# Glossary Entry: `sweep` vs `sweep_claim`

**Path:** `bridgelet-audit/glossary/sweep-vs-sweep-claim.md`  
**Component:** `EphemeralAccount` & `SweepController`  
**Related Functions:** `SweepController::execute_sweep()`, `SweepController::claim()`, `EphemeralAccount::sweep()`, `EphemeralAccount::sweep_claim()`

---

## Overview

Bridgelet Core provides two execution pathways for sweeping funds from an `EphemeralAccount` to a recipient address:

1. **Off-Chain Signed Sweep (`execute_sweep` / `EphemeralAccount::sweep`)**
2. **Gas-Free Direct Claim (`claim` / `EphemeralAccount::sweep_claim`)**

---

## Direct Comparison Matrix

| Feature / Property | `execute_sweep` (`sweep`) | `claim` (`sweep_claim`) |
| :--- | :--- | :--- |
| **Authentication Scheme** | Ed25519 signature verified in `SweepController` | Soroban native `require_auth()` signed by recipient |
| **Relayer / Fee Payment** | Relayer submits tx with Ed25519 signature payload | Relayer submits tx; recipient signs Soroban auth entry |
| **Mempool Parameter Locking** | `destination` locked cryptographically inside signature | Parameter passed explicitly; subject to frontrunning in flexible mode |
| **Sweep Nonce Update** | Increments `SweepController` nonce on success | Does **not** increment `SweepController` nonce |
| **Destination Lock Constraint** | Checks `authorized_destination` if set | Checks `authorized_destination` if set |
| **Account State Machine** | Transitions `Active`/`PaymentReceived` $\rightarrow$ `Swept` | Transitions `Active`/`PaymentReceived` $\rightarrow$ `Swept` |

---

## Detailed Execution Patterns

### 1. `execute_sweep` Pathway
- Off-chain system signs `SHA256(destination || nonce || controller_id)`.
- Invokes `SweepController::execute_sweep(ephemeral_account, destination, auth_signature)`.
- `SweepController` verifies signature, increments sweep nonce, authorizes downstream invocation, and calls `EphemeralAccount::sweep()`.
- `EphemeralAccount` marks state as `Swept`, reclaims reserve, and `SweepController` executes token transfers.

### 2. `claim` Pathway
- Recipient signs Soroban authorization entry for `SweepController::claim(recipient, ephemeral_account)`.
- Relayer submits transaction and pays gas fees.
- `SweepController` verifies `recipient.require_auth()` and checks `validate_destination()`.
- `SweepController` invokes `EphemeralAccount::sweep_claim(recipient)` via invoker contract authorization context.
- `EphemeralAccount` marks state as `Swept`, reclaims reserve, and `SweepController` executes token transfers.
