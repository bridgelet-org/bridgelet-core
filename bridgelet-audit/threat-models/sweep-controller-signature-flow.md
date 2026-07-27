# Threat Model: SweepController `execute_sweep` Signature Flow

**Path:** `bridgelet-audit/threat-models/sweep-controller-signature-flow.md`  
**Component:** `SweepController`  
**Target Operations:** `execute_sweep()`

---

## Executive Summary

The `SweepController::execute_sweep()` function allows off-chain services (such as a backend application or relayer) to submit an Ed25519 signature to authorize a sweep. This enables a flexible model where users do not need to sign native Soroban auth payloads, but rather standard Ed25519 payloads over a predefined schema.

This threat model evaluates the cryptographic binding of the signature, potential replay attacks, malleability, and destination interception.

---

## Detailed Threat Scenario & Vulnerability Analysis

### 1. Signature Replay Attacks
- **Scenario**: A malicious actor extracts a valid Ed25519 signature from a historical `execute_sweep` transaction and resubmits it to trigger another sweep.
- **Threat**: Unauthorized draining of the ephemeral account if funds are re-deposited.
- **Analysis**: **BLOCKED**. The signature payload mandates the inclusion of a sequential `nonce`. The `SweepController` enforces nonce uniqueness and increments it upon every successful signature verification. A replayed signature will fail the `nonce == expected_nonce` check, rendering replays impossible.

### 2. Destination Tampering (Interception)
- **Scenario**: An attacker monitors the mempool, takes a valid signature and `execute_sweep` call, and swaps out the `destination` argument for their own address.
- **Threat**: The attacker attempts to steal the sweep payload.
- **Analysis**: **BLOCKED**. The Ed25519 signature is evaluated over the `destination`. Specifically:
  `Hash( destination || nonce || controller_id )`.
  If the `destination` argument differs from the one hashed inside the signature, `verify()` will trap with `SignatureVerificationFailed`.

### 3. Cross-Chain / Cross-Contract Replays
- **Scenario**: A signature generated for a testnet `SweepController` or a different Bridgelet instance is submitted to the mainnet `SweepController`.
- **Threat**: Unauthorized execution on a different network or contract instance.
- **Analysis**: **BLOCKED**. The signature payload explicitly includes the `controller_id` (the contract's address/ID) and is bound to the Soroban network passphrase internally (via `env.crypto().ed25519_verify`). Signatures cannot cross environments.

### 4. Ephemeral Account Tampering
- **Scenario**: The attacker changes the `ephemeral_account` argument while keeping the signature intact.
- **Threat**: Sweeping a different user's account using another user's signature.
- **Analysis**: **MODERATE / MITIGATED**. The `execute_sweep` signature specifically authorizes sweeping to a destination, but the `ephemeral_account` itself is passed as an argument. However, if the attacker points to a different `ephemeral_account`, the destination constraint still applies—meaning the attacker would just be sweeping someone else's funds into the *original signer's* destination wallet. To mitigate griefing, `SweepController` bindings can include the `ephemeral_account` directly in the Ed25519 hash payload.

---

## Recommended Mitigations

### 1. Include Ephemeral Account ID in Signature Payload
To completely eliminate cross-account griefing, the signature payload should be updated to:
`Hash( ephemeral_account || destination || nonce || controller_id )`
This rigidly binds the authorization to a specific source account.

### 2. Nonce Management 
Ensure the nonce is stored in persistent or instance storage (never temporary) so that network expiration (TTL) cannot reset the nonce counter and reopen old signatures to replay attacks.
