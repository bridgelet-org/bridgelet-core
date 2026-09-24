# Computing Near-Future Expiry Ledger for Testnet Testing

## Overview

Since `expiry_ledger` is **ledger-sequence-based** (not wall-clock), testing expiry-related behavior (`is_expired()`, recovery-address fallback) quickly on testnet requires knowing the current ledger and Stellar's ~5-second close time to pick a suitably near expiry for fast test iteration.

## Key Concepts

### Ledger Sequence vs Time
- **Ledger**: Monotonically increasing integer (1, 2, 3...)
- **Close Time**: ~5 seconds per ledger on testnet
- **No wall-clock dependency**: Contracts use `env.ledger().sequence()` not `env.ledger().timestamp()`

### Why This Matters for Testing
- Setting `expiry_ledger = current + 100` ≈ 8 minutes
- Setting `expiry_ledger = current + 600` ≈ 50 minutes
- Setting `expiry_ledger = current + 17280` ≈ 24 hours

---

## Computing Expiry Ledger

### Step 1: Get Current Ledger

```bash
# Via stellar-cli
CURRENT_LEDGER=$(stellar network current-ledger --rpc-url https://soroban-testnet.stellar.org)

# Via RPC directly
curl -X POST https://soroban-testnet.stellar.org \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getLatestLedger"}'
```

### Step 2: Calculate Target Expiry

```bash
# For fast iteration (5-10 minutes)
EXPIRY_5MIN=$((CURRENT_LEDGER + 60))      # ~5 minutes
EXPIRY_10MIN=$((CURRENT_LEDGER + 120))    # ~10 minutes
EXPIRY_30MIN=$((CURRENT_LEDGER + 360))    # ~30 minutes

# For overnight testing
EXPIRY_1HR=$((CURRENT_LEDGER + 720))      # ~1 hour
EXPIRY_4HR=$((CURRENT_LEDGER + 2880))     # ~4 hours

# For day+ testing
EXPIRY_1DAY=$((CURRENT_LEDGER + 17280))   # ~24 hours
EXPIRY_1WEEK=$((CURRENT_LEDGER + 120960)) # ~7 days
```

### Step 3: Use in Initialize

```bash
# Initialize ephemeral account with computed expiry
stellar contract invoke \
  --id <EPHEMERAL_ACCOUNT_ID> \
  --network testnet \
  --source testnet-deployer \
  -- \
  initialize \
  --creator <CREATOR> \
  --expiry_ledger $EXPIRY_10MIN \
  --recovery_address <RECOVERY> \
  --authorized_controller <SWEEP_CONTROLLER_ID> \
  --admin <ADMIN>
```

---

## Worked Example

### Scenario: Test Expiry Flow in 10 Minutes

```bash
# 1. Get current ledger (assume 12345678)
CURRENT_LEDGER=12345678

# 2. Compute 10-minute expiry (10 min / 5 sec = 120 ledgers)
EXPIRY=$((12345678 + 120))
# EXPIRY = 12345798

# 3. Initialize account
stellar contract invoke \
  --id <EPHEMERAL_ID> \
  --network testnet \
  --source testnet-deployer \
  -- \
  initialize \
  --creator GAAAA... \
  --expiry_ledger 12345798 \
  --recovery_address GBBBB... \
  --authorized_controller <SWEEP_CTRL_ID> \
  --admin GCCCC...

# 4. Record payment
stellar contract invoke \
  --id <EPHEMERAL_ID> \
  --network testnet \
  --source testnet-watcher \
  -- \
  record_payment \
  --amount 100000000 \
  --asset <USDC_ID>

# 5. Verify not expired immediately
stellar contract invoke \
  --id <EPHEMERAL_ID> \
  --network testnet \
  --source testnet-investigator \
  -- \
  is_expired
# Returns: false

# 6. Wait ~10 minutes (or simulate ledger advance in local sandbox)

# 7. Verify expired
stellar contract invoke \
  --id <EPHEMERAL_ID> \
  --network testnet \
  --source testnet-investigator \
  -- \
  is_expired
# Returns: true

# 8. Test expire() or recover()
stellar contract invoke \
  --id <EPHEMERAL_ID> \
  --network testnet \
  --source testnet-investigator \
  -- \
  expire
```

---

## Automated Script

Save as `compute-expiry.sh`:

```bash
#!/bin/bash
# compute-expiry.sh - Compute expiry_ledger for testnet testing

set -e

RPC_URL="${RPC_URL:-https://soroban-testnet.stellar.org}"
DURATION_MIN="${1:-10}"  # Default 10 minutes

# Get current ledger
CURRENT_LEDGER=$(curl -s -X POST "$RPC_URL" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getLatestLedger"}' | \
  jq -r '.result')

if [ -z "$CURRENT_LEDGER" ] || [ "$CURRENT_LEDGER" = "null" ]; then
  echo "Error: Could not fetch current ledger"
  exit 1
fi

# Calculate ledgers to add (5 sec per ledger)
LEDGERS_TO_ADD=$((DURATION_MIN * 60 / 5))
EXPIRY=$((CURRENT_LEDGER + LEDGERS_TO_ADD))

echo "Current ledger: $CURRENT_LEDGER"
echo "Target duration: ${DURATION_MIN} minutes"
echo "Ledgers to add: $LEDGERS_TO_ADD"
echo "Expiry ledger: $EXPIRY"
echo ""
echo "Use in initialize:"
echo "  --expiry_ledger $EXPIRY"
```

Usage:
```bash
chmod +x compute-expiry.sh
./compute-expiry.sh 10      # 10 minutes
./compute-expiry.sh 60      # 1 hour
./compute-expiry.sh 1440    # 24 hours
```

---

## Testing Expiry Edge Cases

### Case 1: Expiry in Past (Should Fail Init)

```bash
# Try initializing with expiry < current
stellar contract invoke \
  --id <EPHEMERAL_ID> \
  --network testnet \
  --source testnet-deployer \
  -- \
  initialize \
  --creator <CREATOR> \
  --expiry_ledger $((CURRENT_LEDGER - 1)) \
  --recovery_address <RECOVERY> \
  --authorized_controller <SWEEP_CTRL_ID> \
  --admin <ADMIN>
# Expected: Error::InvalidExpiry
```

### Case 2: Expire Immediately After Init

```bash
# Expiry = current + 1 ledger (~5 seconds)
EXPIRY=$((CURRENT_LEDGER + 1))
# Initialize, wait 10 seconds, test is_expired() → true
```

### Case 3: Recovery Address Fallback

```bash
# 1. Initialize with expiry in 2 minutes
# 2. Record payment
# 3. Wait for expiry
# 4. Call expire() (anyone can call)
# 5. Verify funds go to recovery_address
stellar contract invoke \
  --id <EPHEMERAL_ID> \
  --network testnet \
  --source testnet-investigator \
  -- \
  expire
# Should emit AccountExpired with recovery_address
```

### Case 4: Recover vs Expire

```bash
# After expiry, both expire() and recover() work
# expire(): anyone can call
# recover(): only creator OR recovery_address can call (requires auth)

# Test recover by creator
stellar contract invoke \
  --id <EPHEMERAL_ID> \
  --network testnet \
  --source <CREATOR_SECRET> \
  -- \
  recover \
  --caller <CREATOR_ADDRESS>

# Test recover by recovery_address
stellar contract invoke \
  --id <EPHEMERAL_ID> \
  --network testnet \
  --source <RECOVERY_SECRET> \
  -- \
  recover \
  --caller <RECOVERY_ADDRESS>
```

---

## Common Pitfalls

| Pitfall | Symptom | Fix |
|---|---|---|
| Using Unix timestamp as expiry_ledger | `InvalidExpiry` or immediate expiry | Use ledger sequence only |
| Forgetting ledger advances during test | Test times out waiting | Use shorter durations (5-10 min) |
| Confusing `timestamp()` with `sequence()` | Contract logic errors | `env.ledger().sequence()` for expiry |
| Not funding recovery_address | `expire()` succeeds but funds stuck | Fund recovery address with XLM for rent |

---

## Local Sandbox Testing (Fastest Iteration)

For instant ledger advancement, use local Soroban sandbox:

```bash
# Start local sandbox (if available)
soroban sandbox start

# Deploy contracts locally
# Initialize with expiry = current + 1
# Advance ledger manually or wait 5s
# Test expiry instantly
```

---

## Related Documentation

- `testnet/runbooks/stuck-ephemeral-account.md` - Diagnose stuck accounts
- `testnet/registry/verify-contract-live.md` - Health check `is_expired()`
- `testnet/config/nonce-tracking.md` - SweepController nonce (different concept)
- `docs/api-reference.md` - `initialize`, `is_expired`, `expire`, `recover` APIs

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-24 | Initial expiry ledger computation guide |