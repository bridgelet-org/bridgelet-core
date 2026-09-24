# SweepController Nonce Tracking for Testnet

## Overview

`SweepController::execute_sweep` verifies signatures over `hash(destination + nonce + contract_id)` with nonce-based replay protection. This document describes, for testnet integration testing, how to query the contract's current expected nonce before constructing a test signature, since a stale/guessed nonce is a common source of confusing signature-verification failures.

## Nonce Behavior

### Basics
- **Starts at 0** at `initialize()`
- **Increments by 1** after every successful `execute_sweep()` or `claim()`
- **Checked at verification time** — contract always uses current on-chain nonce
- **Not decremented** — monotonically increasing

### Why Nonce Matters
```
Message = SHA256(destination || nonce || contract_id)
```
If signer uses nonce=5 but contract expects nonce=6 → signature verification fails.

---

## Querying Current Nonce

### Via stellar-cli (Recommended)

```bash
# Get current nonce for a SweepController
stellar contract invoke \
  --id <SWEEP_CONTROLLER_ID> \
  --network testnet \
  --source testnet-investigator \
  -- \
  get_nonce

# Output: u64 (e.g., "0", "3", "42")
```

### Via RPC Directly

```bash
curl -X POST https://soroban-testnet.stellar.org \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "simulateTransaction",
    "params": {
      "transaction": "<BASE64_TX_ENVELOPE_FOR_GET_NONCE>"
    }
  }'
```

### In Automated Tests/Scripts

```bash
#!/bin/bash
# get-nonce.sh - Fetch current nonce for signing

SWEEP_ID="${1:-<SWEEP_CONTROLLER_ID>}"

NONCE=$(stellar contract invoke \
  --id "$SWEEP_ID" \
  --network testnet \
  --source testnet-investigator \
  -- \
  get_nonce 2>/dev/null)

if [ $? -ne 0 ] || [ -z "$NONCE" ]; then
  echo "Error: Could not fetch nonce" >&2
  exit 1
fi

echo "$NONCE"
```

---

## Constructing Signatures with Correct Nonce

### Using tools/sweep-signer (Referenced Contextually)

```bash
# tools/sweep-signer expects current nonce
# Typical workflow:

# 1. Fetch nonce
NONCE=$(./get-nonce.sh <SWEEP_CONTROLLER_ID>)

# 2. Build message
# message = SHA256(destination_xdr || nonce_be || contract_id_xdr)

# 3. Sign with Ed25519 private key matching authorized_signer
# 4. Submit execute_sweep with signature
```

### Manual Signature Construction (for testing)

```rust
// Rust pseudo-code for signature construction
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Sha256, Digest};

fn build_sweep_message(
    destination: &Address,
    nonce: u64,
    controller_id: &Address,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(destination.to_xdr_bytes());
    hasher.update(nonce.to_be_bytes());  // 8 bytes big-endian
    hasher.update(controller_id.to_xdr_bytes());
    hasher.finalize().into()
}

fn sign_sweep(
    signing_key: &SigningKey,
    destination: &Address,
    nonce: u64,
    controller_id: &Address,
) -> [u8; 64] {
    let message = build_sweep_message(destination, nonce, controller_id);
    signing_key.sign(&message).to_bytes()
}
```

---

## Common Pitfalls & Solutions

### Pitfall 1: Cached/Stale Nonce

**Symptom**: SignatureVerificationFailed after successful sweep

**Cause**: Signer cached nonce=3, but contract now at nonce=4

**Fix**: Always fetch fresh nonce before signing
```bash
# WRONG: Use cached nonce
NONCE=3  # from earlier

# RIGHT: Fetch fresh
NONCE=$(stellar contract invoke --id $SWEEP_ID --network testnet --source testnet-investigator -- get_nonce)
```

### Pitfall 2: Race Condition (Concurrent Signers)

**Symptom**: Multiple relayers signing simultaneously; only first succeeds

**Cause**: Both read nonce=5, both sign with nonce=5, first to submit wins

**Fix**: Serialize signing with mutex/queue
```rust
// Pseudo-code
let nonce = fetch_nonce().await;
let signature = sign(message_with_nonce).await;
let result = submit_execute_sweep(signature).await;
if result.is_err() {
    // Retry with fresh nonce
}
```

### Pitfall 3: Wrong Contract ID in Message

**Symptom**: SignatureVerificationFailed on first try with correct nonce

**Cause**: Signer used mainnet/localnet contract ID instead of testnet ID

**Fix**: Verify contract ID in signer config
```bash
# Check signer config
cat tools/sweep-signer/config.toml
# Ensure contract_id = testnet SweepController ID

# Verify via RPC
stellar contract invoke --id $SWEEP_ID --network testnet --source testnet-investigator -- get_nonce
```

### Pitfall 4: Destination Mismatch

**Symptom**: SignatureVerificationFailed (not UnauthorizedDestination)

**Cause**: Signature built for destination A, but execute_sweep called with destination B

**Fix**: Ensure exact match
```bash
# Build signature
DESTINATION=<EXACT_ADDRESS>
# Call execute_sweep with SAME destination
stellar contract invoke --id $SWEEP_ID --network testnet --source testnet-relayer -- execute_sweep --ephemeral_account $EPH_ID --destination $DESTINATION --auth_signature $SIG
```

### Pitfall 5: Locked Mode Destination Check

**Symptom**: UnauthorizedDestination (different from SignatureVerificationFailed)

**Cause**: Controller initialized with authorized_destination, but sweep destination differs

**Fix**: 
```bash
# Check if controller is locked (no direct getter)
# If locked, MUST use that destination
# Or call update_authorized_destination (if nonce == 0)
```

---

## Testing Checklist

Before running integration tests:

- [ ] Fetch fresh nonce via `get_nonce`
- [ ] Verify testnet contract ID in signer config
- [ ] Ensure destination in signature == destination in call
- [ ] If locked mode: destination matches authorized_destination
- [ ] Serialize concurrent sweep operations
- [ ] Log nonce used for each signature (for debugging)

---

## Monitoring Nonce

### Health Check

```bash
# Include in health checks
stellar contract invoke \
  --id <SWEEP_CONTROLLER_ID> \
  --network testnet \
  --source testnet-healthcheck \
  -- \
  get_nonce
```

### Event Correlation

```bash
# Get sweep events to verify nonce progression
stellar contract events \
  --contract-id <SWEEP_CONTROLLER_ID> \
  --start-ledger <RECENT> \
  --filter '{"topics": [["sweep"]]}' \
  --network testnet

# Each SweepCompleted event corresponds to nonce increment
# Count events since deploy = expected nonce
```

---

## Related Documentation

- `testnet/runbooks/failed-sweep-signature.md` - Signature failure diagnosis
- `testnet/registry/event-topics.md` - `sweep` event topic
- `testnet/registry/verify-contract-live.md` - Health check includes nonce
- `docs/SIGNATURE_FORMAT.md` - Detailed message format specification
- `tools/sweep-signer/` - Off-chain signing tool (referenced, not edited here)

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-24 | Initial nonce tracking guide |