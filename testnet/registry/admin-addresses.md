# Testnet Contract Admin/Controller Addresses

## Overview

This document records the public addresses (not secret keys) currently configured as `admin`/`authorized_controller` on live testnet `EphemeralAccount`/`SweepController` instances. Sourced by querying the contracts directly, not from deploy scripts/secrets.

## Current Testnet Deployment

> **Note**: As of 2026-09-24, no Bridgelet contracts are deployed on testnet. This document will be populated once contracts are deployed (see `testnet/registry/contract-status.md`).

---

## EphemeralAccount Admin/Controller Addresses

### Per-Instance Roles

Each `EphemeralAccount` instance has three privileged addresses set at `initialize()`:

| Role | Parameter | Purpose | Query Method |
|---|---|---|---|
| **Creator** | `creator` | Account creator; authorizes `initialize()` | `get_info().creator` |
| **Authorized Controller** | `authorized_controller` | Address allowed to call `sweep()`/`sweep_claim()`; typically `SweepController` contract ID | `get_info()` → not directly exposed, but inferred from sweep auth |
| **Admin** | `admin` | Address allowed to call `upgrade(new_wasm_hash)` | Not directly exposed via getter; inferred from upgrade auth |

### Querying Live Addresses

```bash
# Get creator from initialized account
stellar contract invoke \
  --id <EPHEMERAL_ACCOUNT_ID> \
  --network testnet \
  --source testnet-investigator \
  -- \
  get_info

# Output includes: creator, authorized_controller (inferred), admin (inferred)
```

### Expected Values (Post-Deployment)

| Account ID | Creator | Authorized Controller | Admin | Notes |
|---|---|---|---|---|
| TBD | TBD | TBD (SweepController ID) | TBD | Will be populated after deployment |

---

## SweepController Admin/Controller Addresses

### Per-Instance Roles

Each `SweepController` instance has two privileged addresses set at `initialize()`:

| Role | Parameter | Purpose | Query Method |
|---|---|---|---|
| **Creator** | `creator` | Owns controller; authorizes `update_authorized_destination()` | Not directly exposed; inferred from update auth |
| **Authorized Signer** | `authorized_signer` (BytesN<32>) | Ed25519 public key verifying sweep signatures | Not directly exposed; inferred from sig verification |

### Optional Locked Destination

If `authorized_destination` was set at `initialize()`:
- **Authorized Destination**: Only address sweeps can go to
- Query: Not directly exposed; inferred from `update_authorized_destination` failures or `execute_sweep` rejections

### Querying Live Addresses

```bash
# No direct getters for creator/signer in current interface
# Infer from transaction history:
# 1. Check initialize() transaction - creator is tx source
# 2. Check execute_sweep() transactions - signer verified by contract
# 3. Check update_authorized_destination() - creator must authorize
```

### Expected Values (Post-Deployment)

| Controller ID | Creator | Authorized Signer (Ed25519 Pubkey) | Locked Destination | Notes |
|---|---|---|---|---|
| TBD | TBD | TBD | TBD | Will be populated after deployment |

---

## ReserveContract Admin Address

### Per-Instance Roles

| Role | Parameter | Purpose | Query Method |
|---|---|---|---|
| **Admin** | `admin` | Only address allowed to call `set_base_reserve()` | `get_admin()` |

### Querying Live Addresses

```bash
# Get admin address
stellar contract invoke \
  --id <RESERVE_CONTRACT_ID> \
  --network testnet \
  --source testnet-investigator \
  -- \
  get_admin
```

### Expected Values (Post-Deployment)

| ReserveContract ID | Admin | Notes |
|---|---|---|
| TBD | TBD | Not deployed |

---

## AccountFactory Admin/Controller Addresses

### Per-Instance Roles

| Role | Parameter | Purpose | Query Method |
|---|---|---|---|
| **Creator (per-batch)** | `creator` param in `batch_initialize()` | Authorizes batch; becomes `authorized_controller` and `admin` for all created accounts | Inferred from batch_initialize tx source |

### Querying Live Addresses

```bash
# No persistent admin on factory itself
# Each batch has its own creator
```

---

## Verification Checklist

When contracts are deployed, verify and record:

### EphemeralAccount
- [ ] `creator` matches deployer
- [ ] `authorized_controller` = SweepController contract ID
- [ ] `admin` = designated upgrade authority

### SweepController
- [ ] `creator` matches deployer
- [ ] `authorized_signer` = expected Ed25519 pubkey (matches off-chain signer)
- [ ] `authorized_destination` = None (flexible) or expected address (locked)

### ReserveContract
- [ ] `admin` = designated config authority
- [ ] `base_reserve` set to expected value (e.g., 1_000_000_000 stroops)

### AccountFactory
- [ ] Initialized with correct `ephemeral_account_wasm_hash`
- [ ] `batch_initialize` creator matches expected deployer

---

## Security Notes

1. **Never expose secret keys** - This document only records public addresses
2. **Verify on-chain** - Always query contracts directly; deploy scripts/secrets may be stale
3. **Monitor changes** - Track `upgrade()` events (EphemeralAccount) and `set_base_reserve()` (ReserveContract)
4. **Rotation** - Document address rotation procedures for each role

---

## Related Documentation

- `testnet/registry/contract-status.md` - Deployment status table
- `testnet/registry/upgrade-history.md` - Contract upgrade log
- `testnet/registry/verify-contract-live.md` - Health check procedures
- `testnet/config/nonce-tracking.md` - SweepController nonce inspection

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-24 | Initial admin addresses registry template |