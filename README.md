# Bridgelet Core

**Soroban smart contracts for ephemeral account restrictions**

**Status:** Active Development - MVP. Not audited. See [MVP Status](#mvp-status) before deploying anywhere real funds are at risk.

## Overview

Bridgelet Core contains the Soroban smart contracts that enforce single-use restrictions on ephemeral Stellar accounts and manage the sweep logic for transferring funds to permanent wallets.

The workspace contains **four** contracts, not two or three as earlier drafts of this README stated:

| Contract | Purpose |
|---|---|
| `ephemeral_account` | Enforces single-payment, expiry, and sweep-authorization state machine for a temporary account |
| `sweep_controller` | Validates Ed25519-signed sweep authorization and executes SEP-41 token transfers |
| `reserve_contract` | Stores/serves the network base-reserve amount (admin-set config value) used by `ephemeral_account` when reclaiming reserve |
| `account_factory` | Batch-deploys and initializes many `ephemeral_account` instances in one transaction |

## MVP Status

### Current Stub Inventory

| Function | Contract | Status | Notes |
|----------|----------|--------|-------|
| `verify_sweep_authorization` | `EphemeralAccount` | **Not a real signature check** | Ignores the `auth_signature` argument entirely (parameter is prefixed `_`). Authorization instead comes from `authorized_controller.require_auth()` - i.e. it trusts whichever address was set as the controller at `initialize()`. Calling `sweep()` directly (not via `SweepController`) will fail `require_auth` for anyone who isn't that controller, but it performs **no cryptographic verification of the signature itself**. |
| `verify_sweep_auth` | `SweepController` | **Fully implemented** | Real Ed25519 verification (`env.crypto().ed25519_verify`) over `hash(destination + nonce + contract_id)`, with nonce-based replay protection. |
| `execute_transfers` | `SweepController` | **Fully implemented** | Calls SEP-41 `TokenClient::transfer()` for every recorded payment. |
| `batch_initialize` | `AccountFactory` | **Implemented, error detail dropped** | On per-account init failure it returns `error: None` instead of the actual error - see `lib.rs` comment `"In a real implementation, we'd serialize errors"`. Caller can see *that* an account failed but not *why*. |

### Implementation Notes

- **Always deploy sweeps through `SweepController::execute_sweep()` or `::claim()`.** Calling `EphemeralAccount::sweep()` directly bypasses the Ed25519 check entirely - it only works at all because of the `authorized_controller.require_auth()` gate, not because the signature was verified.
- **`SweepController::claim()`** is a gas-free path: the recipient signs a Soroban auth entry for `claim()`, a relayer submits and pays fees, and the controller uses `authorize_as_current_contract()` to satisfy `EphemeralAccount`'s controller check.
- **Reserve tracking is duplicated across two contracts.** `EphemeralAccount` has its own internal `BASE_RESERVE_STROOPS` constant and reserve-tracking storage (`reclaim_reserve_to`), while `ReserveContract` independently stores an admin-settable base reserve value. Nothing in the current code wires `ReserveContract`'s value into `EphemeralAccount`'s reserve logic - confirm this is intentional (e.g. `ReserveContract` feeding a future on-chain read) before relying on `ReserveContract::set_base_reserve` to actually change sweep behavior.
- **`AccountFactory` is real but entirely undocumented and undeployed.** It exists in `contracts/account_factory`, is a workspace member, but is not built by `scripts/build.sh`, not deployed by `scripts/deploy-testnet.sh`, and not tested in CI.

## CI/CD - currently disabled

Despite earlier documentation claiming automated test and deploy pipelines, **both GitHub Actions workflows are non-functional as committed**:

- **`.github/workflows/test.yml` is entirely commented out** (76 of 89 lines are `#`-prefixed; there is no active `on:` trigger or job). No tests, formatting checks, or clippy run automatically on push/PR today.
- **`.github/workflows/deploy-testnet.yml` runs tests/fmt/clippy/build for `ephemeral_account`, `sweep_controller`, and `reserve_contract` only** (not `account_factory`) - but **the actual deployment, artifact-upload, and summary steps are commented out**. The workflow currently only validates the build; it never deploys anything, regardless of what triggers it.

If you need CI, uncomment and fix these before relying on them, and add `account_factory` to all three (test/fmt/clippy/build) steps in both files.

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
brew install binaryen        # macOS
apt-get install binaryen     # Ubuntu/Debian
# or download from: https://github.com/WebAssembly/binaryen/releases
```

## Build & Deploy

```bash
# Build ephemeral_account, sweep_controller, and reserve_contract (NOT account_factory - see below)
./scripts/build.sh

# ⚠️ account_factory is not built by build.sh. To build it too:
cd contracts/account_factory && cargo build --target wasm32-unknown-unknown --release && cd ../..
```

`scripts/deploy-testnet.sh` deploys **only `ephemeral_account` and `sweep_controller`** to testnet and writes their IDs to `deployments/testnet.json`. It does **not** currently deploy `reserve_contract` or `account_factory`, despite referencing a `RESERVE_CONTRACT_ID` variable in its output/artifact steps that is **never assigned** anywhere in the script - with `set -euo pipefail` active, this will abort the script with an "unbound variable" error at that point. Required env vars (see `.env.example`):

```bash
export SIGNER_SECRET_KEY=...              # deployer/admin keypair (S... secret)
export AUTHORIZED_SIGNER_PUBLIC_KEY=...   # Ed25519 pubkey SweepController verifies sweep signatures against
export RECOVERY_ADDRESS=...               # org recovery wallet, used per-account at initialize()
export CREATOR_ADDRESS=...                # address that initializes SweepController

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

## Contract Interfaces

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