# Threat Model: SweepController `claim()` Native-Auth Flow

**Path:** `bridgelet-audit/threat-models/sweep-controller-claim-flow.md`  
**Component:** `SweepController`  
**Target Operations:** `claim()`

---

## Executive Summary

The `SweepController::claim(recipient, ephemeral_account)` function relies on Soroban's native authorization framework (`recipient.require_auth()`) instead of custom Ed25519 signature verification (which is used in `execute_sweep`). 

This threat model analyzes how native authorization behaves when a relayer or the recipient submits the transaction, examining potential attack vectors such as unauthorized claims, fee-bumping exploits, and authorization payload hijacking.

---

## Detailed Threat Scenario & Vulnerability Analysis

### 1. Authorization Payload Hijacking (Relayer Submission)
- **Scenario**: A user signs an authorization payload allowing a relayer to submit the `claim()` transaction on their behalf to cover gas fees.
- **Threat**: A malicious relayer intercepts the auth payload and modifies the `ephemeral_account` argument to point to a different ephemeral account.
- **Analysis**: **BLOCKED**. Soroban's native authorization strictly binds the signature to the exact contract ID, function name, and arguments. Modifying the `ephemeral_account` invalidates the `require_auth()` check. The signature is non-malleable.

### 2. Fee-Bumping and Griefing
- **Scenario**: An attacker observes a valid `claim()` transaction in the mempool submitted by a relayer.
- **Threat**: The attacker submits a duplicate transaction with a higher fee to make the original transaction fail.
- **Analysis**: **LOW IMPACT**. If the attacker successfully fronts the transaction, the `claim()` is executed on behalf of the valid recipient. The funds arrive at the correct destination. The attacker simply pays the gas fees for the user. The original relayer's transaction will fail, causing them to lose a minor base fee, but no funds are stolen.

### 3. Phishing for Native Auth Signatures
- **Scenario**: A malicious dApp prompts the user to sign a Soroban authorization payload for `SweepController::claim()`.
- **Threat**: The user blindly signs the payload, authorizing the smart contract to act on their behalf.
- **Analysis**: **MODERATE**. While `claim()` requires the recipient's authorization, it fundamentally transfers funds *to* the recipient. An attacker tricking a user into signing a `claim()` payload merely allows the attacker to push funds into the user's wallet. It does not allow the attacker to withdraw funds *from* the user's wallet.

### 4. Cross-Contract Reentrancy during Claim
- **Scenario**: The `claim()` function invokes an external contract (e.g., token transfers).
- **Threat**: A malicious token contract reenters `SweepController::claim()`.
- **Analysis**: **BLOCKED**. `SweepController` does not hold custody of funds; it delegates to the `EphemeralAccount`. Furthermore, the ephemeral account transitions its status to `Swept` prior to executing the token transfers, adhering to the Checks-Effects-Interactions pattern. Reentering `claim()` will immediately trap with an `AlreadySwept` error.

---

## Recommended Mitigations

### 1. Maintain Strict Argument Binding
Ensure that `recipient.require_auth()` is the primary enforcement mechanism in `claim()`, and avoid any manual parsing of authorization arguments that could bypass the Soroban host's native checks.

### 2. Transparent Wallet Previews
Wallets interacting with Bridgelet should implement transparent simulation of native auth payloads, clearly showing the user that they are authorizing a `claim` action that will deposit funds into their account, eliminating phishing confusion.

### 3. Status Flags for Reentrancy Protection
Ensure that `EphemeralAccount` explicitly marks its internal state as `Swept` *before* issuing any `token.transfer()` calls during the sweep process, guaranteeing reentrancy safety.
