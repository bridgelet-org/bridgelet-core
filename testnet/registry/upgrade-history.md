# Testnet Contract Upgrade History

## Overview

`EphemeralAccount` supports an admin-gated `upgrade()` function. Since contracts on testnet may be upgraded in place rather than redeployed under a new ID, this log tracks upgrade events so integrators can correlate behavior changes to specific upgrades rather than assuming a bug.

## Upgrade Log

> **Note**: As of 2026-09-24, no Bridgelet contracts are deployed on testnet. This log will be populated once contracts are deployed and upgraded (see `testnet/registry/contract-status.md`).

### EphemeralAccount Upgrades

| Date | Contract ID | Old WASM Hash | New WASM Hash | Reason | Admin | Tx Hash |
|---|---|---|---|---|---|---|
| TBD | TBD | TBD | TBD | Initial deployment | TBD | TBD |

### SweepController Upgrades

| Date | Contract ID | Old WASM Hash | New WASM Hash | Reason | Admin | Tx Hash |
|---|---|---|---|---|---|---|
| TBD | TBD | TBD | TBD | Initial deployment | TBD | TBD |

### ReserveContract Upgrades

| Date | Contract ID | Old WASM Hash | New WASM Hash | Reason | Admin | Tx Hash |
|---|---|---|---|---|---|---|
| TBD | TBD | TBD | TBD | Initial deployment | TBD | TBD |

### AccountFactory Upgrades

| Date | Contract ID | Old WASM Hash | New WASM Hash | Reason | Admin | Tx Hash |
|---|---|---|---|---|---|---|
| TBD | TBD | TBD | TBD | Initial deployment | TBD | TBD |

---

## Upgrade Procedure

### Prerequisites
1. New WASM built and uploaded to testnet: `stellar contract upload --wasm <WASM_PATH> --network testnet`
2. Admin identity with sufficient XLM: `stellar keys generate --global testnet-admin`
3. Current contract ID and admin address known

### Upgrade Command

```bash
# Upload new WASM
NEW_WASM_HASH=$(stellar contract upload \
  --wasm target/wasm32-unknown-unknown/release/ephemeral_account.wasm \
  --network testnet \
  --source testnet-admin)

# Upgrade contract (admin must authorize)
stellar contract invoke \
  --id <EPHEMERAL_ACCOUNT_ID> \
  --network testnet \
  --source testnet-admin \
  -- \
  upgrade \
  --new_wasm_hash $NEW_WASM_HASH
```

### Recording the Upgrade

After successful upgrade, immediately record in this log:

```markdown
| 2026-09-24 | GABC... | abc123... | def456... | Fixed reserve reclaim bug | GADMIN... | txhash... |
```

### Verification After Upgrade

```bash
# 1. Verify contract still responds
stellar contract invoke --id <ID> --network testnet --source testnet-healthcheck -- get_status

# 2. Verify new behavior (e.g., if bug was fixed)
stellar contract invoke --id <ID> --network testnet --source testnet-healthcheck -- get_info_paginated --params '{"limit": 5, "cursor_index": 4294967295}'

# 3. Check events for upgrade confirmation
# (No native upgrade event emitted; monitor via transaction history)
```

---

## Upgrade Impact Matrix

| Contract | Upgrade Affects | State Preserved | Notes |
|---|---|---|---|
| EphemeralAccount | All functions | Yes (instance storage) | Admin-gated; single instance per account |
| SweepController | All functions | Yes (instance storage) | Creator-gated destination updates; nonce preserved |
| ReserveContract | All functions | Yes (instance storage) | Admin-gated; base_reserve preserved |
| AccountFactory | All functions | Yes (instance storage) | WASM hash updated; existing accounts unaffected |

---

## Integrator Guidance

When a behavior change is observed:

1. **Check this log first** - Correlate date with upgrade
2. **Verify WASM hash** - `stellar contract upload` output vs. this log
3. **Review upgrade reason** - Understand intentional vs. accidental changes
4. **Test against known accounts** - Use `known-test-accounts.md` fixtures
5. **Report discrepancies** - If change not in log, may indicate unauthorized upgrade

---

## Monitoring for Upgrades

### Automated Detection (Future)
```bash
# Monitor for upgrade transactions
# Filter for invoke to contract with fn_name = "upgrade"
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
        "contractIds": ["<EPHEMERAL_ACCOUNT_ID>"],
        "topics": []  # No native upgrade event; monitor txs instead
      }]
    }
  }'
```

### Manual Check
```bash
# Get current WASM hash (requires RPC access to contract code)
# Compare with last recorded hash in this log
```

---

## Emergency Rollback

If an upgrade introduces critical bugs:

1. **Build previous known-good WASM**
2. **Upload to testnet** → get new hash
3. **Admin calls `upgrade()` with previous hash**
4. **Record rollback in this log** with reason "Emergency rollback from vX.Y.Z"

> **Note**: Only admin can upgrade. No multi-sig or timelock in current MVP.

---

## Related Documentation

- `testnet/registry/contract-status.md` - Current deployment status
- `testnet/registry/admin-addresses.md` - Admin addresses for upgrade auth
- `testnet/registry/verify-contract-live.md` - Post-upgrade health checks
- `testnet/registry/wasm-hash-reference.md` - WASM hash tracking

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-24 | Initial upgrade history log template |

---

## Upgrade Event Log Template (Issue #984 Addendum)

Contracts on testnet may be upgraded in place rather than redeployed under a new ID, and `EphemeralAccount` exposes an admin-gated `upgrade(new_wasm_hash)`. Use the table below as the ready-to-use log: append one row for every observed or performed upgrade so integrators can correlate a behavior change to a specific upgrade instead of assuming a regression. The rows marked `EXAMPLE` are illustrative formatting samples only — they are **not real upgrade events** and must not be read as history.

| Date | Contract | Contract ID | Old WASM Hash | New WASM Hash | Reason | Commit | Recorded By |
|---|---|---|---|---|---|---|---|
| YYYY-MM-DD | EphemeralAccount | C... | <old 64-hex> | <new 64-hex> | <why the code changed> | <git SHA> | <github login> |
| EXAMPLE 2026-11-01 (ILLUSTRATIVE, NOT REAL) | EphemeralAccount | CB7Z...EXAMPLE | aaaa... | bbbb... | EXAMPLE: "fix reserve reclaim rounding" | EXAMPLE: deadbeef | ustaxs |
| EXAMPLE 2026-11-05 (ILLUSTRATIVE, NOT REAL) | EphemeralAccount | CB7Z...EXAMPLE | bbbb... | cccc... | EXAMPLE: "gas optimization in sweep" | EXAMPLE: cafebabe | ustaxs |

Baseline: `deployments/testnet.json` records `config.ephemeralAccountWasmHash = 5e667ea0687341bdccc81538492143b573777dd04b2450cdeb89b02cee62c58e` as the initially deployed `EphemeralAccount` WASM (deployedAt `2026-10-10T04:38:14Z`). Use this as the "old hash" for the first upgrade you record if no earlier log entry exists.

### Steps to verify an upgrade happened

1. Read the contract's current on-chain code hash from its instance entry and compare it to the recorded `New WASM Hash` (and the baseline above). The instance entry's `executable` field carries the active `wasm_hash`:

```bash
curl -sS https://soroban-testnet.stellar.org \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getLedgerEntries","params":{"keys":["<BASE64_XDR_OF_CONTRACT_INSTANCE_KEY>"]}}'
# Decode `executable.wasm_hash` from the returned instance entry and compare it
# to the Old/New WASM Hash values in the log.
```

2. Compute/compare the hash of the WASM you believe was deployed; `stellar contract upload` prints the hash of an uploaded WASM:

```bash
stellar contract upload \
  --wasm <WASM_PATH> \
  --network testnet \
  --source testnet-admin
```

3. Correlate with transaction history: `upgrade()` is invoked with `--new_wasm_hash`. Note that **no native upgrade event is emitted**, so `getEvents` topic filters will not surface upgrades; you must inspect `getLedgerEntries` / transaction history (and the `upgrade` invocation arguments) rather than events. Record the tx hash and admin address alongside the WASM hashes.