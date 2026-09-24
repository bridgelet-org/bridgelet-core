# Runbook: Investigating AccountFactory batch_initialize Partial Failure

## Overview

This runbook covers the investigative steps to diagnose a partial failure in `AccountFactory::batch_initialize` on testnet. Since the current implementation discards per-account error details (returns `success: false, error: None`), this runbook provides a manual checklist to identify the root cause given only a failed account address.

## Prerequisites

- Access to Stellar Testnet RPC (`https://soroban-testnet.stellar.org`)
- Testnet identity with sufficient XLM for queries (`stellar keys generate --global testnet-investigator`)
- Contract IDs for deployed AccountFactory and EphemeralAccount WASM hash

## Background

`AccountFactory::batch_initialize` deploys N ephemeral accounts in a single transaction using deterministic salts (index-based). Each account is initialized with:
- `creator` = caller (for both `authorized_controller` and `admin`)
- `expiry_ledger` = from request
- `recovery_address` = from request

If any individual `initialize()` call fails, the factory catches the error but discards the error detail, returning only `AccountInitResult { success: false, error: None }`.

## Investigation Steps

### 1. Identify Failed Accounts

```bash
# Get the batch_initialize transaction result
# Look for AccountInitResult entries with success: false
stellar contract invoke \
  --id <ACCOUNT_FACTORY_ID> \
  --network testnet \
  --source testnet-investigator \
  -- \
  batch_initialize_count \
  --requests '[...]'
```

### 2. Check Individual Initialize Preconditions

For each failed account address, manually verify each `initialize()` precondition:

#### A. Check if Already Initialized
```bash
stellar contract invoke \
  --id <FAILED_ACCOUNT_ADDRESS> \
  --network testnet \
  --source testnet-investigator \
  -- \
  get_status
```
- If status ≠ `Active` (0), the account was already initialized
- Expected: `Active` (0) for uninitialized accounts

#### B. Verify Expiry Ledger is in Future
```bash
# Get current ledger
CURRENT_LEDGER=$(stellar network current-ledger --rpc-url https://soroban-testnet.stellar.org)

# Check expiry from factory request
# expiry_ledger must be > CURRENT_LEDGER
```

#### C. Check Creator Authorization
The factory uses the caller as `creator` for both `authorized_controller` and `admin`. Verify the caller authorized the factory call:
```bash
# Check the factory transaction - creator must have signed
stellar contract invoke \
  --id <ACCOUNT_FACTORY_ID> \
  --network testnet \
  --source testnet-investigator \
  -- \
  batch_initialize_count \
  --requests '[...]'
```

#### D. Verify WASM Hash is Set
```bash
# Check factory has WASM hash stored
stellar contract invoke \
  --id <ACCOUNT_FACTORY_ID> \
  --network testnet \
  --source testnet-investigator \
  -- \
  # (no direct getter, but deploy would fail if not set)
```

#### E. Check Salt Collision
Each account uses salt = index (4 bytes at position 28-31). Verify no previous deployment used same salt:
```bash
# Compute expected address for index N
# salt = [0,0,...,0, index_be_bytes]
# address = deployer.with_current_contract(salt).deploy_v2(wasm_hash, ())
```

#### F. Verify EphemeralAccount WASM is Valid
```bash
# Check WASM hash in factory matches deployed code
stellar contract invoke \
  --id <EPHEMERAL_ACCOUNT_ID> \
  --network testnet \
  --source testnet-investigator \
  -- \
  get_status
```
If this fails with "contract not found", the WASM deployment failed.

### 3. Common Failure Modes

| Failure Mode | Symptoms | Resolution |
|--------------|----------|------------|
| `AlreadyInitialized` | Account address already has non-Active status | Use different salt/index; check for duplicate deployments |
| `InvalidExpiry` | `expiry_ledger` ≤ current ledger | Use future ledger (current + buffer) |
| WASM not found | Deploy fails silently | Rebuild and redeploy WASM; update factory |
| Out of gas | Batch too large | Use `batch_initialize_paginated` with smaller pages |
| Salt collision | Deploy fails with "already exists" | Use higher index range |

### 4. Debugging Commands

```bash
# Get current testnet ledger
stellar network current-ledger --rpc-url https://soroban-testnet.stellar.org

# Simulate initialize call for specific account (dry-run)
stellar contract invoke \
  --id <EPHEMERAL_ACCOUNT_WASM_HASH> \
  --network testnet \
  --source testnet-investigator \
  -- \
  initialize \
  --creator <CALLER> \
  --expiry_ledger <FUTURE_LEDGER> \
  --recovery_address <RECOVERY> \
  --authorized_controller <CALLER> \
  --admin <CALLER>

# Check account status directly
stellar contract invoke \
  --id <FAILED_ACCOUNT> \
  --network testnet \
  --source testnet-investigator \
  -- \
  get_status
```

### 5. Using Paginated Batch Initialize

To avoid resource limits and get better error isolation:

```bash
# First page (limit 10)
stellar contract invoke \
  --id <ACCOUNT_FACTORY_ID> \
  --network testnet \
  --source testnet-deployer \
  -- \
  batch_initialize_paginated \
  --creator <CREATOR> \
  --requests '[...]' \
  --params '{"limit": 10, "cursor_index": 4294967295}'

# Subsequent pages (use next_cursor_index from previous response)
stellar contract invoke \
  --id <ACCOUNT_FACTORY_ID> \
  --network testnet \
  --source testnet-deployer \
  -- \
  batch_initialize_paginated \
  --creator <CREATOR> \
  --requests '[...]' \
  --params '{"limit": 10, "cursor_index": 10}'
```

## Escalation

If manual checks don't reveal the cause:
1. Check RPC logs for the transaction
2. Simulate the failed initialize call with `--dry-run` flag (if supported)
3. Contact Bridgelet core team with:
   - Failed account address
   - Factory transaction hash
   - Request parameters used
   - Current ledger at time of failure

## Related Documentation

- `testnet/registry/account-factory-status.md` - Factory deployment status
- `testnet/registry/contract-status.md` - Contract deployment overview
- `docs/api-reference.md` - AccountFactory API reference