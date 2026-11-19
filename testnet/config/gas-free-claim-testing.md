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

---

## Testnet checklist and verification

This section is a walkthrough for running the two-party (recipient + relayer) flow
end to end on public testnet. It is additive to the scenarios above: use it as a
run sheet the first time you exercise `claim()` against a freshly redeployed
controller. All network values here come from
[`network.toml`](network.toml); contract IDs come from `deployments/testnet.json`
or `deployment-artifacts/contract-ids.txt` (and change on every redeploy or
testnet reset).

### 1. Establish the two identities

Keep the two roles separate so a leaked key in one does not compromise the other.
The recipient is the account that will be authorized to receive the funds; the
relayer is the account that submits the transaction and pays its fees.

```bash
stellar keys generate testnet-recipient --network testnet --fund
stellar keys generate testnet-relayer   --network testnet --fund

RECIPIENT_ADDR=$(stellar keys address testnet-recipient)
RELAYER_ADDR=$(stellar keys address testnet-relayer)
echo "recipient=$RECIPIENT_ADDR relayer=$RELAYER_ADDR"
```

If either `stellar keys fund` call fails, the account does not exist on the
ledger. Fund it directly through Friendbot:

```bash
curl "https://friendbot.stellar.org/?addr=$RECIPIENT_ADDR"
curl "https://friendbot.stellar.org/?addr=$RELAYER_ADDR"
```

Confirm both accounts are live before continuing:

```bash
curl -s "https://horizon-testnet.stellar.org/accounts/$RECIPIENT_ADDR" >/dev/null && echo "recipient funded"
curl -s "https://horizon-testnet.stellar.org/accounts/$RELAYER_ADDR"   >/dev/null && echo "relayer funded"
```

### 2. Confirm the deployed controller and its mode

Copy the current `sweepController` ID from `deployments/testnet.json` and verify
it is live with a read-only call (`can_sweep`) before building anything around
it. Note whether the controller is in flexible mode (`authorized_destination =
null`) or locked mode, because that determines which recipient address a claim
will accept.

```bash
SWEEP_ID=<SWEEP_CONTROLLER_ID from deployments/testnet.json>
stellar contract invoke --id "$SWEEP_ID" --network testnet \
  --source testnet-relayer -- can_sweep --ephemeral_account "$EPHEMERAL_ID"
```

### 3. Prepare the ephemeral account

Create and initialize an ephemeral account whose `authorized_controller` is the
deployed `SweepController` contract, then record a payment so the sweep has
something to transfer. The examples in Test Scenario 1 above show the exact
invocations.

### 4. Sign the auth entry as the recipient, submit as the relayer

The authorization is a Soroban auth entry for
`claim(recipient, ephemeral_account)`. It must be signed by the **recipient's**
key and attached to the transaction that the **relayer** submits. The `--source`
flag picks the fee payer (relayer) and must not be set to the recipient: doing so
makes the recipient pay fees and hides the two-party nature of the test.

- With the Rust SDK, build the `claim` invocation and call `submit_as(&relayer)`
  so the recipient's auth is present but the relayer funds the fees (see the
  Rust pattern above).
- With the CLI, make sure the transaction envelope carries the recipient's
  authorization entry while `--source` remains the relayer. A plain
  `--source testnet-recipient` invocation is a single-party test, not this flow.

### 5. Verify the result

Check all four signals rather than assuming success from the submit response:

```bash
# Account status: 2 means Swept
stellar contract invoke --id "$EPHEMERAL_ID" --network testnet \
  --source testnet-relayer -- get_status

# Nonce advanced by exactly one
NONCE_BEFORE=...   # captured immediately before the claim (see nonce-tracking.md)
NONCE_AFTER=$(stellar contract invoke --id "$SWEEP_ID" --network testnet \
  --source testnet-relayer -- get_nonce)
echo "before=$NONCE_BEFORE after=$NONCE_AFTER"   # expect after = before + 1

# SweepCompleted event names the recipient as destination
stellar events --network testnet \
  --contract-id "$SWEEP_ID" --start-ledger "$RECENT_LEDGER" \
  --filter '{"topics": [["sweep"]]}'
```

Finally, confirm the recipient's token balance actually increased through
Horizon. A successful contract call and event do not, by themselves, prove the
custodial payout reached the recipient.

### 6. Confirm destination-locking behaviour

If the controller is locked, repeat the claim with a different recipient and
expect `UnauthorizedDestination`. If it is flexible, a different recipient
should be accepted (provided that account's own auth entry is signed). This
distinguishes a locked-destination rejection from a signature/auth problem.

### 7. Troubleshooting quick reference

| Symptom | What to check |
|---|---|
| Auth failure / `require_auth` rejection | Recipient auth entry missing, or signed by the wrong key; verify the transaction carries the recipient's auth and that `--source` is the relayer |
| `UnauthorizedDestination` | Controller is locked and the recipient does not match `authorized_destination` |
| `AccountNotReady` | No payment was recorded on the ephemeral account |
| `AccountExpired` | The account's `expiry_ledger` has passed; use a fresh account |
| Nonce did not advance | The transaction trapped and rolled back; re-query `get_nonce()` and retry (see `nonce-tracking.md`) |
| Nothing happens / 404s | The contract ID is stale after a testnet reset; re-check `deployments/testnet.json` and redeploy if needed |
| Fee paid by the wrong account | `--source` was set to the recipient; the relayer must be the fee payer |

> This flow is testnet-only. Do not reuse testnet recipient or relayer secrets
> anywhere else, and never place a secret key under `testnet/config/`.
