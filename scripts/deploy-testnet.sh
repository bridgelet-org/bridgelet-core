#!/bin/bash
set -e

NETWORK="${NETWORK:-testnet}"

echo "🚀 Deploying to Stellar ${NETWORK}..."

# Build all contracts first
./scripts/build.sh

# Deploy ephemeral_account
echo ""
echo "Deploying ephemeral_account..."
EPHEMERAL_CONTRACT_ID=$(stellar contract deploy \
    --wasm contracts/ephemeral_account/target/wasm32-unknown-unknown/release/ephemeral_account.wasm \
    --source deployer \
    --network "$NETWORK")

# Deploy sweep_controller
echo "Deploying sweep_controller..."
SWEEP_CONTRACT_ID=$(stellar contract deploy \
    --wasm contracts/sweep_controller/target/wasm32-unknown-unknown/release/sweep_controller.wasm \
    --source deployer \
    --network "$NETWORK")

# Write machine-readable artifact consumed by bridgelet-sdk
mkdir -p artifacts
DEPLOYED_AT=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
cat > artifacts/deployed.json <<EOF
{
  "ephemeral_account": "$EPHEMERAL_CONTRACT_ID",
  "sweep_controller": "$SWEEP_CONTRACT_ID",
  "network": "$NETWORK",
  "deployed_at": "$DEPLOYED_AT"
}
EOF

echo ""
echo "✅ Deployment complete!"
echo ""
echo "📝 Contract IDs:"
echo "  EPHEMERAL_ACCOUNT_CONTRACT_ID=$EPHEMERAL_CONTRACT_ID"
echo "  SWEEP_CONTRACT_ID=$SWEEP_CONTRACT_ID"
echo ""
echo "📄 Artifact written to artifacts/deployed.json"
