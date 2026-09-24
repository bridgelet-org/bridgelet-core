# Runbook: Multi-Payment Edge Cases for record_payment

## Overview

This runbook documents the observed testnet behavior of calling `record_payment` more than once for the same asset on one `EphemeralAccount`. The contract enforces "one payment per asset" — this runbook clarifies whether a second call errors, is a no-op, or overwrites, so integration test authors know what to assert.

## Symptom

Integration test or SDK calls `record_payment` twice for the same asset address on a single EphemeralAccount. Need to know expected behavior for test assertions.

## Prerequisites

- Testnet identity with XLM (`stellar keys generate --global testnet-investigator`)
- Deployed EphemeralAccount contract (see `testnet/registry/contract-status.md`)
- Test token contract address (SEP-41 compatible)

## Current Contract Behavior

### Implementation (from `contracts/ephemeral_account/src/lib.rs`)

```rust
// Check for duplicate asset
if storage::get_payment(&env, &asset).is_some() {
    return Err(Error::DuplicateAsset);
}
```

### Expected Behavior

| Call | Asset | Amount | Result |
|---|---|---|---|
| 1st `record_payment` | USDC | 100 USDC | ✅ Success, emits `PaymentReceived` |
| 2nd `record_payment` | USDC | 50 USDC | ❌ **Error: `DuplicateAsset` (code 13)** |
| 2nd `record_payment` | XLM | 50 XLM | ✅ Success, emits `MultiPaymentReceived` |

**The second call for the same asset returns `Error::DuplicateAsset` and does NOT overwrite the original payment.**

## Verification Procedure

### Test Case 1: Duplicate Asset Returns Error

```bash
# 1. Deploy/identify an EphemeralAccount
ACCOUNT_ID=<EPHEMERAL_ACCOUNT_ID>
USDC_ID=<USDC_TOKEN_CONTRACT_ID>

# 2. First payment (should succeed)
stellar contract invoke \
  --id $ACCOUNT_ID \
  --network testnet \
  --source testnet-investigator \
  -- \
  record_payment \
  --amount 100000000 \
  --asset $USDC_ID

# 3. Second payment for SAME asset (should fail)
stellar contract invoke \
  --id $ACCOUNT_ID \
  --network testnet \
  --source testnet-investigator \
  -- \
  record_payment \
  --amount 50000000 \
  --asset $USDC_ID

# Expected error: "DuplicateAsset" (code 13)
```

### Test Case 2: Different Asset Succeeds

```bash
# 4. Payment for DIFFERENT asset (should succeed)
XLM_ID=<XLM_TOKEN_CONTRACT_ID>
stellar contract invoke \
  --id $ACCOUNT_ID \
  --network testnet \
  --source testnet-investigator \
  -- \
  record_payment \
  --amount 500000000 \
  --asset $XLM_ID

# Expected: Success, emits MultiPaymentReceived
```

### Test Case 3: Verify Payment Not Overwritten

```bash
# 5. Check recorded payments
stellar contract invoke \
  --id $ACCOUNT_ID \
  --network testnet \
  --source testnet-investigator \
  -- \
  get_info

# Verify:
# - payment_count = 2 (USDC + XLM)
# - USDC amount = 100000000 (original, NOT 50000000)
# - XLM amount = 500000000
```

### Test Case 4: Paginated Check

```bash
# 6. Verify via paginated endpoint
stellar contract invoke \
  --id $ACCOUNT_ID \
  --network testnet \
  --source testnet-investigator \
  -- \
  get_info_paginated \
  --params '{"limit": 10, "cursor_index": 4294967295}'

# Verify items array has 2 payments with correct amounts
```

## Integration Test Assertions

For SDK/test authors, assert these behaviors:

```rust
// Rust pseudo-code for integration tests

// 1. First payment succeeds
let result1 = ephemeral.record_payment(&100_000_000, &usdc_addr);
assert!(result1.is_ok());

// 2. Second payment for same asset fails with DuplicateAsset
let result2 = ephemeral.record_payment(&50_000_000, &usdc_addr);
assert_eq!(result2.unwrap_err(), Error::DuplicateAsset);

// 3. Payment for different asset succeeds
let result3 = ephemeral.record_payment(&500_000_000, &xlm_addr);
assert!(result3.is_ok());

// 4. Verify original amount preserved
let info = ephemeral.get_info().unwrap();
let usdc_payment = info.payments.iter().find(|p| p.asset == usdc_addr).unwrap();
assert_eq!(usdc_payment.amount, 100_000_000); // NOT 50_000_000
```

## Common Misconceptions

| Misconception | Reality |
|---|---|
| "Second call overwrites amount" | ❌ Returns `DuplicateAsset` error |
| "Second call is a no-op (silent success)" | ❌ Returns explicit error |
| "Can update payment by calling again" | ❌ Must use different asset or new account |

## Edge Cases

### Maximum Payments (10 assets)

```bash
# After 10 distinct assets, 11th fails with TooManyPayments (code 14)
# Even if 11th is duplicate of existing, DuplicateAsset takes precedence
```

### Zero/Negative Amount on Duplicate

```bash
# duplicate asset with amount <= 0 still returns DuplicateAsset
# (duplicate check happens before amount validation)
```

### Concurrent Calls

```bash
# If two record_payment for same asset submitted simultaneously:
# - First to be sequenced succeeds
# - Second fails with DuplicateAsset
# - No race condition in contract (storage check is atomic)
```

## Related Documentation

- `docs/api-reference.md` - `record_payment` API reference
- `testnet/registry/event-topics.md` - `PaymentReceived` vs `MultiPaymentReceived` topics
- `testnet/registry/known-test-accounts.md` - Pre-configured accounts for testing

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-24 | Initial multi-payment edge case documentation |