# Operational Runbook: Bulk TTL Extension Sweep

## Overview

This runbook describes the procedure for performing a bulk Time-To-Live (TTL) extension pass across all deployed Bridgelet contracts after an incident or as part of routine maintenance. TTL management is critical in Soroban to prevent contract storage from expiring and becoming inaccessible.

## Prerequisites

- Access to the Stellar network RPC endpoint (mainnet or testnet as appropriate)
- Stellar SDK or Soroban RPC client installed
- Administrative access to sign transactions for each contract
- The deployment artifacts file containing central contract IDs
- Ability to query on-chain events and ledger state

## Contract TTL Extension Status

### Current Implementation Analysis

**Reserve Contract:**
- **Status:** ✅ Automatically extends TTL on all function calls
- **Mechanism:** Every public function calls `storage::extend_instance_ttl(&env)` before execution
- **Functions with TTL extension:** `initialize()`, `set_base_reserve()`, `get_base_reserve()`, `require_base_reserve()`, `has_base_reserve()`, `get_admin()`
- **Action required:** Call any read-only function (e.g., `get_base_reserve()`) to extend TTL

**Ephemeral Account Contract:**
- **Status:** ❌ No automatic TTL extension mechanism
- **Mechanism:** None - contract does not call `extend_instance_ttl()` in any function
- **Action required:** Manual TTL extension via Soroban SDK or direct ledger operation

**Account Factory:**
- **Status:** ❌ No automatic TTL extension mechanism
- **Mechanism:** None - contract does not call `extend_instance_ttl()` in any function
- **Action required:** Manual TTL extension via Soroban SDK or direct ledger operation

**Sweep Controller:**
- **Status:** ❌ No automatic TTL extension mechanism
- **Mechanism:** None - contract does not call `extend_instance_ttl()` in any function
- **Action required:** Manual TTL extension via Soroban SDK or direct ledger operation

## Step 1: Enumerate Contract Addresses

### 1.1 Load Central Contract IDs from Deployment Artifacts

Read the central contract IDs from `deployment-artifacts/contract-ids.txt`:

```bash
cat deployment-artifacts/contract-ids.txt
```

Expected output:
```
EPHEMERAL_ACCOUNT_CONTRACT_ID=CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3
SWEEP_CONTROLLER_CONTRACT_ID=CBEU4X5MNGOECBSTNEUFMBALH2YI5YV4UIH7YXRNOLR2DNLZQD4Z5KWE
RESERVE_CONTRACT_CONTRACT_ID=CACYMICFLHSPKVMCK336IEGCWAQB4XBC7P7HTJSQY3R7Y2YASE4AX3MS
ACCOUNT_FACTORY_CONTRACT_ID=CARBMZY4466SWP3RTN3DR2F4JYNR5UCXD2ICFISV2YI4TS3AAZV2TQ24
```

### 1.2 Enumerate Ephemeral Account Addresses from On-Chain Events

Query the ledger for all `AccountCreated` events emitted by the Account Factory:

```bash
# Using Soroban RPC
soroban rpc events \
  --contract-id ACCOUNT_FACTORY_CONTRACT_ID \
  --event-type AccountCreated \
  --limit 1000
```

Or using Stellar SDK:

```javascript
const response = await server.getEvents({
  contract: ACCOUNT_FACTORY_CONTRACT_ID,
  topics: ["AccountCreated"],
  limit: 1000,
});

const ephemeralAccounts = response.records.map(record => {
  // Extract account address from event value
  return record.value.account_address;
});
```

**Alternative: Query Account Factory Storage**

If events are unavailable or incomplete, query the Account Factory's deployment tracking (if implemented):

```bash
# Check if factory stores deployed addresses
soroban rpc get_ledger_entries \
  --contract-id ACCOUNT_FACTORY_CONTRACT_ID \
  --key "DeployedAccounts"
```

### 1.3 Compile Complete Contract List

Create a comprehensive list of all contracts requiring TTL extension:

```json
{
  "central_contracts": {
    "ephemeral_account_template": "CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3",
    "sweep_controller": "CBEU4X5MNGOECBSTNEUFMBALH2YI5YV4UIH7YXRNOLR2DNLZQD4Z5KWE",
    "reserve_contract": "CACYMICFLHSPKVMCK336IEGCWAQB4XBC7P7HTJSQY3R7Y2YASE4AX3MS",
    "account_factory": "CARBMZY4466SWP3RTN3DR2F4JYNR5UCXD2ICFISV2YI4TS3AAZV2TQ24"
  },
  "ephemeral_accounts": [
    "EPHEMERAL_ACCOUNT_1",
    "EPHEMERAL_ACCOUNT_2",
    // ... all discovered ephemeral accounts
  ]
}
```

## Step 2: Extend TTL for Central Contracts

### 2.1 Reserve Contract (Automatic Extension)

The Reserve Contract automatically extends TTL on any function call. Use a read-only function:

```bash
# Call get_base_reserve() to extend TTL
soroban rpc invoke \
  --contract-id CACYMICFLHSPKVMCK336IEGCWAQB4XBC7P7HTJSQY3R7Y2YASE4AX3MS \
  --function get_base_reserve \
  --id
```

Or using Stellar SDK:

```javascript
const contract = new Contract(reserveContractId);
const result = await contract.get_base_reserve();
// This call automatically extends TTL
```

**Expected result:** Function returns the current base reserve value (or None if not set). TTL is extended automatically.

### 2.2 Account Factory (Manual Extension Required)

The Account Factory does not have automatic TTL extension. Use Soroban SDK to manually extend:

```bash
# Using Soroban CLI
soroban contract extend \
  --contract-id CARBMZY4466SWP3RTN3DR2F4JYNR5UCXD2ICFISV2YI4TS3AAZV2TQ24 \
  --ledger-entries-to-extend 100 \
  --source ADMIN_KEY
```

Or using Stellar SDK:

```javascript
const transaction = new TransactionBuilder(account, {
  fee: BASE_FEE,
  networkPassphrase: NETWORK_PASSPHRASE,
})
  .setTimeout(30)
  .addOperation(
    Operation.extendFootprintTtl({
      ext: 100, // Extend by 100 ledgers
    })
  )
  .build();

transaction.sign(adminKeypair);
await server.sendTransaction(transaction);
```

### 2.3 Sweep Controller (Manual Extension Required)

The Sweep Controller does not have automatic TTL extension. Use the same manual extension method:

```bash
soroban contract extend \
  --contract-id CBEU4X5MNGOECBSTNEUFMBALH2YI5YV4UIH7YXRNOLR2DNLZQD4Z5KWE \
  --ledger-entries-to-extend 100 \
  --source ADMIN_KEY
```

### 2.4 Ephemeral Account Template (Manual Extension Required)

The template contract used by the Account Factory requires manual extension:

```bash
soroban contract extend \
  --contract-id CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3 \
  --ledger-entries-to-extend 100 \
  --source ADMIN_KEY
```

## Step 3: Extend TTL for Ephemeral Accounts

### 3.1 Batch Extension for All Ephemeral Accounts

Since ephemeral accounts do not have automatic TTL extension, perform manual extension for each:

```bash
#!/bin/bash
# batch_extend_ephemeral_accounts.sh

EPHEMERAL_ACCOUNTS=(
  "ACCOUNT_ID_1"
  "ACCOUNT_ID_2"
  # ... add all discovered accounts
)

LEDGER_ENTRIES_TO_EXTEND=100
SOURCE_KEY="ADMIN_KEY"

for account in "${EPHEMERAL_ACCOUNTS[@]}"; do
  echo "Extending TTL for $account"
  soroban contract extend \
    --contract-id "$account" \
    --ledger-entries-to-extend "$LEDGER_ENTRIES_TO_EXTEND" \
    --source "$SOURCE_KEY"
  
  # Add delay to avoid rate limiting
  sleep 1
done
```

### 3.2 Filter by Status (Optional)

Optionally, filter ephemeral accounts by status to prioritize active accounts:

```javascript
// For each ephemeral account, check status
const contract = new Contract(accountId);
const info = await contract.get_info();

// Only extend if account is in active state
if (info.status === "Active" || info.status === "PaymentReceived") {
  await extendTTL(accountId);
}
```

Skip accounts that are already:
- `Swept` - funds already transferred, account no longer needed
- `Expired` - past expiry, recovery already handled

## Step 4: Verify TTL Extension

### 4.1 Verify Central Contract TTL

Query the ledger to confirm TTL was extended for each central contract:

```bash
# Check Reserve Contract TTL
soroban rpc get_ledger_entries \
  --contract-id CACYMICFLHSPKVMCK336IEGCWAQB4XBC7P7HTJSQY3R7Y2YASE4AX3MS \
  --key "Instance"
```

Expected response includes:
```json
{
  "key": "...",
  "live_until_ledger": 12345678,  // This should be > current ledger
  "last_modified_ledger": 12345000
}
```

Calculate remaining TTL:
```bash
CURRENT_LEDGER=$(soroban rpc info | jq .current_ledger)
LIVE_UNTIL=$(soroban rpc get_ledger_entries ... | jq .live_until_ledger)
REMAINING=$((LIVE_UNTIL - CURRENT_LEDGER))
echo "Remaining TTL: $REMAINING ledgers"
```

### 4.2 Verify Ephemeral Account TTL

Batch verify all ephemeral accounts:

```bash
#!/bin/bash
# verify_ephemeral_ttl.sh

for account in "${EPHEMERAL_ACCOUNTS[@]}"; do
  echo "Checking TTL for $account"
  
  TTL_INFO=$(soroban rpc get_ledger_entries \
    --contract-id "$account" \
    --key "Instance")
  
  LIVE_UNTIL=$(echo "$TTL_INFO" | jq .live_until_ledger)
  CURRENT_LEDGER=$(soroban rpc info | jq .current_ledger)
  REMAINING=$((LIVE_UNTIL - CURRENT_LEDGER))
  
  echo "  Live until ledger: $LIVE_UNTIL"
  echo "  Current ledger: $CURRENT_LEDGER"
  echo "  Remaining TTL: $REMAINING ledgers"
  
  if [ "$REMAINING" -lt 10000 ]; then
    echo "  WARNING: Low TTL ($REMAINING ledgers)"
  fi
done
```

### 4.3 Generate Verification Report

Create a summary report of all TTL extensions:

```json
{
  "timestamp": "2026-07-24T14:00:00Z",
  "current_ledger": 12345678,
  "extensions_performed": {
    "central_contracts": {
      "reserve_contract": {
        "contract_id": "CACYMICFLHSPKVMCK336IEGCWAQB4XBC7P7HTJSQY3R7Y2YASE4AX3MS",
        "method": "automatic",
        "function_called": "get_base_reserve",
        "previous_live_until": 12345000,
        "new_live_until": 12355000,
        "status": "success"
      },
      "account_factory": {
        "contract_id": "CARBMZY4466SWP3RTN3DR2F4JYNR5UCXD2ICFISV2YI4TS3AAZV2TQ24",
        "method": "manual",
        "ledger_entries_extended": 100,
        "previous_live_until": 12344000,
        "new_live_until": 12354000,
        "status": "success"
      },
      "sweep_controller": {
        "contract_id": "CBEU4X5MNGOECBSTNEUFMBALH2YI5YV4UIH7YXRNOLR2DNLZQD4Z5KWE",
        "method": "manual",
        "ledger_entries_extended": 100,
        "previous_live_until": 12344500,
        "new_live_until": 12354500,
        "status": "success"
      },
      "ephemeral_account_template": {
        "contract_id": "CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3",
        "method": "manual",
        "ledger_entries_extended": 100,
        "previous_live_until": 12344800,
        "new_live_until": 12354800,
        "status": "success"
      }
    },
    "ephemeral_accounts": {
      "total_processed": 150,
      "successful": 148,
      "failed": 2,
      "failed_accounts": [
        "FAILED_ACCOUNT_ID_1",
        "FAILED_ACCOUNT_ID_2"
      ]
    }
  },
  "recommendations": [
    "Investigate failed ephemeral account extensions",
    "Schedule next TTL extension in 90 days",
    "Consider implementing automatic TTL extension in ephemeral accounts"
  ]
}
```

## Step 5: Handle Failures and Edge Cases

### 5.1 Failed Extension Attempts

If an extension fails for a contract:

1. **Check if contract still exists:**
   ```bash
   soroban rpc get_ledger_entries --contract-id FAILED_CONTRACT_ID
   ```
   - If contract not found: It may have been deleted or merged. Remove from tracking.

2. **Check if contract is already expired:**
   ```bash
   # For ephemeral accounts
   soroban rpc invoke --contract-id FAILED_CONTRACT_ID --function is_expired
   ```
   - If expired: Account may need recovery instead of extension.

3. **Check authorization:**
   - Verify the signing key has permission to extend TTL
   - For ephemeral accounts, may need creator or admin authorization

### 5.2 Rate Limiting

If you encounter rate limiting:

1. **Increase delay between requests:**
   ```bash
   sleep 5  # Increase from 1 second to 5 seconds
   ```

2. **Batch operations:**
   - Process contracts in smaller batches
   - Wait between batches

3. **Use higher fee:**
   ```bash
   soroban contract extend --fee 10000 ...
   ```

### 5.3 Insufficient TTL Extension

If the extended TTL is insufficient:

1. **Increase extension amount:**
   ```bash
   soroban contract extend --ledger-entries-to-extend 500 ...
   ```

2. **Schedule more frequent extensions:**
   - Set up automated weekly or monthly extension passes

## Step 6: Schedule Future Extensions

### 6.1 Calculate Extension Frequency

Based on current TTL and extension amount:

```bash
# If extending by 100 ledgers
# Stellar closes a ledger approximately every 5 seconds
# 100 ledgers ≈ 500 seconds ≈ 8 minutes

# For practical purposes, extend by larger amounts:
# 100,000 ledgers ≈ 500,000 seconds ≈ 5.8 days
# 1,000,000 ledgers ≈ 5,000,000 seconds ≈ 58 days
```

### 6.2 Set Up Monitoring

Create alerts for contracts with low TTL:

```javascript
// Check TTL daily
async function checkTTLAlerts() {
  const contracts = await getAllContracts();
  const currentLedger = await getCurrentLedger();
  
  for (const contract of contracts) {
    const ttl = await getContractTTL(contract.id);
    const remaining = ttl.live_until_ledger - currentLedger;
    
    if (remaining < 10000) {  // Less than ~14 hours
      alert(`Low TTL alert: ${contract.id} has ${remaining} ledgers remaining`);
    }
  }
}
```

### 6.3 Automate Extension Schedule

Create a cron job or scheduled task:

```bash
# Run weekly extension sweep
0 0 * * 0 /path/to/batch_extend_ephemeral_accounts.sh
```

Or use a cloud scheduler (AWS Lambda, Google Cloud Functions, etc.) to run the extension script automatically.

## Recommendations for Future Improvements

### 1. Implement Automatic TTL Extension

**Ephemeral Account Contract:**
Add `extend_instance_ttl()` calls to frequently-used functions:
- `get_info()`
- `get_status()`
- `is_expired()`
- `simulate_sweep()`

**Account Factory:**
Add TTL extension to:
- `batch_initialize()` (already extends via deployment)
- Add a `ping()` function for read-only TTL extension

**Sweep Controller:**
Add TTL extension to:
- `get_nonce()`
- `can_sweep()`

### 2. Implement Contract Registry

Create a central registry contract that tracks all deployed ephemeral accounts:

```rust
pub fn register_account(env: Env, account_id: Address) {
    // Store account ID in registry
    // Automatically extend registry TTL on registration
}

pub fn get_all_accounts(env: Env) -> Vec<Address> {
    // Return all registered accounts
}
```

This eliminates the need to query events for account enumeration.

### 3. Add TTL Monitoring Events

Emit events when TTL is extended:

```rust
pub fn extend_ttl(env: Env) {
    env.storage().instance().extend_ttl(10000);
    env.events().publish((symbol!("ttl_extended"),), env.ledger().sequence());
}
```

This enables off-chain monitoring of extension operations.

### 4. Implement TTL Extension Thresholds

Add logic to automatically extend TTL when it falls below a threshold:

```rust
pub fn auto_extend_if_needed(env: Env) {
    let current_ledger = env.ledger().sequence();
    let live_until = env.storage().instance().live_until_ledger();
    
    if live_until - current_ledger < 10000 {
        env.storage().instance().extend_ttl(100000);
    }
}
```

## Emergency Procedures

### TTL Critical (Less than 1000 Ledgers)

If any contract has critically low TTL:

1. **Immediate extension:**
   ```bash
   soroban contract extend --ledger-entries-to-extend 1000000 --priority high
   ```

2. **Escalate to team:**
   - Notify operations team
   - Document incident
   - Review why TTL was not extended earlier

3. **Post-incident review:**
   - Update monitoring thresholds
   - Improve automation
   - Review extension schedule

### Contract Already Expired

If a contract has already expired:

1. **For ephemeral accounts:**
   - Attempt recovery via `recover()` function
   - If recovery fails, funds may be lost
   - Document the incident

2. **For central contracts:**
   - This is a critical incident
   - May require redeployment
   - All dependent contracts may be affected

## Appendix: Quick Reference Commands

### Extend Single Contract
```bash
soroban contract extend \
  --contract-id CONTRACT_ID \
  --ledger-entries-to-extend 100000 \
  --source SOURCE_KEY
```

### Check Contract TTL
```bash
soroban rpc get_ledger_entries \
  --contract-id CONTRACT_ID \
  --key "Instance" | jq '.live_until_ledger'
```

### Get Current Ledger
```bash
soroban rpc info | jq '.current_ledger'
```

### Query AccountCreated Events
```bash
soroban rpc events \
  --contract-id ACCOUNT_FACTORY_ID \
  --event-type AccountCreated \
  --limit 1000
```

### Call Reserve Contract (Auto-extends TTL)
```bash
soroban rpc invoke \
  --contract-id RESERVE_CONTRACT_ID \
  --function get_base_reserve
```
