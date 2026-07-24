# Glossary Entry: Sweep Nonce

**Path:** `bridgelet-audit/glossary/sweep-nonce.md`  
**Component:** `SweepController`  
**Related Functions:** `execute_sweep()`, `get_nonce()`, `update_authorized_destination()`

---

## Overview

The **Sweep Nonce** is a monotonically increasing 64-bit unsigned integer (`u64`) maintained by the `SweepController` contract instance. It provides cryptographic replay protection for off-chain Ed25519 sweep authorizations.

---

## Technical Mechanism

1. **Initialization**: Set to `0` when `SweepController::initialize` is invoked.
2. **Signature Payload Inclusion**: Off-chain signers must construct the cryptographic message digest as:
   $$\text{message} = \text{SHA256}(\text{destination.to\_xdr()} \mathbin{\Vert} \text{nonce}_{\text{be\_u64}} \mathbin{\Vert} \text{controller\_id.to\_xdr()})$$
3. **Verification & State Mutation**:
   - During `execute_sweep()`, `SweepController` fetches its current stored nonce and verifies the signature against `message`.
   - Upon successful signature verification, `SweepController` increments `nonce` (`nonce = nonce + 1`) **before** triggering downstream token transfers.
4. **Replay Prevention**: Any attempt to replay the same signed message fails because the on-chain nonce has incremented, causing signature verification to fail (`Error::SignatureVerificationFailed`).

---

## Important Interactions & Known Behaviors

- **`claim()` Interaction**: The gas-free `claim()` function uses Soroban native authorization (`recipient.require_auth()`) instead of an Ed25519 signature payload, and **does not** increment the controller's `sweep_nonce`.
- **Destination Immutability Lock**: `update_authorized_destination()` checks `nonce > 0` to determine if a sweep has occurred and lock the authorized destination.
