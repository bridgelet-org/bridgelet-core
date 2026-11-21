# Runbook: Multi-Payment Edge Cases for `record_payment`

## Overview

Integrator-facing notes for **black-box** testing of `record_payment` on
testnet. The product docs state that an `EphemeralAccount` allows **one
payment per asset**. This runbook records what a second call for the **same
asset** does in practice on testnet so integration tests know what to assert.

This document does **not** prescribe contract source changes. Treat on-chain
results and events as authoritative if they ever diverge from the table below.

## Symptom

A test or integrator invokes `record_payment` twice against the same
`EphemeralAccount` with the same asset contract id and needs a stable
expectation: error, no-op, or overwrite.

## Prerequisites

- Funded testnet identity (`stellar keys generate --global testnet-investigator`)
- Deployed `EphemeralAccount` id (see `testnet/registry/contract-status.md`)
- At least one SEP-41 test asset contract id on testnet
- Working Soroban RPC (see [rpc-outage.md](rpc-outage.md) if invokes fail generically)

## Observed testnet behavior (black-box)

| Scenario | Second call result | Original payment amount | Events on second call |
|---|---|---|---|
| Same asset, positive amount | **Fails** with contract error **`DuplicateAsset` (code 13)** | Unchanged | No additional payment event for that asset |
| Same asset, different amount | Still **`DuplicateAsset`** | Unchanged (not overwritten) | None for a successful second payment |
| Different asset | **Succeeds** | N/A (new asset row) | Payment / multi-payment event for the new asset |

**Summary for assertions:** a second `record_payment` for an asset that is
already recorded is **neither a silent no-op nor an overwrite**. It **errors**
with `DuplicateAsset`. The first successful amount remains the recorded value.

### What integration tests should assert

1. First `record_payment(asset_A, amount1)` → success.
2. Read-back (e.g. `get_payment` / account info) shows `amount1` for `asset_A`.
3. Second `record_payment(asset_A, amount2)` → **error** `DuplicateAsset`.
4. Read-back still shows `amount1` (not `amount2`).
5. `record_payment(asset_B, amount3)` → success (multi-asset path).

## Verification procedure (testnet CLI)

### Case 1 — duplicate asset errors

```bash
ACCOUNT_ID=<EPHEMERAL_ACCOUNT_ID>
USDC_ID=<USDC_TOKEN_CONTRACT_ID>

# First payment — expect success
stellar contract invoke \
  --id "$ACCOUNT_ID" \
  --network testnet \
  --source testnet-investigator \
  -- \
  record_payment \
  --amount 100000000 \
  --asset "$USDC_ID"

# Second payment, same asset — expect DuplicateAsset (code 13)
stellar contract invoke \
  --id "$ACCOUNT_ID" \
  --network testnet \
  --source testnet-investigator \
  -- \
  record_payment \
  --amount 50000000 \
  --asset "$USDC_ID"
```

### Case 2 — different asset succeeds

```bash
OTHER_ASSET_ID=<OTHER_TOKEN_CONTRACT_ID>
stellar contract invoke \
  --id "$ACCOUNT_ID" \
  --network testnet \
  --source testnet-investigator \
  -- \
  record_payment \
  --amount 500000000 \
  --asset "$OTHER_ASSET_ID"
```

### Case 3 — confirm no overwrite after failed duplicate

After Case 1 fails, invoke whatever read method the deployed interface exposes
for per-asset payment data (e.g. `get_payment` / `get_info`). The amount for
`USDC_ID` must still match the **first** successful call.

## Concurrent submissions

If two `record_payment` transactions for the same asset are submitted close
together:

- At most one should commit as success.
- The other should fail with `DuplicateAsset` (or fail simulation if the first
  is already reflected in the simulation footprint).
- Do not assert both success.

## Related documentation

- `testnet/config/multi-asset-testing.md` — multi-asset test matrix
- `testnet/examples/idempotency-considerations.md` — broader idempotency notes
- `docs/api-reference.md` — `record_payment` API surface
- [rpc-outage.md](rpc-outage.md) — when invokes fail for infrastructure reasons
