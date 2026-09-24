# AccountFactory Testnet Status

## Overview

Since `account_factory` is "real but entirely undocumented and undeployed" per the main README, this document tracks exactly what would need to happen (built by `scripts/build.sh`, deployed, tested in CI) before it could appear in `testnet/registry/contract-status.md` as deployed — without this batch actually performing any of those excluded-folder changes itself.

## Current Status

| Aspect | Status | Details |
|---|---|---|
| **Deployed on Testnet** | ❌ No | Not in deploy scripts |
| **Built by Build Script** | ❌ No | `scripts/build.sh` only builds `ephemeral_account`, `sweep_controller`, `reserve_contract` |
| **Tested in CI** | ❌ No | `.github/workflows/test.yml` fully commented; `deploy-testnet.yml` excludes it |
| **WASM Hash** | Build manually | `cd contracts/account_factory && cargo build --target wasm32-unknown-unknown --release` |
| **Documentation** | ❌ None | Not in README, api-reference.md, or architecture.md |

## Required Steps for Testnet Deployment

### 1. Add to Build Script (`scripts/build.sh`)

```bash
# Current builds:
cargo build --target wasm32-unknown-unknown --release -p ephemeral_account
cargo build --target wasm32-unknown-unknown --release -p sweep_controller
cargo build --target wasm32-unknown-unknown --release -p reserve_contract

# ADD:
cargo build --target wasm32-unknown-unknown --release -p account_factory
```

### 2. Add to CI Workflows

#### `.github/workflows/test.yml` (uncomment and add)
```yaml
# Add to matrix or steps:
- cargo test -p account_factory
- cargo fmt --check -p account_factory
- cargo clippy -p account_factory
```

#### `.github/workflows/deploy-testnet.yml` (uncomment and add)
```yaml
# Add to build step:
- cargo build --target wasm32-unknown-unknown --release -p account_factory
```

### 3. Add to Deploy Script (`scripts/deploy-testnet.sh`)

```bash
# Build AccountFactory (if not in build.sh)
cargo build --target wasm32-unknown-unknown --release -p account_factory

# Deploy AccountFactory
ACCOUNT_FACTORY_ID=$(stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/account_factory.wasm \
  --network testnet \
  --source $SIGNER_SECRET_KEY)

# Initialize with EphemeralAccount WASM hash
EPHEMERAL_WASM_HASH=$(stellar contract upload \
  --wasm target/wasm32-unknown-unknown/release/ephemeral_account.wasm \
  --network testnet \
  --source $SIGNER_SECRET_KEY)

stellar contract invoke \
  --id $ACCOUNT_FACTORY_ID \
  --network testnet \
  --source $SIGNER_SECRET_KEY \
  -- \
  initialize \
  --ephemeral_account_wasm_hash $EPHEMERAL_WASM_HASH
```

### 4. Update Deployments Artifact

Add to `deployments/testnet.json`:
```json
{
  "ephemeral_account": "...",
  "sweep_controller": "...",
  "reserve_contract": "...",
  "account_factory": "..."
}
```

## Contract Interface (Current)

```rust
// AccountFactory
fn initialize(env: Env, ephemeral_account_wasm_hash: BytesN<32>);
fn batch_initialize(
    env: Env,
    creator: Address,
    requests: Vec<AccountInitRequest>,
) -> Vec<AccountInitResult>;
fn batch_initialize_paginated(
    env: Env,
    creator: Address,
    requests: Vec<AccountInitRequest>,
    params: PaginationParams,
) -> PaginatedAccountInitResultResponse;
fn batch_initialize_count(env: Env, requests: Vec<AccountInitRequest>) -> u32;
```

### Known Gaps (Not Fixed by This Batch)

1. **Error detail dropped**: `AccountInitResult.error` is always `None` on failure
2. **No event emission**: Factory emits no events for account creation
3. **Salt derivation**: Uses simple index-based salt (0, 1, 2...); not cryptographically random
4. **No access control**: `initialize()` has no auth; anyone can set WASM hash
5. **Fixed controller/admin**: All created accounts get `authorized_controller = creator` and `admin = creator`

## Verification Commands (Post-Deployment)

```bash
# Check factory initialized with correct WASM hash
# (No direct getter; verify by deploying test account)

# Test batch_initialize_count
stellar contract invoke \
  --id $ACCOUNT_FACTORY_ID \
  --network testnet \
  --source testnet-healthcheck \
  -- \
  batch_initialize_count \
  --requests '[{"expiry_ledger": 999999999, "recovery_address": "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"}]'

# Test paginated batch_initialize (dry-run with small limit)
stellar contract invoke \
  --id $ACCOUNT_FACTORY_ID \
  --network testnet \
  --source testnet-deployer \
  -- \
  batch_initialize_paginated \
  --creator <TEST_CREATOR> \
  --requests '[...]' \
  --params '{"limit": 1, "cursor_index": 4294967295}'
```

## Testing Requirements

Before considering AccountFactory "testnet-ready":

- [ ] Unit tests pass (`cargo test -p account_factory`)
- [ ] Integration tests added (deploy + batch_initialize + verify accounts)
- [ ] Pagination tested (multi-page batch_initialize_paginated)
- [ ] Error handling verified (duplicate init, invalid expiry, etc.)
- [ ] Gas costs measured for various batch sizes
- [ ] Salt collision resistance verified

## Related Documentation

- `testnet/registry/contract-status.md` - Overall contract status table
- `testnet/registry/wasm-hash-reference.md` - WASM hash tracking
- `testnet/registry/verify-contract-live.md` - Health check procedures
- `testnet/runbooks/account-factory-batch-failure.md` - Failure investigation runbook
- `docs/api-reference.md` - API reference (needs AccountFactory section)
- `docs/architecture.md` - Architecture doc (has AccountFactory section)

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-24 | Initial AccountFactory status tracking |