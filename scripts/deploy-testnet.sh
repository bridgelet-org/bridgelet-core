#!/bin/bash
set -e

echo "🚀 Deploying to Stellar Testnet..."

# Build first
./scripts/build.sh

# Deploy ephemeral_account
echo "Deploying ephemeral_account contract..."
EPHEMERAL_CONTRACT_ID=$(stellar contract deploy \
    --wasm contracts/ephemeral_account/target/wasm32-unknown-unknown/release/ephemeral_account.wasm \
    --source deployer \
    --network testnet)

echo ""
echo "✅ Deployment complete!"
echo ""
echo "📝 Contract IDs:"
echo "EPHEMERAL_ACCOUNT_CONTRACT_ID=$EPHEMERAL_CONTRACT_ID"
echo ""
echo "Add these to your .env file in bridgelet-sdk"