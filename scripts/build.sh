#!/bin/bash
set -e

echo "🔨 Building Bridgelet Core contracts..."

# Build ephemeral_account contract
echo "Building ephemeral_account..."
cd contracts/ephemeral_account
cargo build --target wasm32-unknown-unknown --release
cd ../..

# Build sweep_controller contract
echo "Building sweep_controller..."
cd contracts/sweep_controller
cargo build --target wasm32-unknown-unknown --release
cd ../..

# Build reserve_contract contract
echo "Building reserve_contract..."
cd contracts/reserve_contract
cargo build --target wasm32-unknown-unknown --release
cd ../..

echo "✅ Build complete!"
echo "Contracts location: contracts/*/target/wasm32-unknown-unknown/release/"
ls -lh contracts/*/target/wasm32-unknown-unknown/release/*.wasm