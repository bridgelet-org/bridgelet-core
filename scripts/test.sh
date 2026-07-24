#!/bin/bash
set -e

echo "🧪 Running tests..."

# Test ephemeral_account
echo "Testing ephemeral_account..."
cd contracts/ephemeral_account
cargo test
cd ../..

# Test sweep_controller
# NOTE: this contract's unit tests live in src/, but its integration tests
# live under tests/integration.rs and are NOT picked up by a bare `cargo
# test` alone in every workspace layout — run both explicitly to be safe.
echo "Testing sweep_controller..."
cd contracts/sweep_controller
cargo test
cargo test --test integration
cd ../..
 
# Test reserve_contract
# NOTE: previously not run by this script at all.
echo "Testing reserve_contract..."
cd contracts/reserve_contract
cargo test
cd ../..
 
# Test account_factory
# NOTE: previously not run by this script at all — this contract has its
# own test.rs but was completely untested by scripts/test.sh.
echo "Testing account_factory..."
cd contracts/account_factory
cargo test
cd ../..

echo "✅ All tests passed!"