 Bridgelet Core

**Soroban smart contracts for ephemeral account restrictions**

**Status:** Active Development

## Overview

Bridgelet Core contains the Soroban smart contracts that enforce single-use restrictions on ephemeral Stellar accounts and manage the sweep logic for transferring funds to permanent wallets.

## MVP Status

### Current Stub Inventory

| Function | Contract | Stub Status | Production Requirement | Tracking Issue |
|----------|----------|-------------|------------------------|----------------|
| `verify_sweep_authorization` | EphemeralAccount | **Partial** - Uses `require_auth()` instead of Ed25519 signature verification | Implement `env.crypto().ed25519_verify()` against stored `authorized_signer` with signature covering destination + nonce + contract_id | #86 |
| Token transfers | SweepController | **Implemented** - `execute_transfers()` calls `token.transfer()` for all assets | Already implemented in `transfers.rs` | N/A |

### Implementation Notes

- **EphemeralAccount::sweep()**: Currently uses Soroban's `require_auth()` for authorization instead of cryptographic Ed25519 signature verification. The signature parameters (`destination`, `auth_signature`) are accepted but not cryptographically verified. Production implementation should use `env.crypto().ed25519_verify()` similar to SweepController's implementation.
- **SweepController::claim()**: Experimental gas-free claim path. The recipient signs a Soroban auth entry for `claim(recipient, ephemeral_account)`, and a relayer/SDK can submit the transaction and pay fees. Internally the controller uses `authorize_as_current_contract()` so the downstream `EphemeralAccount::sweep()` call can satisfy `authorized_controller.require_auth()`.
- **SweepController::execute_transfers()**: Token transfer logic is fully implemented using SEP-41 token contracts. All recorded payments are transferred atomically to the destination.
- **Security guidance**: Always route sweeps through `SweepController` for proper Ed25519 signature verification. Do not call `EphemeralAccount::sweep()` directly until the signature verification stub is replaced.

## Tech Stack

- **Language:** Rust
- **Framework:** Soroban SDK 22.0.0
- **Testing:** soroban-cli + Rust test framework
- **Build:** Cargo + stellar-cli

## Contracts

### 1. `ephemeral_account`
- Single inbound payment enforcement (multi-asset supported, one payment per asset)
- Controller-gated sweep + gas-free `sweep_claim` path
- Time-based expiration (`expiry_ledger`) with recovery-address fallback
- Internal base-reserve reclaim bookkeeping
- Event emission for auditability
- Upgradeable via `upgrade()` (admin-gated)

### 2. `sweep_controller`
- Real Ed25519 signature verification with nonce replay protection
- Optional "locked" mode restricting sweeps to one pre-authorized destination
- Executes atomic multi-asset SEP-41 token transfers
- Gas-free `claim()` path for recipient-signed, relayer-submitted sweeps

### 3. `reserve_contract`
- Admin-set base reserve amount (bounded to 10,000 XLM / 100,000,000,000 stroops)
- Simple init/get/set/has interface - no integration wiring into `ephemeral_account` yet (see note above)

### 4. `account_factory`
- Batch-deploys N `ephemeral_account` instances from a stored WASM hash in a single transaction
- Per-account init failures are caught but the specific error is currently discarded

## Project Structure

```
contracts/
├── ephemeral_account/
│   ├── src/
│   │   ├── lib.rs           # Main contract (initialize, record_payment, sweep, sweep_claim, expire, upgrade...)
│   │   ├── storage.rs       # State management
│   │   ├── events.rs        # Event definitions
│   │   ├── errors.rs        # Error types
│   │   └── test.rs          # Unit tests
│   └── Cargo.toml
├── sweep_controller/
│   ├── src/
│   │   ├── lib.rs
│   │   ├── authorization.rs # Ed25519 verification + nonce logic
│   │   ├── transfers.rs     # SEP-41 token transfer execution
│   │   ├── storage.rs
│   │   └── errors.rs
│   ├── tests/
│   │   └── integration.rs
│   └── Cargo.toml
├── reserve_contract/
│   ├── src/
│   │   ├── lib.rs
│   │   ├── storage.rs
│   │   ├── events.rs
│   │   ├── errors.rs
│   │   └── test.rs
│   └── Cargo.toml
├── account_factory/
│   ├── src/
│   │   ├── lib.rs
│   │   └── test.rs
│   └── Cargo.toml
└── shared/
    └── src/
        ├── lib.rs
        └── types.rs          # Payment, AccountStatus, AccountInfo, AccountInitRequest/Result
```

## Prerequisites
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Stellar CLI
cargo install --locked stellar-cli --version 23.4.1

# Add wasm target
rustup target add wasm32-unknown-unknown

# Install Binaryen (for WASM optimization)
# Minimum required version: 100
# macOS:
brew install binaryen
# Ubuntu/Debian:
apt-get install binaryen
# Or download from: https://github.com/WebAssembly/binaryen/releases
```

## Build & Deploy

```bash
# Build contracts (with WASM optimization if binaryen is installed)
./scripts/build.sh

# The build script automatically optimizes WASM files using wasm-opt -O3
# if Binaryen is installed. This typically reduces binary size by 15-30%.
# If wasm-opt is not found, the build continues without optimization.

# Run tests
cargo test

# Set .env values with:
$ set -a
source .env
set +a

# then deploy

./scripts/deploy-testnet.sh
```

**Fix needed before this script is production-usable:** deploy `reserve_contract` (and `account_factory`, if you intend to use it) and either set `RESERVE_CONTRACT_ID` from that deployment or remove the references to it.

## Testing

```bash
# scripts/test.sh currently only runs `cargo test` for ephemeral_account.
# It does NOT run sweep_controller, reserve_contract, or account_factory tests.
./scripts/test.sh

# To actually cover everything that has tests:
for c in ephemeral_account sweep_controller reserve_contract account_factory; do
  (cd contracts/$c && cargo test)
done

# sweep_controller's integration tests live under tests/, not src/, and need the
# --test flag explicitly:
(cd contracts/sweep_controller && cargo test --test integration)
```

There is no `scripts/test-local.sh` in this repo - earlier README drafts referenced it, but the actual local-testing entrypoint is `scripts/test.sh` (unit tests only; no local sandbox deployment).

## CI/CD

**Currently disabled - see [CI/CD - currently disabled](#cicd--currently-disabled) above.** The workflow files exist but their bodies are commented out (test.yml fully; deploy-testnet.yml's actual deploy steps). Treat any prior documentation claiming automated testnet deployment on merge as aspirational, not current behavior.

#### Required GitHub Secrets (once workflows are re-enabled)
- `TESTNET_DEPLOYER_SECRET_KEY`: Stellar testnet deployer secret key (`S...` format)

## CI/CD

### Automated Testing
- **Test Workflow** (`.github/workflows/test.yml`): Runs on every push to `main`/`develop` and on PRs to `main`
  - Runs cargo tests for all contracts
  - Checks code formatting with `cargo fmt`
  - Runs clippy for linting
  - Builds all contracts for wasm32-unknown-unknown target
  - Uploads WASM artifacts for deployment

### Automated Testnet Deployment
- **Deploy Workflow** (`.github/workflows/deploy-testnet.yml`): Automatically deploys to Stellar Testnet on merge to `main`
  - Runs tests, format checks, clippy, and builds before deployment
  - Deploys all three contracts: `ephemeral_account`, `sweep_controller`, `reserve_contract`
  - Stores contract IDs as CI artifacts (90-day retention)
  - Posts deployment summary with contract IDs to GitHub Actions summary
  - Can also be triggered manually via `workflow_dispatch`

#### Required GitHub Secrets
To enable automated deployments, add the following secret to your GitHub repository:
- `TESTNET_DEPLOYER_SECRET_KEY`: Stellar testnet deployer secret key (S... format)

#### Manual Deployment
To trigger a manual deployment:
1. Go to Actions tab in GitHub
2. Select "Deploy to Testnet" workflow
3. Click "Run workflow"
4. Optionally provide a reason for the deployment

## Contract Interfaces

These interfaces are published as real Rust traits in
[`contracts/shared/src/interfaces.rs`](contracts/shared/src/interfaces.rs)
(`EphemeralAccountInterface`, `SweepControllerInterface`). Each contract
implements the matching trait, so the interface stays in sync with the
implementation at compile time. The error type is an associated type, letting
each contract keep its own error enum.

### EphemeralAccount (actual signatures)
```rust
pub trait EphemeralAccountInterface {
    fn initialize(
        env: Env,
        creator: Address,
        expiry_ledger: u32,
        recovery_address: Address,
        authorized_controller: Address,  // <- not in earlier README drafts
        admin: Address,                  // <- not in earlier README drafts
    ) -> Result<(), Error>;

    fn record_payment(env: Env, amount: i128, asset: Address) -> Result<(), Error>;

    // NOTE: auth_signature is accepted but NOT cryptographically verified here.
    fn sweep(env: Env, destination: Address, auth_signature: BytesN<64>) -> Result<(), Error>;

    // Gas-free path used by SweepController::claim(); no signature param.
    fn sweep_claim(env: Env, destination: Address) -> Result<(), Error>;

    fn is_expired(env: Env) -> bool;
}
```

### SweepController (actual signatures)
```rust
pub trait SweepControllerInterface {
    fn initialize(
        env: Env,
        creator: Address,
        authorized_signer: BytesN<32>,
        authorized_destination: Option<Address>,
    ) -> Result<(), Error>;

    fn execute_sweep(
        env: Env,
        ephemeral_account: Address,
        destination: Address,
        auth_signature: BytesN<64>,
    ) -> Result<(), Error>;

    fn claim(env: Env, recipient: Address, ephemeral_account: Address) -> Result<(), Error>;

    fn can_sweep(env: Env, ephemeral_account: Address) -> bool;

    fn update_authorized_destination(env: Env, new_destination: Address) -> Result<(), Error>;
}
```

## Events

```rust
AccountCreated { creator, expiry_ledger }
PaymentReceived { amount, asset }
MultiPaymentReceived { ... }
SweepExecutedMulti { destination, payments }
AccountExpired { recovery_address, total_amount, reserve_amount }
ReserveReclaimed { destination, amount, sweep_id, fully_reclaimed, remaining_reserve }
SweepCompleted { ephemeral_account, destination, amount }        # emitted by SweepController
DestinationAuthorized { destination }                            # emitted by SweepController
DestinationUpdated { old_destination, new_destination }          # emitted by SweepController
```

## Security Considerations

- All storage keys use proper namespacing.
- Reentrancy protection via status update before external calls (`Swept` status is set before transfer logic runs).
- Timestamp-based expiration uses ledger sequence, not wall-clock time.
- **Real signature verification only happens inside `SweepController`.** Do not treat `EphemeralAccount::sweep()`'s `auth_signature` parameter as verified - it isn't.
- `AccountFactory::batch_initialize` swallows per-account error detail; a partial-failure batch will tell you *which* address failed but not why - plan monitoring accordingly.

See [Security Model](./docs/security.md), [Signature Format](./docs/SIGNATURE_FORMAT.md), and [Reentrancy Analysis](./docs/reentrancy-analysis.md).

## Documentation

- [Contract Architecture](./docs/architecture.md)
- [API Reference](./docs/api-reference.md)
- [Security Model](./docs/security.md)
- [Signature Format](./docs/SIGNATURE_FORMAT.md)
- [Reentrancy Analysis](./docs/reentrancy-analysis.md)
- [Testing Guide](./docs/testing.md)

There is no `CONTRIBUTING.md` in this repository at present.

## License

MIT
