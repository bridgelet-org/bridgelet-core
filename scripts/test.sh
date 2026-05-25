#!/bin/bash
set -e

echo "🧪 Running tests..."

# Test ephemeral_account
echo "Testing ephemeral_account..."
cd contracts/ephemeral_account
cargo test
cd ../..

echo "✅ All tests passed!"