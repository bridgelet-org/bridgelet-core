# Runbook: Diagnosing Stuck Ephemeral Accounts

## Overview

An `EphemeralAccount` on testnet appears "stuck" — payment received but `sweep()`/`sweep_claim()` fails, or `is_expired()` never returns true when expected. This runbook covers the most likely causes given known stub behavior.

## Symptom Checklist

| Symptom | Likely Section |
|---|---|
| `sweep()` returns `Unauthorized` | Section A: Controller Authorization |
| `sweep()` returns `AccountExpired` but ledger < expiry | Section B: Expiry Ledger Math |
| `sweep()` returns `AlreadySwept` but funds not received | Section C: Sweep Status vs Transfers |
| `is_expired()` returns `false` past expected expiry | Section B: Expiry Ledger Math |
| `sweep_claim()` fails | Section D: Claim Path |
| `expire()` returns `NotExpired` | Section B: Expiry Ledger Math |

---

## Prerequisites

- Testnet identity with XLM (`stellar keys generate --global testnet-investigator`)
- Contract IDs from `testnet/registry/contract-status.md`
- Access to Soroban RPC: `https://soroban-testnet.stellar.org`

---

## Section A: Controller Authorization Issues

### A1: Caller is Not Authorized Controller

**Check**: Who is calling `sweep()`?
```bash
# Only the authorized_controller (typically SweepController contract ID) can call sweep()
# Direct calls from any other address fail with Unauthorized

# Verify authorized_controller for this account
stellar contract invoke \
  --id <EPHEMERAL_ACCOUNT_ID> \
  --network testnet \
  --source testnet-investigator \
  -- \
  get_info

# Check: get_info() doesn't directly expose authorized_controller
# But sweep() requires authorized_controller.require_auth()
# So caller MUST be the SweepController contract (or address set at init)
```

**Resolution**: Call `sweep()` via `SweepController::execute_sweep()` or `::claim()`, not directly.

### A2: SweepController Not Authorized

**Check**: Is SweepController the authorized_controller?
```bash
# At initialize(), authorized_controller was set
# Should be the SweepController contract ID
# If different address was used, sweep() will fail
```

**Resolution**: Re-deploy with correct `authorized_controller` = SweepController ID.

### A3: SweepController Not Initialized

```bash
# Verify SweepController is initialized
stellar contract invoke \
  --id <SWEEP_CONTROLLER_ID> \
  --network testnet \
  --source testnet-investigator \
  -- \
  get_nonce

# If error: Contract not initialized
# → Initialize SweepController first
```

---

## Section B: Expiry Ledger Math Issues

### B1: Ledger Sequence vs Wall Clock

**Key Concept**: `expiry_ledger` is a **ledger sequence number**, NOT a timestamp.

- Stellar ledgers close ~every 5 seconds
- 1 day ≈ 17,280 ledgers
- 1 year ≈ 6,307,200 ledgers

**Common Mistake**: Using Unix timestamp as `expiry_ledger`.

```bash
# Check current ledger
CURRENT_LEDGER=$(stellar network current-ledger --rpc-url https://soroban-testnet.stellar.org)
echo "Current ledger: $CURRENT_LEDGER"

# Check account's expiry_ledger
stellar contract invoke \
  --id <EPHEMERAL_ACCOUNT_ID> \
  --network testnet \
  --source testnet-investigator \
  -- \
  get_info
# Look at expiry_ledger field

# If expiry_ledger < CURRENT_LEDGER → Account IS expired
# If expiry_ledger > CURRENT_LEDGER → Account NOT expired
```

### B2: Fast-Iteration Testing (Near-Future Expiry)

For quick test iteration, set expiry to current ledger + small buffer:

```bash
# Example: Expire in ~5 minutes (600 ledgers)
EXPIRY=$((CURRENT_LEDGER + 600))

# Initialize with this expiry
stellar contract invoke \
  --id <EPHEMERAL_ACCOUNT_ID> \
  --network testnet \
  --source testnet-deployer \
  -- \
  initialize \
  --creator <CREATOR> \
  --expiry_ledger $EXPIRY \
  --recovery_address <RECOVERY> \
  --authorized_controller <SWEEP_CONTROLLER_ID> \
  --admin <ADMIN>
```

### B3: is_expired() Returns False Unexpectedly

```bash
# Check current ledger
stellar network current-ledger --rpc-url https://soroban-testnet.stellar.org

# Check is_expired()
stellar contract invoke \
  --id <EPHEMERAL_ACCOUNT_ID> \
  --network testnet \
  --source testnet-investigator \
  -- \
  is_expired

# If false but expected true:
# 1. Verify expiry_ledger in get_info()
# 2. Compare with current ledger
# 3. Remember: ledger sequence increments every ~5s
```

---

## Section C: Sweep Status vs Transfers

### C1: Status = Swept But Transfers Failed

**Important**: `EphemeralAccount::sweep()` sets status to `Swept` **before** token transfers (via SweepController). If transfers fail:
- Account status = `Swept` (cannot sweep again)
- Funds may still be in account
- `execute_sweep` returns `TransferFailed`

**Check**:
```bash
# 1. Check account status
stellar contract invoke --id <EPHEMERAL_ACCOUNT_ID> --network testnet --source testnet-investigator -- get_status
# Returns 2 (Swept)

# 2. Check SweepController events for TransferFailed
stellar contract events --contract-id <SWEEP_CONTROLLER_ID> --start-ledger <RECENT> --filter '{"topics": [["sweep"]]}' --network testnet

# 3. Check token balances on EphemeralAccount address
# (Use Horizon to check SEP-41 token balances)
```

**Resolution**: Retry `execute_sweep` with same signature (nonce already incremented, so need new signature with new nonce). OR call `reclaim_reserve` if only reserve remains.

### C2: Reserve Reclaim Stuck

```bash
# After sweep, reserve reclaims incrementally
# Check reserve status
stellar contract invoke --id <EPHEMERAL_ACCOUNT_ID> --network testnet --source testnet-investigator -- get_reserve_remaining
stellar contract invoke --id <EPHEMERAL_ACCOUNT_ID> --network testnet --source testnet-investigator -- get_reserve_available
stellar contract invoke --id <EPHEMERAL_ACCOUNT_ID> --network testnet --source testnet-investigator -- is_reserve_reclaimed

# If reserve_remaining > 0 but available = 0:
# → Need more ledgers for rent exemption to free up reserve
# Call reclaim_reserve periodically
```

---

## Section D: Claim Path Issues

### D1: Claim Fails with UnauthorizedDestination

```bash
# If SweepController initialized with authorized_destination (locked mode)
# claim() recipient MUST match that destination

# Check if controller is locked
# (No direct getter; infer from initialize params or update_authorized_destination failures)

# Resolution: Use correct recipient, or unlock via update_authorized_destination (if nonce == 0)
```

### D2: Claim Auth Not Provided

```bash
# claim() requires recipient.require_auth()
# Recipient MUST sign the transaction auth entry
# Relayer submits but ONLY recipient signs

# Verify: Transaction has recipient's Soroban auth entry
```

---

## Section E: Complete Diagnostic Checklist

Run these in order:

```bash
#!/bin/bash
# diagnose-stuck-account.sh

ACCOUNT_ID=$1
SWEEP_ID=$2

echo "=== Diagnosing Stuck Account: $ACCOUNT_ID ==="
echo ""

echo "1. Account Status:"
stellar contract invoke --id $ACCOUNT_ID --network testnet --source testnet-investigator -- get_status

echo ""
echo "2. Account Info:"
stellar contract invoke --id $ACCOUNT_ID --network testnet --source testnet-investigator -- get_info

echo ""
echo "3. Is Expired:"
stellar contract invoke --id $ACCOUNT_ID --network testnet --source testnet-investigator -- is_expired

echo ""
echo "4. Current Ledger:"
stellar network current-ledger --rpc-url https://soroban-testnet.stellar.org

echo ""
echo "5. Reserve Status:"
stellar contract invoke --id $ACCOUNT_ID --network testnet --source testnet-investigator -- get_reserve_remaining
stellar contract invoke --id $ACCOUNT_ID --network testnet --source testnet-investigator -- get_reserve_available
stellar contract invoke --id $ACCOUNT_ID --network testnet --source testnet-investigator -- is_reserve_reclaimed

echo ""
echo "6. SweepController Nonce:"
stellar contract invoke --id $SWEEP_ID --network testnet --source testnet-investigator -- get_nonce

echo ""
echo "7. SweepController Can Sweep:"
stellar contract invoke --id $SWEEP_ID --network testnet --source testnet-investigator -- can_sweep --ephemeral_account $ACCOUNT_ID
```

---

## Quick Reference: Error Meanings

| Error | Sweep/Expire | Meaning | Action |
|---|---|---|---|
| `Unauthorized` | sweep | Caller ≠ authorized_controller | Use SweepController |
| `AlreadySwept` | sweep/claim | Status = Swept | Check if transfers succeeded (C1) |
| `AccountExpired` | sweep | Current ledger ≥ expiry_ledger | Account expired; use expire/recover |
| `NotExpired` | expire | Current ledger < expiry_ledger | Wait or check expiry_ledger math |
| `InvalidStatus` | expire/recover | Status = Swept or Expired | Already terminal |
| `NoPaymentReceived` | sweep | No payments recorded | Check get_payment_count |
| `TransferFailed` | execute_sweep | SEP-41 transfer failed | Retry with new signature (nonce++) |

---

## Escalation

If all checks pass but account still stuck:

1. **Check transaction history** for the account on Horizon
2. **Verify WASM hash** matches expected (see `wasm-hash-reference.md`)
3. **Check for contract upgrade** (see `upgrade-history.md`)
4. **Contact core team** with:
   - Account ID
   - All diagnostic outputs above
   - Expected vs actual behavior

---

## Related Documentation

- `testnet/config/expiry-ledger-testing.md` - Near-future expiry computation
- `testnet/registry/verify-contract-live.md` - Health checks
- `testnet/registry/upgrade-history.md` - Contract upgrade log
- `testnet/registry/admin-addresses.md` - Controller/admin addresses
- `docs/SIGNATURE_FORMAT.md` - Signature format for sweep auth

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-24 | Initial stuck ephemeral account runbook |