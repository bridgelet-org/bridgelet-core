#!/bin/bash
set -e

# ---------------------------------------------------------------------------
# Parse --network flag (default: testnet)
# ---------------------------------------------------------------------------
NETWORK="testnet"
while [[ $# -gt 0 ]]; do
    case "$1" in
        --network)
            NETWORK="$2"
            shift 2
            ;;
        *)
            echo "Unknown argument: $1" >&2
            echo "Usage: $0 [--network testnet|mainnet|futurenet]" >&2
            exit 1
            ;;
    esac
done

# ---------------------------------------------------------------------------
# Validate --network matches NODE_ENV when NODE_ENV is set.
# Prevents accidentally deploying to mainnet from a testnet shell (or vice versa).
# ---------------------------------------------------------------------------
if [[ -n "$NODE_ENV" ]]; then
    if [[ "$NODE_ENV" != "$NETWORK" ]]; then
        echo "❌ Network mismatch: --network=$NETWORK but NODE_ENV=$NODE_ENV" >&2
        echo "   Set NODE_ENV=$NETWORK or pass --network=$NODE_ENV to align them." >&2
        exit 1
    fi
fi

# Warn loudly when targeting mainnet
if [[ "$NETWORK" == "mainnet" ]]; then
    echo "⚠️  WARNING: deploying to MAINNET. Press Ctrl-C within 5 seconds to abort."
    sleep 5
fi

echo "🚀 Deploying to Stellar ${NETWORK}..."

# ---------------------------------------------------------------------------
# Pre-flight: verify deployer account exists and has sufficient balance.
# Requires the Stellar CLI 'deployer' identity to be configured.
# ---------------------------------------------------------------------------
echo "Checking deployer account..."
DEPLOYER_ADDRESS=$(stellar keys address deployer 2>/dev/null) || {
    echo "❌ Could not resolve 'deployer' identity. Run: stellar keys generate deployer" >&2
    exit 1
}

BALANCE_OUTPUT=$(stellar account balance "$DEPLOYER_ADDRESS" --network "$NETWORK" 2>/dev/null) || {
    echo "❌ Account $DEPLOYER_ADDRESS not found on $NETWORK." >&2
    echo "   Fund it first: https://friendbot.stellar.org/?addr=$DEPLOYER_ADDRESS" >&2
    exit 1
}

# Require at least 5 XLM (in stroops: 50000000) to cover base reserves + fees
NATIVE_BALANCE=$(echo "$BALANCE_OUTPUT" | grep -i '"native"' | grep -oE '"balance":"[0-9]+"' | grep -oE '[0-9]+' || echo "0")
MIN_BALANCE=50000000
if (( NATIVE_BALANCE < MIN_BALANCE )); then
    READABLE=$(echo "scale=7; $NATIVE_BALANCE/10000000" | bc)
    echo "❌ Deployer balance too low: ${READABLE} XLM (need ≥ 5 XLM)." >&2
    exit 1
fi

echo "✅ Deployer: $DEPLOYER_ADDRESS (balance check passed)"

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------
./scripts/build.sh

# ---------------------------------------------------------------------------
# Deploy
# ---------------------------------------------------------------------------
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
echo ""
echo "Add these to your .env file in bridgelet-sdk"
echo "  SWEEP_CONTRACT_ID=$SWEEP_CONTRACT_ID"
echo ""
echo "📄 Artifact written to artifacts/deployed.json"
