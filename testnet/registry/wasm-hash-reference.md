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