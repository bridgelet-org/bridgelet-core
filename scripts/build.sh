#!/bin/bash
set -e

echo "🔨 Building Bridgelet Core contracts..."

# Build ephemeral_account contract
echo "Building ephemeral_account..."
cd contracts/ephemeral_account
cargo build --target wasm32-unknown-unknown --release
cd ../..

echo "✅ Build complete!"
echo "Contracts location: contracts/ephemeral_account/target/wasm32-unknown-unknown/release/"
ls -lh contracts/ephemeral_account/target/wasm32-unknown-unknown/release/*.wasm