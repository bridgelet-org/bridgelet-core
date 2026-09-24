# Testnet Contract Health Check Procedure

## Overview

This document describes the exact `soroban contract invoke` (or RPC call) procedures to confirm a given testnet contract ID is live, unpaused, and returns sane values for read-only methods. This is a lightweight health-check procedure distinct from automated monitoring.

## Prerequisites

- Testnet identity with XLM for queries (`stellar keys generate --global testnet-healthcheck`)
- Stellar CLI installed (`cargo install --locked stellar-cli --version 23.4.1`)
- Testnet network configured (`stellar network add --global testnet --rpc-url https://soroban-testnet.stellar.org --network-passphrase "Test SDF Network ; September 2015"`)

---

## Health Check Commands by Contract

### 1. EphemeralAccount Contract

**Expected deployed**: Yes (see `testnet/registry/contract-status.md`)

#### Basic Liveness Check
```bash
# Check contract exists and responds
stellar contract invoke \
  --id <EPHEMERAL_ACCOUNT_ID> \
  --network testnet \
  --source testnet-healthcheck \
  -- \
  get_status
```
**Expected**: Returns `0` (Active) for uninitialized, `1` (PaymentReceived), `2` (Swept), or `3` (Expired)

#### Check Expiry Logic
```bash
# Verify is_expired returns false for active accounts
stellar contract invoke \
  --id <EPHEMERAL_ACCOUNT_ID> \
  --network testnet \
  --source testnet-healthcheck \
  -- \
  is_expired
```
**Expected**: `false` for accounts not yet expired, `true` for expired accounts

#### Verify Payment Recording Works
```bash
# For initialized accounts, check payment count
stellar contract invoke \
  --id <EPHEMERAL_ACCOUNT_ID> \
  --network testnet \
  --source testnet-healthcheck \
  -- \
  get_payment_count
```
**Expected**: `0` (no payments) to `10` (max payments)

#### Full State Check (if initialized)
```bash
# Get complete account info
stellar contract invoke \
  --id <EPHEMERAL_ACCOUNT_ID> \
  --network testnet \
  --source testnet-healthcheck \
  -- \
  get_info
```
**Expected**: Returns `AccountInfo` struct with valid fields:
- `creator`: Valid address
- `status`: 0-3
- `expiry_ledger`: Future ledger (if Active/PaymentReceived)
- `recovery_address`: Valid address
- `payment_count`: Matches `get_payment_count`
- `payments`: Array of Payment structs
- `swept_to`: None (if not swept/expired) or valid address

#### Paginated Payment Check
```bash
# Test pagination endpoint
stellar contract invoke \
  --id <EPHEMERAL_ACCOUNT_ID> \
  --network testnet \
  --source testnet-healthcheck \
  -- \
  get_info_paginated \
  --params '{"limit": 5, "cursor_index": 4294967295}'
```
**Expected**: Returns `PaginatedPaymentResponse` with:
- `items`: Array of Payment (0-5 items)
- `next_cursor_index`: Valid index or `4294967295` (NO_CURSOR)
- `total_count`: Matches `payment_count`

### 2. SweepController Contract

**Expected deployed**: Yes (see `testnet/registry/contract-status.md`)

#### Basic Liveness Check
```bash
# Check contract exists and responds
stellar contract invoke \
  --id <SWEEP_CONTROLLER_ID> \
  --network testnet \
  --source testnet-healthcheck \
  -- \
  can_sweep \
  --ephemeral_account <KNOWN_EPHEMERAL_ACCOUNT_ID>
```
**Expected**: `false` (for accounts with no payment) or `true` (for sweepable accounts)

#### Verify Nonce Tracking
```bash
# Get current nonce (should be 0 if no sweeps executed)
stellar contract invoke \
  --id <SWEEP_CONTROLLER_ID> \
  --network testnet \
  --source testnet-healthcheck \
  -- \
  get_nonce
```
**Expected**: `0` (no sweeps yet) or positive integer (number of sweeps executed)

#### Check Destination Locking (if locked mode)
```bash
# Verify authorized destination if locked
stellar contract invoke \
  --id <SWEEP_CONTROLLER_ID> \
  --network testnet \
  --source testnet-healthcheck \
  -- \
  # No direct getter, but update_authorized_destination would fail if nonce > 0
```

### 3. ReserveContract Contract

**Expected deployed**: **No** (see `testnet/registry/reserve-contract-status.md`)

#### If/When Deployed:
```bash
# Check base reserve value
stellar contract invoke \
  --id <RESERVE_CONTRACT_ID> \
  --network testnet \
  --source testnet-healthcheck \
  -- \
  get_base_reserve
```
**Expected**: `Some(<stroops>)` or `None` (if not set)

```bash
# Check if reserve is configured
stellar contract invoke \
  --id <RESERVE_CONTRACT_ID> \
  --network testnet \
  --source testnet-healthcheck \
  -- \
  has_base_reserve
```
**Expected**: `true` or `false`

### 4. AccountFactory Contract

**Expected deployed**: **No** (see `testnet/registry/account-factory-status.md`)

#### If/When Deployed:
```bash
# Check total accounts for a request batch
stellar contract invoke \
  --id <ACCOUNT_FACTORY_ID> \
  --network testnet \
  --source testnet-healthcheck \
  -- \
  batch_initialize_count \
  --requests '[{"expiry_ledger": 999999999, "recovery_address": "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"}, ...]'
```
**Expected**: Returns count matching request array length

```bash
# Test paginated initialization (dry-run with small limit)
stellar contract invoke \
  --id <ACCOUNT_FACTORY_ID> \
  --network testnet \
  --source testnet-healthcheck \
  -- \
  batch_initialize_paginated \
  --creator <TEST_CREATOR> \
  --requests '[...]' \
  --params '{"limit": 1, "cursor_index": 4294967295}'
```
**Expected**: Returns `PaginatedAccountInitResultResponse` with 1 item

---

## RPC Direct Calls (Alternative to CLI)

For programmatic health checks, use Soroban RPC directly:

### Get Contract Events (Verify Recent Activity)
```bash
curl -X POST https://soroban-testnet.stellar.org \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getEvents",
    "params": {
      "startLedger": 1000000,
      "filters": [{
        "type": "contract",
        "contractIds": ["<CONTRACT_ID>"],
        "topics": [["created"], ["payment"], ["sweep"]]
      }]
    }
  }'
```

### Simulate Transaction (Verify Contract Logic)
```bash
curl -X POST https://soroban-testnet.stellar.org \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "simulateTransaction",
    "params": {
      "transaction": "<BASE64_ENVELOPE>",
      "resourceConfig": {
        "instructionLeeway": 1000000
      }
    }
  }'
```

---

## Health Check Script

Save as `healthcheck.sh` and run periodically:

```bash
#!/bin/bash
# healthcheck.sh - Bridgelet testnet contract health check

set -e

EPHEMERAL_ID="${EPHEMERAL_ID:-<EPHEMERAL_ACCOUNT_ID>}"
SWEEP_ID="${SWEEP_ID:-<SWEEP_CONTROLLER_ID>}"
IDENTITY="${IDENTITY:-testnet-healthcheck}"

echo "=== Bridgelet Testnet Health Check ==="
echo "Time: $(date -u)"
echo "EphemeralAccount: $EPHEMERAL_ID"
echo "SweepController: $SWEEP_ID"
echo ""

echo "1. EphemeralAccount get_status:"
stellar contract invoke --id "$EPHEMERAL_ID" --network testnet --source "$IDENTITY" -- get_status

echo ""
echo "2. EphemeralAccount is_expired:"
stellar contract invoke --id "$EPHEMERAL_ID" --network testnet --source "$IDENTITY" -- is_expired

echo ""
echo "3. EphemeralAccount get_payment_count:"
stellar contract invoke --id "$EPHEMERAL_ID" --network testnet --source "$IDENTITY" -- get_payment_count

echo ""
echo "4. SweepController get_nonce:"
stellar contract invoke --id "$SWEEP_ID" --network testnet --source "$IDENTITY" -- get_nonce

echo ""
echo "5. SweepController can_sweep (known account):"
stellar contract invoke --id "$SWEEP_ID" --network testnet --source "$IDENTITY" -- can_sweep --ephemeral_account "$EPHEMERAL_ID"

echo ""
echo "=== Health Check Complete ==="
```

Run with:
```bash
chmod +x healthcheck.sh
EPHEMERAL_ID=<ID> SWEEP_ID=<ID> ./healthcheck.sh
```

---

## Expected Values Reference

| Contract | Method | Healthy Response | Unhealthy Indicator |
|---|---|---|---|
| EphemeralAccount | `get_status` | 0, 1, 2, or 3 | Error / timeout |
| EphemeralAccount | `is_expired` | `true` or `false` | Error / timeout |
| EphemeralAccount | `get_payment_count` | 0-10 | Error / timeout / >10 |
| EphemeralAccount | `get_info` | Valid AccountInfo | Error / timeout / invalid fields |
| SweepController | `get_nonce` | ≥0 integer | Error / timeout |
| SweepController | `can_sweep` | `true` or `false` | Error / timeout |
| ReserveContract | `has_base_reserve` | `true` or `false` | Error / timeout / not deployed |
| AccountFactory | `batch_initialize_count` | ≥0 integer | Error / timeout / not deployed |

---

## Troubleshooting

| Symptom | Likely Cause | Action |
|---|---|---|
| "Contract not found" | Wrong contract ID or not deployed | Verify ID in `contract-status.md` |
| "Host function error" | Contract panicked / logic error | Check contract logs; may need upgrade |
| Timeout (>30s) | RPC overload / contract infinite loop | Retry; check RPC status |
| Invalid XDR response | Contract upgraded / interface changed | Verify WASM hash matches expected |

---

## Related Documentation

- `testnet/registry/contract-status.md` - Current contract IDs and status
- `testnet/registry/event-topics.md` - Event topic reference for monitoring
- `testnet/registry/upgrade-history.md` - Track contract upgrades
- `testnet/config/nonce-tracking.md` - SweepController nonce inspection

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-24 | Initial health check procedure for testnet contracts |