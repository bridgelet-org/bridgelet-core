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

---

## Known non-sensitive addresses from the deployment record

The values below are recorded in `deployments/testnet.json` (`config`) and are all **public, non-secret** identifiers: Stellar account/contract addresses (`G...`) and one raw Ed25519 **public** key in hex. No secret (`S...`) keys are recorded here or anywhere in this registry. These are the values in the committed deployment record, but IDs and roles can change every time `scripts/deploy-testnet.sh` is re-run and after testnet resets, so **each value must be confirmed on-chain before it is trusted** using the commands that follow.

| Config key (deployments/testnet.json) | Value | Public / secret | Role on live testnet (from deployment record) |
|---|---|---|---|
| `creatorAddress` | `GCKGORHRRXLVWDVPR565V6JTMYP43TGH4KH5X7REVW7AL4LMZZLF4PAA` | Public / non-secret | Creator used to initialize `SweepController`; also the `creator` value passed at `EphemeralAccount` initialization |
| `reserveAdminAddress` | `GCKGORHRRXLVWDVPR565V6JTMYP43TGH4KH5X7REVW7AL4LMZZLF4PAA` | Public / non-secret | Admin recorded for `ReserveContract`; same value as `creatorAddress`. `ReserveContract` is not part of the supported testnet deployment, so this is a record-only value |
| `recoveryAddress` | `GAEG3HJONLNMHB6L37YX5QHHBYWAKDQZ4GV7UPOWUPI6TOKICHG2PRE6` | Public / non-secret | `recovery_address` set per account at `EphemeralAccount::initialize()` (expiry fallback) |
| `authorizedSigner` | `16ac79d642e33ac696e822cc7175ad7ddd5587a9dd8b6942640cbb832a1ab4bf` | Public / non-secret | Raw 32-byte Ed25519 **public** key (hex) verified by `SweepController::verify_sweep_auth()`; this is not a `G...` strkey and not a private key |

### Read commands to confirm each value on-chain

```bash
# 0. Read the recorded values from the deployment record itself.
jq '.config' deployments/testnet.json

# 1. EphemeralAccount creator + recovery address: both are returned by get_info().
stellar contract invoke \
  --id <EPHEMERAL_ACCOUNT_ID> \
  --network testnet \
  --source testnet-investigator \
  -- \
  get_info
# Compare the `creator` and `recovery_address` fields of the result to
# creatorAddress and recoveryAddress above. Example ID from the deployment record:
#   EPHEMERAL_ACCOUNT_ID=CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3

# 2. SweepController creator (DataKey::Creator) and authorized signer
#    (DataKey::AuthorizedSigner) have NO public getter in the current interface.
#    Confirm them by decoding the contract's instance entry via RPC: fetch the
#    instance entry (key = LedgerKey::ContractData with the LedgerKeyContractInstance
#    key), then read the AuthorizedSigner and Creator entries from its storage map.
curl -sS https://soroban-testnet.stellar.org \
  -H 'Content-Type: application/json' \
  -d '{
    "jsonrpc": "2.0", "id": 1, "method": "getLedgerEntries",
    "params": { "keys": ["<BASE64_XDR_OF_CONTRACT_INSTANCE_KEY>"] }
  }'
# Example controller ID from the deployment record:
#   SWEEP_CONTROLLER_ID=CAJB4RRM5JRPXLITC5JUA7Q3LU2WQJDRPXVKMORXWHJJBPZRYWDJ5V4S
# The AuthorizedSigner field is a BytesN<32>; compare its hex to authorizedSigner above.

# 3. SweepController creator can also be cross-checked from the initialize()
#    transaction in transaction history: the creator is the tx source that
#    satisfied creator.require_auth().

# 4. reserveAdminAddress: ReserveContract is NOT deployed/unwired on testnet
#    (see README and testnet/docs/overview.md), so there is no live contract to
#    query. Treat this entry as a record-only value until that contract is deployed.
```

All commands above are read-only. Never paste a secret (`S...`) key into this file or into any command that writes state.