#!/bin/bash
set -e

echo "🚀 Deploying to Stellar Testnet..."

# Build first
./scripts/build.sh

# Deploy ephemeral_account
echo "Deploying ephemeral_account contract..."
EPHEMERAL_CONTRACT_ID=$(soroban contract deploy \
    --wasm contracts/ephemeral_account/target/wasm32-unknown-unknown/release/ephemeral_account.wasm \
    --source $DEPLOYER_SECRET_KEY \
    --network testnet)

echo "✅ Ephemeral Account deployed: $EPHEMERAL_CONTRACT_ID"

# Deploy sweep_controller
echo "Deploying sweep_controller contract..."
SWEEP_CONTRACT_ID=$(soroban contract deploy \
    --wasm contracts/sweep_controller/target/wasm32-unknown-unknown/release/sweep_controller.wasm \
    --source $DEPLOYER_SECRET_KEY \
    --network testnet)

echo "✅ Sweep Controller deployed: $SWEEP_CONTRACT_ID"

# Deploy reserve_contract
echo "Deploying reserve_contract contract..."
RESERVE_CONTRACT_ID=$(soroban contract deploy \
    --wasm contracts/reserve_contract/target/wasm32-unknown-unknown/release/reserve_contract.wasm \
    --source $DEPLOYER_SECRET_KEY \
    --network testnet)

echo "✅ Reserve Contract deployed: $RESERVE_CONTRACT_ID"

echo ""
echo "✅ Deployment complete!"
echo ""
echo "📝 Contract IDs:"
echo "EPHEMERAL_ACCOUNT_CONTRACT_ID=$EPHEMERAL_CONTRACT_ID"
echo "SWEEP_CONTROLLER_CONTRACT_ID=$SWEEP_CONTRACT_ID"
echo "RESERVE_CONTRACT_CONTRACT_ID=$RESERVE_CONTRACT_ID"
echo ""

# Save contract IDs to file for CI artifacts
mkdir -p deployment-artifacts
cat > deployment-artifacts/contract-ids.txt <<EOF
EPHEMERAL_ACCOUNT_CONTRACT_ID=$EPHEMERAL_CONTRACT_ID
SWEEP_CONTROLLER_CONTRACT_ID=$SWEEP_CONTRACT_ID
RESERVE_CONTRACT_CONTRACT_ID=$RESERVE_CONTRACT_ID
EOF

echo "Contract IDs saved to deployment-artifacts/contract-ids.txt"
echo ""
echo "Add these to your .env file in bridgelet-sdk"