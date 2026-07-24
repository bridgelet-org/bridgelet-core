# Threat Model: Mempool Frontrunning Risk Analysis on Sweep Transactions

**Path:** `bridgelet-audit/threat-models/frontrunning-risk.md`  
**Component:** `SweepController` & `EphemeralAccount`  
**Target Operations:** `execute_sweep()`, `claim()`

---

## Executive Summary

When a sweep transaction (`execute_sweep` or `claim`) is broadcast to the Stellar network, it enters the unconfirmed transaction pool (mempool) before ledger inclusion. The parameters—including `ephemeral_account`, `destination`, and signature payloads—are publicly visible to network nodes and potential frontrunners. 

This analysis evaluates two primary threat vectors:
1. **Payment Interception**: Can a frontrunner modify or copy a broadcast sweep transaction to divert funds to an attacker-controlled address?
2. **Claim Preemption**: Can a frontrunner claim an active `EphemeralAccount` to their own wallet before the legitimate recipient's transaction lands on ledger?

---

## Detailed Threat Scenario & Vulnerability Analysis

### Threat Scenario 1: Intercepting Payment in `execute_sweep`
- **Mechanism**: The off-chain SDK/relayer submits `SweepController::execute_sweep(ephemeral_account, destination, auth_signature)`.
- **Mempool Exposure**: Attacker observes the transaction parameters in the mempool.
- **Attacker Strategy**: The attacker attempts to construct a modified transaction substituting `destination = attacker_address`.
- **Outcome & Analysis**: **BLOCKED (Cryptographically Secured)**.
  - The Ed25519 authorization signature is evaluated over:
    $$\text{SHA256}(\text{destination.to\_xdr()} \mathbin{\Vert} \text{nonce}_{\text{be\_u64}} \mathbin{\Vert} \text{controller\_id.to\_xdr()})$$
  - If the attacker changes `destination` to `attacker_address`, `SweepController::execute_sweep` invokes `AuthContext::verify()`, which fails with `Error::SignatureVerificationFailed` (or traps on host signature check).
  - If the attacker replays the *exact same* transaction without modifying `destination`, the transaction executes with the legitimate `destination`. The attacker spends gas and achieves no theft.
  - If `SweepController` is initialized in **Locked Mode** (`authorized_destination = Some(locked_addr)`), `validate_destination()` enforces `destination == locked_addr`, providing a second defense layer.

---

### Threat Scenario 2: Claim Preemption in `claim` (Gas-Free Path)
- **Mechanism**: The recipient or relayer submits `SweepController::claim(recipient, ephemeral_account)`.
- **Mempool Exposure**: Attacker observes `claim(legitimate_recipient, ephemeral_account)` in the mempool.
- **Attacker Strategy**: The attacker immediately signs and submits `claim(attacker_address, ephemeral_account)` with higher transaction fee to land earlier in the same ledger block.
- **Outcome & Analysis**: **VULNERABLE in Flexible Mode / PROTECTED in Locked Mode**.

#### A. In Flexible Mode (`authorized_destination = None`)
- `SweepController::claim` requires `recipient.require_auth()`. An attacker can generate a valid Soroban auth entry for their own address (`attacker_address`).
- `claim` does **not** verify an Ed25519 signature from `authorized_signer`.
- If `claim(attacker_address, ephemeral_account)` executes first:
  1. `validate_destination()` passes (no destination lock set).
  2. `EphemeralAccount::sweep_claim(attacker_address)` transitions account status to `Swept`.
  3. All recorded token payments and base reserve are transferred to `attacker_address`.
  4. The original transaction `claim(legitimate_recipient, ephemeral_account)` then reverts with `Error::AlreadySwept`.
- **Result**: Frontrunner successfully steals all funds in flexible mode.

#### B. In Locked Mode (`authorized_destination = Some(locked_address)`)
- `validate_destination()` explicitly asserts `recipient == authorized_destination`.
- If the attacker submits `claim(attacker_address, ephemeral_account)`, `validate_destination()` returns `Err(Error::UnauthorizedDestination)`.
- **Result**: Frontrunning attempt fails; funds remain secure.

---

## Summary Matrix of Frontrunning Vectors

| Pathway | Operating Mode | Frontrunner Action | Result / Impact | Security Status |
| :--- | :--- | :--- | :--- | :--- |
| `execute_sweep()` | Flexible or Locked | Change `destination` to attacker | Signature verification fails | **SECURE** |
| `execute_sweep()` | Flexible or Locked | Replay original tx | Original destination receives funds | **SECURE** |
| `claim()` | **Locked Mode** | Submit `claim(attacker, ephemeral)` | Fails with `UnauthorizedDestination` | **SECURE** |
| `claim()` | **Flexible Mode** | Submit `claim(attacker, ephemeral)` | Account swept to attacker | **VULNERABLE** |

---

## Recommended Mitigations

### 1. Mandate Locked Mode for `claim()` Invocations
When utilizing the gas-free `claim()` interface, always initialize `SweepController` with `authorized_destination = Some(recipient_address)`. This guarantees that no third party can redirect funds via `claim()`.

### 2. Restrict Flexible Mode to `execute_sweep()`
If a application architecture requires dynamic/flexible destinations (where destination is not pre-set at initialization), **never use `claim()`**. Force all sweeps through `execute_sweep()`, which binds `destination` cryptographically inside the Ed25519 signature payload.

### 3. Private RPC / Direct Validator Submission
Relayers submitting `claim()` or `execute_sweep()` transactions should use private Horizon/RPC endpoints or direct validator submission channels (e.g. specialized transaction submission nodes) to prevent mempool snooping by sandwich/frontrunning bots.

### 4. Recipient Authentication Payload Binding (Protocol Update)
Future contract iterations of `SweepController::claim()` can accept an off-chain authorization voucher signed by `authorized_signer` binding `(recipient, ephemeral_account)`, ensuring even flexible-mode `claim()` calls cannot be hijacked by an arbitrary caller.
