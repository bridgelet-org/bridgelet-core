# Testing SweepController::claim() Gas-Free Relayer Path

## Overview

The `claim()` gas-free path (recipient signs a Soroban auth entry, a relayer submits and pays fees) is architecturally distinct from the standard signed-sweep path and needs its own testnet testing documentation, since it requires simulating a two-party (recipient + relayer) interaction rather than a single signer.

## Architecture Comparison

| Aspect | `execute_sweep` (Signed) | `claim` (Gas-Free) |
|---|---|---|
| **Signer** | Relayer (with Ed25519 key) | Recipient (Soroban auth) |
| **Fee Payer** | Relayer | Relayer |
| **Auth Mechanism** | Ed25519 signature over message | Soroban native `require_auth()` |
| **Signature Format** | `hash(destination + nonce + contract_id)` | Transaction auth entry for `claim(recipient, ephemeral_account)` |
| **Nonce Used** | Yes (incremented per sweep) | Yes (same nonce counter) |
| **Destination Locking** | Enforced | Enforced |

## Prerequisites

- **Two testnet identities**:
  - `testnet-relayer` — pays fees, submits transaction
  - `testnet-recipient` — signs auth entry, receives funds
- Both funded: `stellar keys fund <identity> --network testnet`
- Deployed contracts: SweepController + EphemeralAccount
- SweepController initialized (with or without locked destination)

---

## Setup: Create Identities

```bash
# Relayer (pays fees, submits tx)
stellar keys generate --global testnet-relayer
stellar keys fund testnet-relayer --network testnet

# Recipient (signs auth, receives funds)
stellar keys generate --global testnet-recipient
stellar keys fund testnet-recipient --network testnet

# Get addresses
RELAYER_ADDR=$(stellar keys address testnet-relayer)
RECIPIENT_ADDR=$(stellar keys address testnet-recipient)
echo "Relayer: $RELAYER_ADDR"
echo "Recipient: $RECIPIENT_ADDR"
```

---

## Test Scenario 1: Flexible Mode (No Locked Destination)

### Setup: Initialize SweepController in Flexible Mode

```bash
# Initialize WITHOUT authorized_destination (flexible mode)
stellar contract invoke \
  --id <SWEEP_CONTROLLER_WASM_HASH> \
  --network testnet \
  --source testnet-deployer \
  -- \
  initialize \
  --creator <CREATOR> \
  --authorized_signer <ED25519_PUBKEY> \
  --authorized_destination null
```

### Test: Claim to Arbitrary Destination

```bash
# 1. Create and initialize EphemeralAccount
# (Use AccountFactory or direct deploy + initialize)
EPHEMERAL_ID=<EPHEMERAL_ACCOUNT_ID>

# 2. Record payment
stellar contract invoke \
  --id $EPHEMERAL_ID \
  --network testnet \
  --source testnet-watcher \
  -- \
  record_payment \
  --amount 100000000 \
  --asset <USDC_ID>

# 3. Verify can_sweep
stellar contract invoke \
  --id <SWEEP_CONTROLLER_ID> \
  --network testnet \
  --source testnet-investigator \
  -- \
  can_sweep \
  --ephemeral_account $EPHEMERAL_ID
# Should return: true

# 4. Recipient calls claim() — ONLY recipient signs auth
# Relayer submits and pays fees
stellar contract invoke \
  --id <SWEEP_CONTROLLER_ID> \
  --network testnet \
  --source testnet-relayer \
  -- \
  claim \
  --recipient $RECIPIENT_ADDR \
  --ephemeral_account $EPHEMERAL_ID

# Expected: Success!
# - Relayer pays fees
# - Recipient's Soroban auth satisfies EphemeralAccount's authorized_controller check
# - Funds transferred to RECIPIENT_ADDR
# - Nonce incremented
```

### Verify Results

```bash
# Check account status
stellar contract invoke \
  --id $EPHEMERAL_ID \
  --network testnet \
  --source testnet-investigator \
  -- \
  get_status
# Should return: 2 (Swept)

# Check sweep completed event
stellar contract events \
  --contract-id <SWEEP_CONTROLLER_ID> \
  --start-ledger <RECENT> \
  --filter '{"topics": [["sweep"]]}' \
  --network testnet
# Should show SweepCompleted with destination = RECIPIENT_ADDR

# Check recipient token balance (via Horizon)
```

---

## Test Scenario 2: Locked Mode (Destination Pre-Set)

### Setup: Initialize SweepController in Locked Mode

```bash
# Initialize WITH authorized_destination (locked mode)
stellar contract invoke \
  --id <SWEEP_CONTROLLER_WASM_HASH> \
  --network testnet \
  --source testnet-deployer \
  -- \
  initialize \
  --creator <CREATOR> \
  --authorized_signer <ED25519_PUBKEY> \
  --authorized_destination $RECIPIENT_ADDR
```

### Test: Claim Must Match Locked Destination

```bash
# 1. Create new EphemeralAccount and record payment
EPHEMERAL_ID_2=<NEW_EPHEMERAL_ID>
# ... record payment ...

# 2. Recipient claims (MUST match locked destination)
stellar contract invoke \
  --id <SWEEP_CONTROLLER_ID> \
  --network testnet \
  --source testnet-relayer \
  -- \
  claim \
  --recipient $RECIPIENT_ADDR \
  --ephemeral_account $EPHEMERAL_ID_2
# Expected: Success (destination matches)

# 3. Try claim with DIFFERENT recipient (should fail)
OTHER_RECIPIENT=<OTHER_ADDRESS>
stellar contract invoke \
  --id <SWEEP_CONTROLLER_ID> \
  --network testnet \
  --source testnet-relayer \
  -- \
  claim \
  --recipient $OTHER_RECIPIENT \
  --ephemeral_account $EPHEMERAL_ID_2
# Expected: Error::UnauthorizedDestination
```

---

## Test Scenario 3: Nonce Behavior

### Verify Nonce Increments on Claim

```bash
# Get nonce before claim
NONCE_BEFORE=$(stellar contract invoke \
  --id <SWEEP_CONTROLLER_ID> \
  --network testnet \
  --source testnet-investigator \
  -- \
  get_nonce)

# Execute claim
stellar contract invoke \
  --id <SWEEP_CONTROLLER_ID> \
  --network testnet \
  --source testnet-relayer \
  -- \
  claim \
  --recipient $RECIPIENT_ADDR \
  --ephemeral_account $EPHEMERAL_ID

# Get nonce after claim
NONCE_AFTER=$(stellar contract invoke \
  --id <SWEEP_CONTROLLER_ID> \
  --network testnet \
  --source testnet-investigator \
  -- \
  get_nonce)

# Verify: NONCE_AFTER = NONCE_BEFORE + 1
echo "Before: $NONCE_BEFORE, After: $NONCE_AFTER"
```

### Verify Claim Fails with Stale Nonce (Simulated)

Note: Can't easily test stale nonce for claim() since nonce is checked internally, but the principle is same as execute_sweep.

---

## Test Scenario 4: Expired Account Cannot Claim

```bash
# 1. Create account with near expiry (1-2 minutes)
# 2. Record payment
# 3. Wait for expiry
# 4. Try claim — should fail with AccountExpired

stellar contract invoke \
  --id <SWEEP_CONTROLLER_ID> \
  --network testnet \
  --source testnet-relayer \
  -- \
  claim \
  --recipient $RECIPIENT_ADDR \
  --ephemeral_account $EXPIRED_EPHEMERAL_ID
# Expected: Error::AccountExpired (or similar)
```

---

## Test Scenario 5: No Payment Recorded

```bash
# 1. Create account, NO payment recorded
# 2. Try claim
stellar contract invoke \
  --id <SWEEP_CONTROLLER_ID> \
  --network testnet \
  --source testnet-relayer \
  -- \
  claim \
  --recipient $RECIPIENT_ADDR \
  --ephemeral_account $EMPTY_EPHEMERAL_ID
# Expected: Error::AccountNotReady
```

---

## Integration Test Patterns

### Rust SDK Test Pattern

```rust
#[tokio::test]
async fn test_claim_gas_free_path() {
    let relayer = testnet_relayer();
    let recipient = testnet_recipient();
    let ephemeral = create_funded_ephemeral_account().await;
    
    // Recipient only signs the claim auth entry
    // Relayer submits and pays fees
    let result = sweep_controller
        .claim(&recipient.address, &ephemeral.address)
        .submit_as(&relayer)  // Relayer pays fees
        .await;
    
    assert!(result.is_ok());
    
    // Verify recipient received funds
    let balance = token_client.balance(&recipient.address).await;
    assert_eq!(balance, EXPECTED_AMOUNT);
    
    // Verify nonce incremented
    let nonce = sweep_controller.get_nonce().await;
    assert_eq!(nonce, INITIAL_NONCE + 1);
}
```

### CLI Test Pattern

```bash
#!/bin/bash
# test-claim.sh - Automated claim test

set -e

SWEEP_ID=<SWEEP_CONTROLLER_ID>
EPHEMERAL_ID=<EPHEMERAL_ACCOUNT_ID>
RECIPIENT=<RECIPIENT_ADDR>

echo "Testing claim() gas-free path..."

# 1. Verify can_sweep
stellar contract invoke --id $SWEEP_ID --network testnet --source testnet-investigator -- can_sweep --ephemeral_account $EPHEMERAL_ID

# 2. Execute claim (recipient signs, relayer pays)
stellar contract invoke --id $SWEEP_ID --network testnet --source testnet-relayer -- claim --recipient $RECIPIENT --ephemeral_account $EPHEMERAL_ID

# 3. Verify swept
stellar contract invoke --id $EPHEMERAL_ID --network testnet --source testnet-investigator -- get_status
# Expect: 2 (Swept)

echo "Claim test passed!"
```

---

## Debugging Failed Claims

| Error | Cause | Fix |
|---|---|---|
| `UnauthorizedDestination` | Locked mode + wrong recipient | Use locked destination |
| `AccountNotReady` | No payment recorded | Record payment first |
| `AccountExpired` | Past expiry_ledger | Use non-expired account |
| Auth failure | Recipient didn't sign | Ensure `--source testnet-recipient` NOT used; auth is in tx, not source |

**Key**: The `--source` flag identifies the FEE PAYER (relayer). The recipient authorizes via Soroban auth entry in the transaction, NOT via `--source`.

---

## Related Documentation

- `testnet/config/nonce-tracking.md` - Nonce inspection for claim()
- `testnet/runbooks/failed-sweep-signature.md` - Signature issues (execute_sweep)
- `testnet/runbooks/stuck-ephemeral-account.md` - Account won't claim
- `docs/api-reference.md` - `claim()` API reference
- `docs/SIGNATURE_FORMAT.md` - Signature format (for execute_sweep comparison)

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-24 | Initial gas-free claim testing guide |