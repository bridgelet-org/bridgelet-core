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

---

## Testnet pre-flight checklist for nonce-safe signing

The single most reliable way to avoid a confusing `SignatureVerificationFailed`
is to treat `get_nonce()` as a value you read at signing time, never as a value
you remember. This section is an additive pre-flight checklist for testnet
signing with `tools/sweep-signer` (the canonical signer) or any equivalent
off-chain signer.

### Why a stale nonce looks like a signature bug

The signed message is `SHA256(destination_xdr || nonce_be_u64 ||
contract_id_xdr)`. The nonce is one input to the hash, so signing with a nonce
that differs from the contract's current one produces a completely different
digest. The contract reports only a signature-verification failure; there is no
error that says "wrong nonce". If a signature fails to verify, a stale nonce is
one of the first things to rule out, alongside wrong destination XDR, wrong
contract ID and the wrong signing key.

### Checklist

- [ ] Query `get_nonce()` on the **deployed testnet** `sweepController` from
  `deployments/testnet.json` (or `deployment-artifacts/contract-ids.txt`),
  immediately before signing. Do not use a nonce cached from a previous test run
  or from another network.
- [ ] Use the nonce as a big-endian `u64` (8 bytes) and keep it identical in the
  signer input and in the contract call. The signer tool handles XDR encoding of
  addresses for you; avoid hand-rolling it.
- [ ] Confirm the destination address and the contract ID used to build the
  message are the exact ones passed to `execute_sweep`. A mismatch in either
  produces the same verification failure.
- [ ] Serialize concurrent sweeps against the same controller. Two workers that
  both read nonce N will both sign for N, and only the first submission will
  succeed; the second must re-read the nonce and retry.
- [ ] Record the nonce used for each signature in your logs so a failure can be
  correlated with the on-chain value.

### Race and retry handling

Nonce and signature must be produced from a single fresh read, and a failed
submission must trigger a fresh read rather than a resend of the same envelope:

1. Call `get_nonce()` to read N.
2. Build and sign the message for N.
3. Submit `execute_sweep`.
4. If submission fails or the invocation does not take effect, go back to step 1
   and read the nonce again. Do not assume N is still current.

For a `claim()` (gas-free) flow the nonce is checked internally rather than
supplied by the caller, but it shares the same counter. A claim that advances
the nonce will invalidate any execute_sweep signature you prepared earlier for
the old value.

### Can a failed sweep advance the nonce?

Treat this as "query, do not assume":

- If the Soroban transaction **traps/reverts**, all of its state changes are
  rolled back, so the nonce does **not** advance. A signature prepared for the
  nonce you read before the failed transaction is still usable.
- If the `execute_sweep` / `claim` invocation **succeeds on-chain**, the nonce
  advances by one. This is true even if a later, off-chain step (for example the
  SDK's custodial payout via Horizon) has a problem. A sweep that "looked
  failed" from the application side may therefore still have consumed its nonce.
- Because a testnet reset wipes contract state and a redeploy creates a new
  controller, the current nonce can also change out from under you entirely;
  re-check the contract ID and the live nonce after either event.

In all cases the safe rule is the same: call `get_nonce()` again before every
signature and never reuse a signature across attempts.

### Concrete command sequence

```bash
SWEEP_ID=<SWEEP_CONTROLLER_ID from deployments/testnet.json>
DESTINATION=<G..._RECIPIENT_ADDRESS>
EPHEMERAL_ID=<EPHEMERAL_ACCOUNT_ID>

# 1. Read the nonce immediately before signing
NONCE=$(stellar contract invoke \
  --id "$SWEEP_ID" --network testnet \
  --source testnet-relayer -- get_nonce)
echo "signing with nonce=$NONCE"

# 2. Build and sign with the canonical signer tool
#    tools/sweep-signer uses soroban-sdk XDR serialization and an S... secret
SIG=$(cargo run --quiet --manifest-path tools/sweep-signer/Cargo.toml -- \
  --contract-id "$SWEEP_ID" \
  --destination "$DESTINATION" \
  --nonce "$NONCE")
# Refer to tools/sweep-signer --help for exact flag names in your checkout.

# 3. Submit using the SAME nonce-derived signature
stellar contract invoke \
  --id "$SWEEP_ID" --network testnet \
  --source testnet-relayer -- \
  execute_sweep \
  --ephemeral_account "$EPHEMERAL_ID" \
  --destination "$DESTINATION" \
  --auth_signature "$SIG"

# 4. On any failure, return to step 1 and re-read get_nonce()
NONCE_AFTER=$(stellar contract invoke \
  --id "$SWEEP_ID" --network testnet \
  --source testnet-relayer -- get_nonce)
echo "nonce now=$NONCE_AFTER"
```

> The `tools/sweep-signer` flags in step 2 are illustrative; check the tool's
> `--help` in your checkout for the current argument names. The invariant that
> matters is that the nonce passed to the signer is the one returned by the
> immediately preceding `get_nonce()` call.
