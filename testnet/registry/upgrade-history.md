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