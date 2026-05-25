# Testing Guide

This document explains how to run tests, write new tests, and understand the testing strategy for Bridgelet Core smart contracts.

## Table of Contents

- [Quick Start](#quick-start)
- [Running Tests](#running-tests)
- [Testing Strategy](#testing-strategy)
- [Writing Tests](#writing-tests)
- [Local Sandbox Testing](#local-sandbox-testing)
- [Troubleshooting](#troubleshooting)

## Quick Start

```bash
# Run all tests
./scripts/test.sh

# Or run tests for a specific contract
cd contracts/ephemeral_account && cargo test
cd contracts/sweep_controller && cargo test
```

## Running Tests

### Unit Tests

Unit tests are located alongside the contract code in `src/test.rs` files. They test individual contract functions in isolation.

#### Run all unit tests:

```bash
# From project root
./scripts/test.sh

# Or manually
cd contracts/ephemeral_account
cargo test
```

#### Run specific test:

```bash
cd contracts/ephemeral_account
cargo test test_initialize
```

#### Run tests with output:

```bash
cargo test -- --nocapture
```

#### Run tests in a specific contract:

```bash
# Ephemeral Account contract
cd contracts/ephemeral_account && cargo test

# Sweep Controller contract
cd contracts/sweep_controller && cargo test
```

### Integration Tests

Integration tests are located in the `tests/` directory and test interactions between multiple contracts.

#### Run integration tests:

```bash
cd contracts/sweep_controller
cargo test --test integration
```

#### Run all tests (unit + integration):

```bash
cd contracts/sweep_controller
cargo test
```

### Test Coverage

While Rust doesn't have built-in coverage tools, you can use external tools:

```bash
# Install cargo-tarpaulin (coverage tool)
cargo install cargo-tarpaulin

# Run with coverage
cargo tarpaulin --out Html
```

## Testing Strategy

### Goals

1. **Unit Test Coverage**: Aim for >90% coverage of all contract functions
2. **Integration Testing**: Test all cross-contract interactions
3. **Edge Cases**: Test error conditions, boundary values, and invalid inputs
4. **State Transitions**: Verify all valid state transitions and prevent invalid ones

### Test Categories

#### 1. Unit Tests (`src/test.rs`)

Unit tests verify individual contract functions work correctly in isolation:

- **Initialization tests**: Verify contract can be initialized correctly
- **State transition tests**: Verify valid state changes
- **Error handling tests**: Verify proper error responses for invalid operations
- **Authorization tests**: Verify access control works correctly
- **Boundary tests**: Test edge cases (zero values, max values, etc.)

#### 2. Integration Tests (`tests/integration.rs`)

Integration tests verify contracts work together:

- **Cross-contract calls**: Test interactions between contracts
- **End-to-end flows**: Test complete user workflows
- **Multi-contract state**: Verify state consistency across contracts

### Test Structure

Each test should follow the Arrange-Act-Assert pattern:

1. **Arrange**: Set up the test environment (create Env, deploy contracts, generate addresses)
2. **Act**: Execute the function being tested
3. **Assert**: Verify the expected outcome

## Writing Tests

### Unit Test Example

Here's an example of a unit test from `ephemeral_account/src/test.rs`:

```rust
#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env, BytesN};

    #[test]
    fn test_initialize() {
        // Arrange
        let env = Env::default();
        let contract_id = env.register_contract(None, EphemeralAccountContract);
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        // Act
        client.initialize(&creator, &expiry_ledger, &recovery);

        // Assert
        let status = client.get_status();
        assert_eq!(status, AccountStatus::Active);
        assert_eq!(client.is_expired(), false);
    }
}
```

### Testing Error Conditions

Use `#[should_panic]` to test error conditions:

```rust
#[test]
#[should_panic(expected = "Error(Contract, #1)")] // AlreadyInitialized
fn test_double_initialize() {
    let env = Env::default();
    let contract_id = env.register_contract(None, EphemeralAccountContract);
    let client = EphemeralAccountContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let expiry_ledger = env.ledger().sequence() + 1000;

    // First initialization should succeed
    client.initialize(&creator, &expiry_ledger, &recovery);

    // Second initialization should panic
    client.initialize(&creator, &expiry_ledger, &recovery);
}
```

### Testing with Mocked Auth

For functions that require authorization, use `env.mock_all_auths()`:

```rust
#[test]
fn test_expiration() {
    let env = Env::default();
    env.mock_all_auths(); // Mock all authorization checks
    
    let contract_id = env.register_contract(None, EphemeralAccountContract);
    let client = EphemeralAccountContractClient::new(&env, &contract_id);

    // ... rest of test
}
```

### Integration Test Example

Here's an example of an integration test from `sweep_controller/tests/integration.rs`:

```rust
#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};
use sweep_controller::{SweepController, SweepControllerClient};

#[test]
fn test_execute_sweep() {
    // Arrange
    let env = Env::default();
    env.mock_all_auths();

    // Deploy ephemeral account
    let ephemeral_id = env.register_contract(None, ephemeral_account::EphemeralAccountContract);
    let ephemeral_client =
        ephemeral_account::EphemeralAccountContractClient::new(&env, &ephemeral_id);

    // Deploy sweep controller
    let controller_id = env.register_contract(None, SweepController);
    let controller_client = SweepControllerClient::new(&env, &controller_id);

    // Setup test data
    let creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let destination = Address::generate(&env);
    let asset = Address::generate(&env);
    let expiry = env.ledger().sequence() + 1000;

    // Initialize ephemeral account
    ephemeral_client.initialize(&creator, &expiry, &recovery);
    ephemeral_client.record_payment(&100, &asset);

    // Act
    assert!(controller_client.can_sweep(&ephemeral_id));
    
    let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
    controller_client.execute_sweep(&ephemeral_id, &destination, &auth_sig);

    // Assert
    let status = ephemeral_client.get_status();
    assert_eq!(status, ephemeral_account::AccountStatus::Swept);
}
```

### Testing Ledger Time

To test time-based logic, manipulate the ledger sequence:

```rust
#[test]
fn test_expiration() {
    let env = Env::default();
    let contract_id = env.register_contract(None, EphemeralAccountContract);
    let client = EphemeralAccountContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let expiry_ledger = env.ledger().sequence() + 10;

    // Initialize
    client.initialize(&creator, &expiry_ledger, &recovery);
    assert_eq!(client.is_expired(), false);

    // Advance ledger past expiry
    env.ledger().set_sequence_number(expiry_ledger + 1);

    // Verify expiration
    assert_eq!(client.is_expired(), true);
}
```

### Best Practices

1. **Use descriptive test names**: Test names should clearly describe what they test
   - Good: `test_double_initialize_panics`
   - Bad: `test1`

2. **Test one thing per test**: Each test should verify a single behavior

3. **Use helper functions**: Extract common setup code into helper functions

4. **Test both success and failure paths**: Verify functions work correctly and fail appropriately

5. **Test edge cases**: Zero values, maximum values, boundary conditions

6. **Verify state changes**: After calling a function, verify all expected state changes occurred

7. **Test events**: Verify events are emitted correctly (if applicable)

## Local Sandbox Testing

### Prerequisites

```bash
# Install Soroban CLI
cargo install --locked soroban-cli --version 22.0.0

# Verify installation
soroban --version
```

### Starting a Local Sandbox

```bash
# Start local sandbox (runs in foreground)
soroban sandbox start

# Or run in background
soroban sandbox start --background
```

The sandbox will start on:
- **RPC URL**: `http://localhost:8000`
- **Network Passphrase**: Test SDF Network ; September 2015

### Deploying Contracts to Sandbox

```bash
# Build contract first
cd contracts/ephemeral_account
cargo build --target wasm32-unknown-unknown --release

# Deploy to local sandbox
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/ephemeral_account.wasm \
  --network local \
  --source-account <ACCOUNT_SECRET_KEY>
```

### Interacting with Deployed Contracts

```bash
# Invoke a function
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network local \
  --source-account <ACCOUNT_SECRET_KEY> \
  -- initialize \
  --creator <CREATOR_ADDRESS> \
  --expiry_ledger 1000 \
  --recovery_address <RECOVERY_ADDRESS>
```

### Using Test Scripts

The project includes test scripts in `scripts/`:

```bash
# Run all tests
./scripts/test.sh

# Build contracts
./scripts/build.sh

# Deploy to testnet (not local)
./scripts/deploy-testnet.sh
```

### Stopping the Sandbox

```bash
# If running in foreground, use Ctrl+C

# If running in background, find and kill the process
pkill soroban-sandbox
```

## Troubleshooting

### Common Issues

#### Tests fail with "Contract not found"

**Problem**: Contract WASM file not built or path is incorrect.

**Solution**:
```bash
# Build the contract first
cd contracts/ephemeral_account
cargo build --target wasm32-unknown-unknown --release

# Verify WASM file exists
ls target/wasm32-unknown-unknown/release/ephemeral_account.wasm
```

#### Integration tests fail with import errors

**Problem**: Integration tests need the contract WASM to be built first.

**Solution**:
```bash
# Build ephemeral_account contract first
cd contracts/ephemeral_account
cargo build --target wasm32-unknown-unknown --release

# Then run integration tests
cd ../sweep_controller
cargo test --test integration
```

#### Authorization errors in tests

**Problem**: Tests calling functions that require authorization fail.

**Solution**: Use `env.mock_all_auths()` before calling authorized functions:

```rust
#[test]
fn test_authorized_function() {
    let env = Env::default();
    env.mock_all_auths(); // Add this line
    
    // ... rest of test
}
```

#### Tests fail with "Error(Contract, #X)"

**Problem**: Contract error codes can be hard to debug.

**Solution**: 
1. Check the error definitions in `src/errors.rs`
2. Use `#[should_panic(expected = "Error(Contract, #X)")]` to test expected errors
3. Add debug logging (if needed) using `env.log()` in contract code

#### Ledger sequence issues

**Problem**: Tests involving time/ledger sequence fail unexpectedly.

**Solution**: 
- Always check current ledger sequence: `env.ledger().sequence()`
- Set ledger sequence explicitly: `env.ledger().set_sequence_number(n)`
- Ensure expiry is in the future: `expiry_ledger > env.ledger().sequence()`

#### WASM build errors

**Problem**: `cargo build` fails with WASM target errors.

**Solution**:
```bash
# Add WASM target
rustup target add wasm32-unknown-unknown

# Verify installation
rustup target list --installed | grep wasm32
```

#### Test compilation errors

**Problem**: Tests don't compile due to missing imports or type errors.

**Solution**:
1. Ensure `#[cfg(test)]` is used for test modules
2. Import test utilities: `use soroban_sdk::testutils::Address as _;`
3. Check that `dev-dependencies` in `Cargo.toml` includes `soroban-sdk` with `testutils` feature:
   ```toml
   [dev-dependencies]
   soroban-sdk = { version = "22.0.0", features = ["testutils"] }
   ```

#### Sandbox connection issues

**Problem**: Cannot connect to local sandbox.

**Solution**:
```bash
# Check if sandbox is running
curl http://localhost:8000

# Restart sandbox
pkill soroban-sandbox
soroban sandbox start --background

# Check sandbox logs
soroban sandbox logs
```

#### Test timeout or hangs

**Problem**: Tests run indefinitely or timeout.

**Solution**:
1. Check for infinite loops in contract code
2. Verify all authorization checks are mocked
3. Ensure ledger sequence is set correctly for time-based tests
4. Run tests with timeout: `cargo test -- --test-threads=1`

### Getting Help

1. **Check contract logs**: Add `env.log()` statements in contract code (remove before production)
2. **Run tests with output**: `cargo test -- --nocapture` to see print statements
3. **Check Soroban SDK documentation**: [Soroban SDK Docs](https://soroban.stellar.org/docs)
4. **Review existing tests**: Look at `src/test.rs` and `tests/integration.rs` for examples

### Debug Tips

1. **Use `env.log()` for debugging** (remove before production):
   ```rust
   env.log(&format!("Debug: value = {}", value));
   ```

2. **Test in isolation**: Comment out other tests to isolate the failing test

3. **Check test order**: Some tests might depend on state from previous tests

4. **Verify test data**: Print test values to ensure they're what you expect

5. **Use `cargo test -- --nocapture`**: See all output including logs

## Additional Resources

- [Soroban Documentation](https://soroban.stellar.org/docs)
- [Soroban SDK Reference](https://docs.rs/soroban-sdk/)
- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Cargo Test Documentation](https://doc.rust-lang.org/cargo/commands/cargo-test.html)


