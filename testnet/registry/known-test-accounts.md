# Known Testnet Ephemeral Accounts

## Overview

This document maintains a small set of long-lived, intentionally non-swept testnet `EphemeralAccount` instances for demo/documentation purposes. These accounts can be referenced in documentation, screenshots, and integration tests without disappearing when someone else sweeps or expires them.

## Account Registry

| Account ID | Purpose | Created | Expiry Ledger | Status | Notes |
|---|---|---|---|---|---|
| TBD | Demo account for screenshots | TBD | TBD | Pending deployment | Will be created after EphemeralAccount deployed |
| TBD | Integration test fixture | TBD | TBD | Pending deployment | Reserved for SDK test suite |
| TBD | Documentation walkthrough | TBD | TBD | Pending deployment | Used in CLI examples |

> **Note**: Accounts will be added once `EphemeralAccount` is deployed to testnet (see `testnet/registry/contract-status.md`).

## Creation Procedure

When EphemeralAccount is deployed, create known accounts using:

```bash
# 1. Create a dedicated identity for known accounts
stellar keys generate --global testnet-known-accounts
stellar keys fund testnet-known-accounts --network testnet

# 2. For each known account, deploy new EphemeralAccount instance
# Using AccountFactory (once deployed) or direct deploy:

# Direct deploy (requires WASM hash):
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/ephemeral_account.wasm \
  --network testnet \
  --source testnet-known-accounts

# 3. Initialize with far-future expiry (e.g., 10 years = ~63M ledgers)
CURRENT_LEDGER=$(stellar network current-ledger --rpc-url https://soroban-testnet.stellar.org)
EXPIRY=$((CURRENT_LEDGER + 63000000))

stellar contract invoke \
  --id <NEW_ACCOUNT_ID> \
  --network testnet \
  --source testnet-known-accounts \
  -- \
  initialize \
  --creator <KNOWN_ACCOUNTS_PUBLIC_KEY> \
  --expiry_ledger $EXPIRY \
  --recovery_address <KNOWN_ACCOUNTS_PUBLIC_KEY> \
  --authorized_controller <SWEEP_CONTROLLER_ID> \
  --admin <KNOWN_ACCOUNTS_PUBLIC_KEY>

# 4. Record a small test payment (optional, for PaymentReceived status)
stellar contract invoke \
  --id <NEW_ACCOUNT_ID> \
  --network testnet \
  --source testnet-known-accounts \
  -- \
  record_payment \
  --amount 1000000 \
  --asset <TEST_TOKEN_ID>
```

## Maintenance Rules

1. **Never sweep these accounts** - They are for documentation only
2. **Use far-future expiry** - Minimum 1 year, preferably 10+ years
3. **Document in this file** - Update table immediately after creation
4. **Monitor status** - Periodically verify they remain Active/PaymentReceived
5. **Rotate if needed** - If an account is accidentally swept, create replacement
6. **Maintain minimum balance** - Ensure fee-payer identities remain funded above thresholds in `testnet/monitoring/balance-thresholds.md`

## Usage in Documentation

Reference these accounts in:
- CLI examples in `docs/api-reference.md`
- Screenshots in README and docs
- Integration test fixtures in `bridgelet-sdk`
- Walkthrough guides

Example reference:
> "See known account `GABC...` (created 2026-09-24, expires 2036) for a live `PaymentReceived` state example."

## Related Documentation

- `testnet/registry/contract-status.md` - Contract deployment status
- `testnet/registry/verify-contract-live.md` - Health check procedures
- `docs/api-reference.md` - CLI examples using known accounts
- `testnet/monitoring/balance-thresholds.md` - Minimum balance thresholds and Friendbot re-funding triggers

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-24 | Initial known accounts registry template |

---

## Long-lived account nomination form and invariants

The rows below are templates only. Every value is a `PLACEHOLDER`; copy a row when nominating a real account and replace each placeholder with verified values. Do not treat any placeholder as a live address or contract ID.

| Label | Contract ID | Ephemeral account address | expiry_ledger | Purpose | Last checked |
|---|---|---|---|---|---|
| PLACEHOLDER (e.g. `demo-screenshots-01`) | PLACEHOLDER (`C...`) | PLACEHOLDER (`G...` account address) | PLACEHOLDER (far-future ledger) | PLACEHOLDER (e.g. screenshots) | PLACEHOLDER (YYYY-MM-DD) |
| PLACEHOLDER (e.g. `docs-walkthrough-01`) | PLACEHOLDER (`C...`) | PLACEHOLDER (`G...` account address) | PLACEHOLDER (far-future ledger) | PLACEHOLDER (e.g. CLI examples) | PLACEHOLDER (YYYY-MM-DD) |

### Invariants for every nominated account

- **Far-future expiry**: set `expiry_ledger` at least one year out, preferably 10+ years (~63M ledgers), so the account does not lapse during its documentation lifetime.
- **Never swept**: these accounts must never be swept or have their reserve reclaimed. Do not route them through `SweepController::execute_sweep()` / `claim()` or any sweep runbook.
- **Labeled non-production**: in every doc, screenshot, or fixture that references one, mark it explicitly as a non-production, test-only account so it is never mistaken for a real user account.
- **Funded above minimum threshold**: maintain fee-payer balance above the re-funding trigger documented in `testnet/monitoring/balance-thresholds.md` so demonstrations and CI fixtures never fail mid-presentation due to gas exhaustion.

### Quarterly check procedure

Run at least once per quarter, and after any testnet reset:

1. For each row, confirm the contract ID resolves by invoking a read-only method against it (see `testnet/registry/verify-contract-live.md`). If the ID does not resolve, the account was lost in a testnet reset; redeploy and re-nominate it.
2. Confirm the account has not been swept: invoke its status/read methods and verify it is still `Active` or `PaymentReceived`, not `Swept`.
3. Confirm `expiry_ledger` is still in the future and comfortably beyond the one-year minimum; extend or replace the account if it is approaching expiry.
4. Update the `Last checked` cell with the current `YYYY-MM-DD` date.
5. If any check fails, mark the row as retired, create a replacement per the Creation Procedure above, and update every doc/screenshot that referenced the old account.
