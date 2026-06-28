set -euo pipefail

# ---------------------------------------------------------------------------
# Bridgelet — Testnet Deployment Script
# Deploys EphemeralAccount and SweepController contracts to Stellar testnet
# and records the resulting contract IDs.
#
# Prerequisites:
#   - stellar CLI installed and configured
#   - SIGNER_SECRET_KEY env var set (the deployer/admin keypair)
#   - AUTHORIZED_SIGNER_PUBLIC_KEY env var set (Ed25519 pubkey for sweep auth)
#   - RECOVERY_ADDRESS env var set (organization's recovery wallet)
# ---------------------------------------------------------------------------

NETWORK="testnet"

NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
HORIZON_URL="https://horizon-testnet.stellar.org"
SOROBAN_RPC_URL="https://soroban-testnet.stellar.org"
WASM_DIR="target/wasm32-unknown-unknown/release"
DEPLOYMENTS_FILE="deployments/testnet.json"

: "${SIGNER_SECRET_KEY:?SIGNER_SECRET_KEY must be set}"
: "${AUTHORIZED_SIGNER_PUBLIC_KEY:?AUTHORIZED_SIGNER_PUBLIC_KEY must be set}"
: "${RECOVERY_ADDRESS:?RECOVERY_ADDRESS must be set}"
: "${CREATOR_ADDRESS:?CREATOR_ADDRESS must be set}"

echo "==> Building contracts..."
./scripts/build.sh

echo "==> Deploying EphemeralAccount contract..."
EPHEMERAL_CONTRACT_ID=$(stellar contract deploy \
  --wasm "$WASM_DIR/ephemeral_account.wasm" \
  --source "$SIGNER_SECRET_KEY" \
  --network "$NETWORK" \
  --rpc-url "$SOROBAN_RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE")

echo "    EphemeralAccount deployed: $EPHEMERAL_CONTRACT_ID"

echo "==> Deploying SweepController contract..."
SWEEP_CONTRACT_ID=$(stellar contract deploy \
  --wasm "$WASM_DIR/sweep_controller.wasm" \
  --source "$SIGNER_SECRET_KEY" \
  --network "$NETWORK" \
  --rpc-url "$SOROBAN_RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE")

echo "    SweepController deployed: $SWEEP_CONTRACT_ID"

echo "==> Initializing SweepController..."
stellar contract invoke \
  --id "$SWEEP_CONTRACT_ID" \
  --source "$SIGNER_SECRET_KEY" \
  --network "$NETWORK" \
  --rpc-url "$SOROBAN_RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- initialize \
  --creator "$CREATOR_ADDRESS" \
  --authorized_signer "$AUTHORIZED_SIGNER_PUBLIC_KEY" \
  --authorized_destination null

echo "==> Writing deployment record..."
mkdir -p deployments
cat > "$DEPLOYMENTS_FILE" <<EOF
{
  "network": "$NETWORK",
  "deployedAt": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "contracts": {
    "ephemeralAccount": "$EPHEMERAL_CONTRACT_ID",
    "sweepController": "$SWEEP_CONTRACT_ID"
  },
  "config": {
    "authorizedSigner": "$AUTHORIZED_SIGNER_PUBLIC_KEY",
    "creatorAddress": "$CREATOR_ADDRESS"
  }
}
EOF

echo "==> Deployment complete. Record saved to $DEPLOYMENTS_FILE"
echo ""
echo "    EphemeralAccount : $EPHEMERAL_CONTRACT_ID"
echo "    SweepController  : $SWEEP_CONTRACT_ID"
echo ""
echo "    Set these in your SDK .env:"
echo "    STELLAR_CONTRACT_EPHEMERAL_ACCOUNT=$EPHEMERAL_CONTRACT_ID"
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
echo "    STELLAR_CONTRACT_SWEEP_CONTROLLER=$SWEEP_CONTRACT_ID"
