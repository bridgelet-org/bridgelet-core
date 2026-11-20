# Testnet WASM Hash Reference

## Overview

Alongside contract IDs, this document records the WASM hash each deployed testnet contract currently points to, so a maintainer can confirm "is testnet actually running the code in this exact commit" without needing access to the deploy pipeline's own artifacts.

## Current WASM Hashes

> **Note**: As of 2026-09-24, no Bridgelet contracts are deployed on testnet. This table will be populated once contracts are deployed (see `testnet/registry/contract-status.md`).

| Contract | Contract ID | WASM Hash | Commit SHA | Deploy Date | Deployed By |
|---|---|---|---|---|---|
| EphemeralAccount | TBD | TBD | TBD | TBD | TBD |
| SweepController | TBD | TBD | TBD | TBD | TBD |
| ReserveContract | TBD | TBD | TBD | TBD | TBD |
| AccountFactory | TBD | TBD | TBD | TBD | TBD |

---

## How to Verify WASM Hash

### Method 1: From Deploy Output (Preferred)

When deploying, capture the WASM hash:

```bash
# Upload WASM and get hash
WASM_HASH=$(stellar contract upload \
  --wasm target/wasm32-unknown-unknown/release/ephemeral_account.wasm \
  --network testnet \
  --source testnet-deployer)

# Record: EphemeralAccount | <CONTRACT_ID> | $WASM_HASH | <COMMIT_SHA> | <DATE> | <DEPLOYER>
```

### Method 2: From Contract Code (If Contract ID Known)

```bash
# Get contract code hash via RPC
# Note: Requires Soroban RPC with getContractCode or similar
# This may not be directly available via stellar-cli

# Alternative: Check deployments/testnet.json if deploy script records it
cat deployments/testnet.json
```

### Method 3: Local Build Verification

```bash
# Build locally at specific commit
git checkout <COMMIT_SHA>
cd contracts/ephemeral_account
cargo build --target wasm32-unknown-unknown --release

# Compute WASM hash
WASM_HASH=$(stellar contract upload \
  --wasm target/wasm32-unknown-unknown/release/ephemeral_account.wasm \
  --network testnet \
  --source testnet-deployer \
  --dry-run 2>&1 | grep -oE 'wasm hash: [a-f0-9]+' | cut -d' ' -f3)

# Compare with recorded hash
echo "Local:  $WASM_HASH"
echo "Recorded: <RECORDED_HASH>"
```

---

## WASM Hash Format

- **Length**: 32 bytes (64 hex characters)
- **Algorithm**: SHA-256 of WASM binary
- **Encoding**: Lowercase hex
- **Example**: `a1b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef123456`

---

## Verification Workflow

### For Maintainers

```bash
#!/bin/bash
# verify-wasm.sh - Verify testnet contracts match expected commits

CONTRACTS=(
  "ephemeral_account:<EPHEMERAL_ID>:<EXPECTED_HASH>"
  "sweep_controller:<SWEEP_ID>:<EXPECTED_HASH>"
  # "reserve_contract:<RESERVE_ID>:<EXPECTED_HASH>"
  # "account_factory:<FACTORY_ID>:<EXPECTED_HASH>"
)

for entry in "${CONTRACTS[@]}"; do
  IFS=':' read -r name id expected_hash <<< "$entry"
  echo "Checking $name ($id)..."
  
  # Get actual hash from RPC (if supported)
  # actual_hash=$(get_contract_code_hash $id)
  
  # For now, manual comparison:
  echo "  Expected: $expected_hash"
  echo "  Actual:   [MANUAL CHECK NEEDED]"
done
```

### For CI/CD Integration

Add to deploy pipeline:
```yaml
# .github/workflows/deploy-testnet.yml
- name: Record WASM hashes
  run: |
    echo "EPHEMERAL_WASM_HASH=$EPHEMERAL_WASM_HASH" >> $GITHUB_ENV
    echo "SWEEP_WASM_HASH=$SWEEP_WASM_HASH" >> $GITHUB_ENV
    # Update wasm-hash-reference.md via bot or artifact
```

---

## Commit-to-Hash Mapping

| Commit SHA | Date | EphemeralAccount | SweepController | ReserveContract | AccountFactory |
|---|---|---|---|---|---|
| TBD | TBD | TBD | TBD | TBD | TBD |

---

## Automated Staleness Check

Pair with `testnet/registry/last-verified.json` - if WASM hash in this doc doesn't match what's on-chain, flag for investigation.

```bash
# Check if recorded hash matches on-chain (future RPC support)
# If mismatch: CONTRACT UPGRADED OR DEPLOY DRIFT DETECTED
```

---

## Related Documentation

- `testnet/registry/contract-status.md` - Contract deployment status
- `testnet/registry/last-verified.json` - Machine-readable timestamps
- `testnet/registry/upgrade-history.md` - Upgrade log (WASM hash changes)
- `testnet/registry/verify-contract-live.md` - Health checks

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-24 | Initial WASM hash reference template |

---

## Recorded WASM hashes snapshot

The table below snapshots the WASM hash recorded in the committed `deployments/testnet.json`, alongside the contract ID from the same record. This lets a maintainer confirm that testnet runs the code in this exact commit without access to deploy-pipeline artifacts. A hash is filled in only where the deployment record actually contains one.

| Contract | Contract ID (from `deployments/testnet.json`) | Recorded WASM hash |
|---|---|---|
| EphemeralAccount | `CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3` | `5e667ea0687341bdccc81538492143b573777dd04b2450cdeb89b02cee62c58e` |
| SweepController | `CAJB4RRM5JRPXLITC5JUA7Q3LU2WQJDRPXVKMORXWHJJBPZRYWDJ5V4S` | not available in the deployment record |
| ReserveContract | `CBG3Y4GWFTL4D37JGV2XWAWC7EUQ7XGH4XM3SFLQYYSKPB2IYMAS54HY` | not available in the deployment record |
| AccountFactory | `CDBY4L3NTLW4OBZZTY73VL3NNWO2AGFFGBRJVDL3IMKDDLFF2W57WK6H` | not available in the deployment record |

> The presence of an ID in `deployments/testnet.json` does not by itself confirm a live deployment. Per the main README, the deploy script deploys only `ephemeral_account` and `sweep_controller`; `reserve_contract` and `account_factory` are not deployed by it. Verify any ID on-chain before relying on it.

To read a contract's current on-chain code hash and compare it with the recorded value:

```bash
RECORDED_HASH=5e667ea0687341bdccc81538492143b573777dd04b2450cdeb89b02cee62c58e
ONCHAIN_HASH=$(stellar contract info hash --id CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3 --network testnet)

echo "Recorded: $RECORDED_HASH"
echo "On-chain: $ONCHAIN_HASH"
[ "$RECORDED_HASH" = "$ONCHAIN_HASH" ] && echo "MATCH" || echo "MISMATCH (deploy drift or upgrade)"
```

For another contract, replace the `--id` value with that contract's ID and set `RECORDED_HASH` to its recorded value (or note that none is available in the deployment record). To compare against this commit's own binary, compute the local hash with `stellar contract info hash --wasm target/wasm32-unknown-unknown/release/ephemeral_account.wasm` and compare it with `$ONCHAIN_HASH`.
