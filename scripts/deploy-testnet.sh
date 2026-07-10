set -euo pipefail

# ---------------------------------------------------------------------------
# Bridgelet - Testnet Deployment Script
# Deploys all four workspace contracts to Stellar testnet and records the
# resulting contract IDs.
#
# CHANGES from the previous version of this script:
#   - Previously referenced $RESERVE_CONTRACT_ID in its final output/artifact
#     steps without ever assigning it. Under `set -euo pipefail` that caused
#     an "unbound variable" crash after SweepController was deployed. This
#     version actually deploys and initializes reserve_contract.
#   - account_factory was a workspace member that this script never touched
#     at all. This version deploys it and initializes it with
#     ephemeral_account's installed WASM hash.
#
# Prerequisites:
#   - stellar CLI installed and configured
#   - SIGNER_SECRET_KEY env var set (the deployer/admin keypair)
#   - AUTHORIZED_SIGNER_PUBLIC_KEY env var set (Ed25519 pubkey for sweep auth)
#   - RECOVERY_ADDRESS env var set (organization's recovery wallet)
#   - CREATOR_ADDRESS env var set (creator for SweepController::initialize)
#   - RESERVE_ADMIN_ADDRESS env var (optional - defaults to CREATOR_ADDRESS)
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

# Optional: falls back to CREATOR_ADDRESS if not explicitly provided.
RESERVE_ADMIN_ADDRESS="${RESERVE_ADMIN_ADDRESS:-$CREATOR_ADDRESS}"
 
echo "==> Building contracts..."
./scripts/build.sh

# ---------------------------------------------------------------------------
# EphemeralAccount
# ---------------------------------------------------------------------------
echo "==> Deploying EphemeralAccount contract..."
EPHEMERAL_CONTRACT_ID=$(stellar contract deploy \
  --wasm "$WASM_DIR/ephemeral_account.wasm" \
  --source "$SIGNER_SECRET_KEY" \
  --network "$NETWORK" \
  --rpc-url "$SOROBAN_RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE")

echo "    EphemeralAccount deployed: $EPHEMERAL_CONTRACT_ID"

# Install (upload) the ephemeral_account WASM separately so we have its hash
# on hand for AccountFactory::initialize below. `stellar contract deploy`
# already installs the wasm under the hood, but `contract install` gives us
# the hash directly without redeploying.
echo "==> Installing EphemeralAccount WASM (for AccountFactory)..."
EPHEMERAL_WASM_HASH=$(stellar contract install \
  --wasm "$WASM_DIR/ephemeral_account.wasm" \
  --source "$SIGNER_SECRET_KEY" \
  --network "$NETWORK" \
  --rpc-url "$SOROBAN_RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE")
 
echo "    EphemeralAccount WASM hash: $EPHEMERAL_WASM_HASH"
 
# ---------------------------------------------------------------------------
# SweepController
# ---------------------------------------------------------------------------
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

# ---------------------------------------------------------------------------
# ReserveContract
# ---------------------------------------------------------------------------
echo "==> Deploying ReserveContract contract..."
RESERVE_CONTRACT_ID=$(stellar contract deploy \
  --wasm "$WASM_DIR/reserve_contract.wasm" \
  --source "$SIGNER_SECRET_KEY" \
  --network "$NETWORK" \
  --rpc-url "$SOROBAN_RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE")
 
echo "    ReserveContract deployed: $RESERVE_CONTRACT_ID"
 
echo "==> Initializing ReserveContract..."
stellar contract invoke \
  --id "$RESERVE_CONTRACT_ID" \
  --source "$SIGNER_SECRET_KEY" \
  --network "$NETWORK" \
  --rpc-url "$SOROBAN_RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- initialize \
  --admin "$RESERVE_ADMIN_ADDRESS"
 
# ---------------------------------------------------------------------------
# AccountFactory
# ---------------------------------------------------------------------------
echo "==> Deploying AccountFactory contract..."
FACTORY_CONTRACT_ID=$(stellar contract deploy \
  --wasm "$WASM_DIR/account_factory.wasm" \
  --source "$SIGNER_SECRET_KEY" \
  --network "$NETWORK" \
  --rpc-url "$SOROBAN_RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE")
 
echo "    AccountFactory deployed: $FACTORY_CONTRACT_ID"
 
echo "==> Initializing AccountFactory with EphemeralAccount WASM hash..."
stellar contract invoke \
  --id "$FACTORY_CONTRACT_ID" \
  --source "$SIGNER_SECRET_KEY" \
  --network "$NETWORK" \
  --rpc-url "$SOROBAN_RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- initialize \
  --ephemeral_account_wasm_hash "$EPHEMERAL_WASM_HASH"
 
# ---------------------------------------------------------------------------
# Record deployment
# ---------------------------------------------------------------------------
echo "==> Writing deployment record..."
mkdir -p deployments
cat > "$DEPLOYMENTS_FILE" <<EOF
{
  "network": "$NETWORK",
  "deployedAt": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "contracts": {
    "ephemeralAccount": "$EPHEMERAL_CONTRACT_ID",
    "sweepController": "$SWEEP_CONTRACT_ID",
    "reserveContract": "$RESERVE_CONTRACT_ID",
    "accountFactory": "$FACTORY_CONTRACT_ID"
  },
  "config": {
    "authorizedSigner": "$AUTHORIZED_SIGNER_PUBLIC_KEY",
    "creatorAddress": "$CREATOR_ADDRESS",
    "reserveAdminAddress": "$RESERVE_ADMIN_ADDRESS",
    "recoveryAddress": "$RECOVERY_ADDRESS",
    "ephemeralAccountWasmHash": "$EPHEMERAL_WASM_HASH"
  }
}
EOF

echo "==> Deployment complete. Record saved to $DEPLOYMENTS_FILE"
echo ""
echo "    EphemeralAccount : $EPHEMERAL_CONTRACT_ID"
echo "    SweepController  : $SWEEP_CONTRACT_ID"
echo "    ReserveContract  : $RESERVE_CONTRACT_ID"
echo "    AccountFactory   : $FACTORY_CONTRACT_ID"
echo ""
echo "    Set these in your SDK .env:"
echo "    STELLAR_CONTRACT_EPHEMERAL_ACCOUNT=$EPHEMERAL_CONTRACT_ID"
echo "    SWEEP_CONTROLLER_CONTRACT_ID=$SWEEP_CONTRACT_ID"
echo "    RESERVE_CONTRACT_CONTRACT_ID=$RESERVE_CONTRACT_ID"
echo "    ACCOUNT_FACTORY_CONTRACT_ID=$FACTORY_CONTRACT_ID"
echo ""

# Save contract IDs to file for CI artifacts
mkdir -p deployment-artifacts
cat > deployment-artifacts/contract-ids.txt <<EOF
EPHEMERAL_ACCOUNT_CONTRACT_ID=$EPHEMERAL_CONTRACT_ID
SWEEP_CONTROLLER_CONTRACT_ID=$SWEEP_CONTRACT_ID
RESERVE_CONTRACT_CONTRACT_ID=$RESERVE_CONTRACT_ID
ACCOUNT_FACTORY_CONTRACT_ID=$FACTORY_CONTRACT_ID
EOF

echo "Contract IDs saved to deployment-artifacts/contract-ids.txt"
