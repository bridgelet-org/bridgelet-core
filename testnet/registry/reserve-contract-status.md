# ReserveContract Testnet Status

## Overview

This document explicitly tracks `reserve_contract`'s testnet (non-)status, noting both that it isn't deployed AND that even if deployed, nothing in `EphemeralAccount` currently reads its value — two separate gaps that shouldn't be conflated when someone eventually decides to close them.

## Current Status

| Aspect | Status | Details |
|---|---|---|
| **Deployed on Testnet** | ❌ No | Not included in `scripts/deploy-testnet.sh` |
| **Built by Build Script** | ✅ Yes | `scripts/build.sh` includes `reserve_contract` |
| **WASM Hash** | Available | Build locally: `cd contracts/reserve_contract && cargo build --target wasm32-unknown-unknown --release` |
| **EphemeralAccount Integration** | ❌ No wiring | EphemeralAccount uses hardcoded `BASE_RESERVE_STROOPS = 1_000_000_000` |
| **SweepController Integration** | ❌ No wiring | Not applicable |
| **AccountFactory Integration** | ❌ No wiring | Not applicable |

## Two Separate Gaps

### Gap 1: No Testnet Deployment
- `scripts/deploy-testnet.sh` deploys only `ephemeral_account` and `sweep_controller`
- `RESERVE_CONTRACT_ID` variable is referenced but never assigned
- With `set -euo pipefail`, script aborts with "unbound variable" error
- **Fix**: Add ReserveContract deployment to deploy script

### Gap 2: No Cross-Contract Wiring
- `EphemeralAccount` maintains internal reserve tracking:
  - `BASE_RESERVE_STROOPS = 1_000_000_000` (1 XLM)
  - `get_reserve_remaining()`, `get_reserve_available()`, `reclaim_reserve()`
- `ReserveContract` independently stores admin-settable base reserve:
  - `set_base_reserve()`, `get_base_reserve()`, `require_base_reserve()`
- **No contract calls `ReserveContract` on-chain**
- **Fix**: Modify `EphemeralAccount` to call `ReserveContract.require_base_reserve()` instead of using constant

## Contract Interface (Current)

```rust
// ReserveContract (deployed independently)
fn initialize(env: Env, admin: Address) -> Result<(), Error>;
fn set_base_reserve(env: Env, amount: i128) -> Result<(), Error>;  // admin-gated, max 100_000_000_000 stroops
fn get_base_reserve(env: Env) -> Option<i128>;
fn require_base_reserve(env: Env) -> Result<i128, Error>;  // Errors if not set
fn has_base_reserve(env: Env) -> bool;
fn get_admin(env: Env) -> Option<Address>;

// EphemeralAccount (internal reserve tracking)
const BASE_RESERVE_STROOPS: i128 = 1_000_000_000;  // Hardcoded!
fn get_reserve_remaining(env: Env) -> i128;
fn get_reserve_available(env: Env) -> i128;
fn is_reserve_reclaimed(env: Env) -> bool;
fn reclaim_reserve(env: Env) -> Result<i128, Error>;
fn reclaim_reserve_to(env: &Env, destination: &Address, sweep_id: u64) -> Result<i128, Error>;
```

## Proposed Integration Path

### Option A: EphemeralAccount Reads from ReserveContract
```rust
// In EphemeralAccount::reclaim_reserve_to()
let reserve_contract = ReserveContractClient::new(env, reserve_contract_address);
let base_reserve = reserve_contract.require_base_reserve()?;  // Instead of constant
// ... rest of logic uses base_reserve
```
**Requires**: ReserveContract deployed first, address passed to EphemeralAccount at initialize

### Option B: ReserveContract as Config Only (Off-chain)
- Keep EphemeralAccount internal tracking
- Use ReserveContract as off-chain reference for SDKs
- **Pros**: No cross-contract call overhead
- **Cons**: Two sources of truth; drift possible

### Option C: Deprecate ReserveContract
- Remove ReserveContract entirely
- EphemeralAccount's internal tracking is sufficient
- **Pros**: Simpler architecture
- **Cons**: Loses admin-configurable reserve for network changes

## Deployment Requirements

To deploy ReserveContract to testnet:

1. **Add to `scripts/build.sh`** (already present)
2. **Add to `scripts/deploy-testnet.sh`**:
   ```bash
   # Deploy ReserveContract
   RESERVE_CONTRACT_ID=$(stellar contract deploy \
     --wasm target/wasm32-unknown-unknown/release/reserve_contract.wasm \
     --network testnet \
     --source $SIGNER_SECRET_KEY)
   
   # Initialize with admin
   stellar contract invoke \
     --id $RESERVE_CONTRACT_ID \
     --network testnet \
     --source $SIGNER_SECRET_KEY \
     -- \
     initialize \
     --admin $ADMIN_ADDRESS
   
   # Set base reserve (1 XLM = 1_000_000_000 stroops)
   stellar contract invoke \
     --id $RESERVE_CONTRACT_ID \
     --network testnet \
     --source $ADMIN_SECRET_KEY \
     -- \
     set_base_reserve \
     --amount 1000000000
   ```
3. **Update `deployments/testnet.json`** to include `RESERVE_CONTRACT_ID`
4. **Verify** with `verify-contract-live.md` procedure

## Verification Commands (Post-Deployment)

```bash
# Check admin
stellar contract invoke --id $RESERVE_CONTRACT_ID --network testnet --source testnet-healthcheck -- get_admin

# Check base reserve
stellar contract invoke --id $RESERVE_CONTRACT_ID --network testnet --source testnet-healthcheck -- get_base_reserve

# Check has_base_reserve
stellar contract invoke --id $RESERVE_CONTRACT_ID --network testnet --source testnet-healthcheck -- has_base_reserve
```

## Related Documentation

- `testnet/registry/contract-status.md` - Overall contract status table
- `testnet/registry/wasm-hash-reference.md` - WASM hash tracking
- `testnet/registry/verify-contract-live.md` - Health check procedures
- `docs/fee-model.md` - Fee payer documentation (reserve reclaim costs)
- `docs/architecture.md` - System architecture (shows disconnected ReserveContract)

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-24 | Initial ReserveContract status tracking |