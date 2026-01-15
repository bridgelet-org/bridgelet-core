# Bridgelet Core

**Soroban smart contracts for ephemeral account restrictions**

## Overview

Bridgelet Core contains the Soroban smart contracts that enforce single-use restrictions on ephemeral Stellar accounts and manage the sweep logic for transferring funds to permanent wallets.

## Tech Stack

- **Language:** Rust
- **Framework:** Soroban SDK 22.0.0
- **Testing:** soroban-cli + Rust test framework
- **Build:** Cargo + stellar-cli

## Contracts

### 1. EphemeralAccount Contract
Manages restrictions on temporary accounts:
- Single inbound payment enforcement
- Authorized sweep destination
- Time-based expiration logic
- Event emission for auditability

### 2. SweepController Contract
Handles fund transfers:
- Validates claim authorization
- Executes atomic sweeps
- Handles multi-asset transfers
- Reclaims base reserves

## Project Structure

contracts/
├── ephemeral_account/
│   ├── src/
│   │   ├── lib.rs           # Main contract
│   │   ├── storage.rs       # State management
│   │   ├── events.rs        # Event definitions
│   │   └── errors.rs        # Error types
│   └── Cargo.toml
├── sweep_controller/
│   ├── src/
│   │   ├── lib.rs
│   │   ├── authorization.rs
│   │   └── transfers.rs
│   └── Cargo.toml
└── shared/
└── types.rs             # Shared types

## Prerequisites
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Soroban CLI
cargo install --locked soroban-cli --version 22.0.0

# Add wasm target
rustup target add wasm32-unknown-unknown
```

## Build & Deploy
```bash
# Build contracts
./scripts/build.sh

# Run tests
cargo test

# Deploy to testnet
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/ephemeral_account.wasm \
  --network testnet \
  --source SIGNER_SECRET_KEY
```

## Testing
```bash
# Unit tests
cargo test

# Integration tests
cargo test --test integration

# Deploy to local sandbox for testing
./scripts/test-local.sh
```

## Contract Interfaces

### EphemeralAccount
```rust
pub trait EphemeralAccountInterface {
    // Initialize ephemeral account with restrictions
    fn initialize(
        env: Env,
        creator: Address,
        sweep_destination: Option<Address>,
        expiry_timestamp: u64
    ) -> Result<(), Error>;
    
    // Record inbound payment (called automatically)
    fn record_payment(env: Env, amount: i128, asset: Address) -> Result<(), Error>;
    
    // Execute sweep to permanent wallet
    fn sweep(env: Env, destination: Address) -> Result<(), Error>;
    
    // Check if account is expired
    fn is_expired(env: Env) -> bool;
}
```

See [Bridgelet Documentation](https://github.com/bridgelet-org/bridgelet) for full API reference.

## Events

Contracts emit events for off-chain monitoring:
```rust
AccountCreated { account_id, creator, expiry }
PaymentReceived { account_id, amount, asset }
SweepExecuted { account_id, destination, amount }
AccountExpired { account_id }
```

## Security Considerations

- All storage keys use proper namespacing
- Authorization checks on every state-changing operation
- Reentrancy protection via Soroban's execution model
- Timestamp-based expiration uses ledger time

See [Security Audit Report](./docs/security-audit.md) (coming soon)

## Documentation

- [Contract Architecture](./docs/architecture.md)
- [API Reference](./docs/api-reference.md)
- [Security Model](./docs/security.md)
- [Testing Guide](./docs/testing.md)

## License

MIT