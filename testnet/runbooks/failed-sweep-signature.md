# Runbook: Diagnosing Failed Sweep Signatures

## Overview

`SweepController::execute_sweep` performs real Ed25519 verification over `hash(destination + nonce + contract_id)` with nonce-based replay protection. This runbook covers the common causes of a rejected signature on testnet.

## Symptom

`execute_sweep` transaction fails with `SignatureVerificationFailed` error (or transaction aborts due to failed `ed25519_verify` host function trap).

## Prerequisites

- Testnet identity with XLM (`stellar keys generate --global testnet-investigator`)
- Deployed SweepController and EphemeralAccount contracts
- Access to off-chain signer (tools/sweep-signer or equivalent)
- Contract IDs from `testnet/registry/contract-status.md`

## Signature Verification Details

### Message Format

```rust
message = SHA256(
    destination.to_xdr()           // 32-byte address
    || nonce as u64 big-endian    // 8 bytes
    || controller_contract_id.to_xdr()  // 32-byte address
)
```

### Verification Flow

1. Off-chain signer builds message using **current on-chain nonce**
2. Signer produces 64-byte Ed25519 signature
3. Relayer submits `execute_sweep(ephemeral_account, destination, signature)`
4. Contract verifies: `ed25519_verify(authorized_signer, message, signature)`
5. **On failure**: Host function traps → transaction aborts
6. **On success**: Nonce increments → sweep proceeds

---

## Common Causes & Diagnosis

### Cause 1: Stale Nonce (Most Common)

**Symptom**: Signature verified against old nonce; contract expects newer nonce.

**Diagnosis**:
```bash
# Check current on-chain nonce
stellar contract invoke \
  --id <SWEEP_CONTROLLER_ID> \
  --network testnet \
  --source testnet-investigator \
  -- \
  get_nonce

# Compare with nonce used to build signature
# If signature_nonce < on_chain_nonce → STALE NONCE
```

**Resolution**: Re-fetch nonce and re-sign.

### Cause 2: Wrong Contract ID in Message

**Symptom**: Signature built with mainnet/localnet contract ID instead of testnet ID.

**Diagnosis**:
```bash
# Verify SweepController contract ID matches
echo "Expected: <TESTNET_SWEEP_CONTROLLER_ID>"
echo "Used in signature: <CHECK_SIGNER_LOGS>"

# Check contract ID in signer tool config
cat tools/sweep-signer/config.toml  # or equivalent
```

**Resolution**: Ensure signer uses correct testnet contract ID.

### Cause 3: Destination Address Mismatch

**Symptom**: Signature built for one destination, but `execute_sweep` called with different destination.

**Diagnosis**:
```bash
# Check destination in execute_sweep call
# Compare with destination used in signature
# Must be EXACT match (case-sensitive, same address format)
```

**Resolution**: Ensure destination in signature == destination in call.

### Cause 4: Wrong Authorized Signer

**Symptom**: Signature signed by different Ed25519 key than `authorized_signer` on controller.

**Diagnosis**:
```bash
# No direct getter for authorized_signer in current interface
# Infer from:
# 1. Initialize transaction - authorized_signer passed as param
# 2. Successful sweeps - signatures that worked
# 3. Check deploy script / config for expected pubkey
```

**Resolution**: Use correct Ed25519 private key matching controller's `authorized_signer`.

### Cause 5: Locked Destination Mode Violation

**Symptom**: Controller initialized with `authorized_destination`, but sweep destination differs.

**Diagnosis**:
```bash
# Try execute_sweep with different destination
# If controller is locked, fails with UnauthorizedDestination
# (This is a DIFFERENT error than SignatureVerificationFailed)
```

**Resolution**: Use the locked destination, or call `update_authorized_destination` first (if nonce == 0).

### Cause 6: Nonce Increment Race Condition

**Symptom**: Multiple concurrent sweeps; nonce increments between sign and submit.

**Diagnosis**:
```bash
# Check if multiple relayers signing simultaneously
# Only one will succeed; others get stale nonce
# Nonce increments on SUCCESSFUL verification (before sweep)
```

**Resolution**: Serialize sweep signing; use queue or locking.

---

## Step-by-Step Diagnosis Procedure

### Step 1: Get Current Nonce
```bash
CURRENT_NONCE=$(stellar contract invoke \
  --id <SWEEP_CONTROLLER_ID> \
  --network testnet \
  --source testnet-investigator \
  -- \
  get_nonce)
echo "Current nonce: $CURRENT_NONCE"
```

### Step 2: Verify Signature Construction
```bash
# Manually reconstruct message
# destination (32 bytes XDR) + nonce (8 bytes BE) + contract_id (32 bytes XDR)
# SHA256 of above
# Verify signature matches this message
```

### Step 3: Check Controller Config
```bash
# Verify controller initialized correctly
# Check deploy logs for authorized_signer and authorized_destination
```

### Step 4: Test with Known-Good Signature
```bash
# Use tools/sweep-signer to generate fresh signature
# with current nonce and testnet contract ID
# Submit and verify success
```

### Step 5: Check Controller Events
```bash
# Look for SweepCompleted events to see last successful sweep
stellar contract events \
  --contract-id <SWEEP_CONTROLLER_ID> \
  --start-ledger <RECENT> \
  --filter '{"topics": [["sweep"]]}' \
  --network testnet
```

---

## Quick Reference: Error Codes

| Error | Meaning | Likely Cause |
|---|---|---|
| `SignatureVerificationFailed` | Ed25519 verify failed | Stale nonce, wrong contract ID, wrong destination, wrong key |
| `UnauthorizedDestination` | Locked mode violation | Destination ≠ authorized_destination |
| `AuthorizedSignerNotSet` | Controller not initialized | Deploy/init issue |
| `InvalidNonce` | Nonce out of sequence | Race condition or manual nonce manipulation |

---

## Prevention Checklist

For SDK/relayer implementers:

- [ ] Always fetch fresh nonce via `get_nonce` before signing
- [ ] Cache nonce for minimal time (sign → submit immediately)
- [ ] Use testnet contract ID from config (not mainnet/localnet)
- [ ] Serialize sweep operations per controller (mutex/queue)
- [ ] Log nonce used for each signature for debugging
- [ ] Handle `SignatureVerificationFailed` by re-fetching nonce and retrying
- [ ] Monitor `get_nonce` in health checks

---

## Related Documentation

- `testnet/config/nonce-tracking.md` - Nonce inspection procedures
- `testnet/registry/event-topics.md` - `sweep` event topic
- `testnet/registry/verify-contract-live.md` - Health checks
- `docs/SIGNATURE_FORMAT.md` - Detailed signature format spec
- `tools/sweep-signer/` - Off-chain signing tool (referenced, not edited here)

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-24 | Initial failed sweep signature runbook |

---

## Fast triage: three-question decision tree

When `execute_sweep` fails with `SignatureVerificationFailed`, the signature is checked against exactly one message: `SHA256(destination_xdr || nonce_be_u64 || contract_id_xdr)`. A byte-level mismatch in any of the three components produces the same generic failure, so answer these three questions in order. Every command below is read-only and uses the testnet investigator identity; no secret keys are needed or should be logged.

| # | Question | Symptom that points here | Most likely cause | Exact command to check | Fix |
|---|---|---|---|---|---|
| 1 | Is the nonce current? | Signature worked before, or a sweep happened just prior; failure is intermittent | Stale nonce — signature was built against an older on-chain nonce | `stellar contract invoke --id <SWEEP_CONTROLLER_ID> --network testnet --source testnet-investigator -- get_nonce` (compare result with the nonce logged by the signer; signature nonce < on-chain nonce means stale) | Fetch a fresh nonce with `get_nonce()`, re-sign, and submit immediately. Serialize concurrent sweeps per controller. |
| 2 | Is the contract ID correct? | Signature rejects on testnet but verifies locally; multiple deployments/configs exist | Wrong contract ID in the hashed message (mainnet/localnet ID, or an older testnet deployment) | `python -c "import json; print(json.load(open('deployments/testnet.json'))['contracts']['sweepController'])"` and compare with the ID in `tools/sweep-signer/config.toml` or signer logs | Point the signer at the testnet `sweepController` ID from `deployments/testnet.json` and re-sign. The signature is bound to one deployment by `contract_id_xdr`. |
| 3 | Is the destination identical? | Failure appears only for one payout; the destination in the request/events differs from the signed one | Destination address mismatch — signed destination XDR does not byte-match the address passed to `execute_sweep` | `stellar contract events --contract-id <SWEEP_CONTROLLER_ID> --start-ledger <RECENT> --filter '{"topics": [["sweep"]]}' --network testnet` and compare the `destination` in recent events with the signed destination; regenerate with `tools/sweep-signer` to confirm | Use the exact same destination (same strkey, same XDR bytes) in both the signature and the `execute_sweep` call. |

If all three match and the failure persists, fall through to Causes 4-6 above (wrong authorized signer key, locked-destination mode, or a nonce increment race). A locked-mode violation returns `UnauthorizedDestination`, not `SignatureVerificationFailed`, so it is not a cause of this specific error.

### What to capture in an incident report

- [ ] Error observed (`SignatureVerificationFailed` vs. host-function trap) and the full transaction hash
- [ ] SweepController contract ID used on-chain and the ID the signer actually hashed
- [ ] On-chain `get_nonce()` output at the time of the failed submission
- [ ] Nonce used to build the signature (from signer logs; never the private key)
- [ ] Destination address used in the signature and the destination passed to `execute_sweep`
- [ ] Signer tool/config version and which deployment config was active (testnet vs. other)
- [ ] Whether other sweeps were submitted for the same controller around the same time
- [ ] Relevant `SweepCompleted` events (or their absence) from the controller
- [ ] Whether regenerating a signature with `tools/sweep-signer` and a fresh nonce succeeds
