#!/bin/bash
set -euo pipefail

# ---------------------------------------------------------------------------
# Bridgelet - Local Stellar Sandbox Setup
# Pulls stellar/quickstart, starts in standalone mode, funds test accounts,
# deploys both contracts, and writes contract IDs to sandbox-output.json.
#
# Usage:
#   ./scripts/test-local.sh          # full setup + deploy + smoke tests
#   ./scripts/test-local.sh --clean  # tear down and start fresh
#
# Prerequisites:
#   - Docker running
#   - stellar-cli installed
# ---------------------------------------------------------------------------

NETWORK="standalone"
PASSPHRASE="Standalone Network ; February 2017"
SOROBAN_RPC_URL="http://localhost:8000/soroban/rpc"
HORIZON_URL="http://localhost:8000"
CONTAINER_NAME="bridgelet-sandbox"
WASM_DIR="target/wasm32v1-none/release"
SANDBOX_OUTPUT="sandbox-output.json"

# ── Helper ──────────────────────────────────────────────────────────────
log() { echo -e "\033[1;36m==> $1\033[0m"; }
err() { echo -e "\033[1;31mERROR: $1\033[0m" >&2; exit 1; }

# ── Clean mode ──────────────────────────────────────────────────────────
if [[ "${1:-}" == "--clean" ]]; then
    log "Stopping and removing existing sandbox container..."
    docker rm -f "$CONTAINER_NAME" 2>/dev/null || true
    rm -f "$SANDBOX_OUTPUT"
    log "Clean complete."
    exit 0
fi

# ── Check prerequisites ─────────────────────────────────────────────────
for cmd in docker stellar; do
    command -v "$cmd" >/dev/null 2>&1 || err "$cmd is not installed."
done
docker info >/dev/null 2>&1 || err "Docker is not running."

# ── Start Stellar quickstart container ──────────────────────────────────
if docker ps -a --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
    if docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
        log "Sandbox container already running."
    else
        log "Starting existing sandbox container..."
        docker start "$CONTAINER_NAME"
    fi
else
    log "Pulling stellar/quickstart and starting sandbox..."
    docker run -d --name "$CONTAINER_NAME" \
        -p 8000:8000 \
        --platform linux/amd64 \
        stellar/quickstart:soroban-dev \
        --standalone \
        --enable-soroban-rpc
fi

# Wait for Horizon to become available
log "Waiting for Horizon to become ready..."
for i in $(seq 1 60); do
    if curl -sf "$HORIZON_URL" >/dev/null 2>&1; then
        log "Horizon is ready."
        break
    fi
    if [ "$i" -eq 60 ]; then
        err "Horizon did not become ready within 60 seconds."
    fi
    sleep 1
done

# Wait for Soroban RPC
log "Waiting for Soroban RPC to become ready..."
for i in $(seq 1 60); do
    if curl -sf "$SOROBAN_RPC_URL" >/dev/null 2>&1; then
        log "Soroban RPC is ready."
        break
    fi
    if [ "$i" -eq 60 ]; then
        err "Soroban RPC did not become ready within 60 seconds."
    fi
    sleep 1
done

# ── Fund test accounts ──────────────────────────────────────────────────
log "Creating and funding test accounts..."

# Deployer / admin account
DEPLOYER_SECRET=$(stellar keys generate --network standalone --fund 2>/dev/null | tail -1)
DEPLOYER_ADDRESS=$(stellar keys address "$DEPLOYER_SECRET" --network standalone 2>/dev/null)
log "Deployer: $DEPLOYER_ADDRESS"

# Authorized signer keypair (for off-chain sweep authorization)
SIGNER_SECRET=$(stellar keys generate --network standalone --fund 2>/dev/null | tail -1)
SIGNER_ADDRESS=$(stellar keys address "$SIGNER_SECRET" --network standalone 2>/dev/null)
SIGNER_PUBLIC_KEY=$(stellar keys public "$SIGNER_SECRET" --network standalone 2>/dev/null)
log "Signer: $SIGNER_ADDRESS"

# Recovery address
RECOVERY_SECRET=$(stellar keys generate --network standalone --fund 2>/dev/null | tail -1)
RECOVERY_ADDRESS=$(stellar keys address "$RECOVERY_SECRET" --network standalone 2>/dev/null)
log "Recovery: $RECOVERY_ADDRESS"

# Destination wallet
DESTINATION_SECRET=$(stellar keys generate --network standalone --fund 2>/dev/null | tail -1)
DESTINATION_ADDRESS=$(stellar keys address "$DESTINATION_SECRET" --network standalone 2>/dev/null)
log "Destination: $DESTINATION_ADDRESS"

# ── Build contracts ─────────────────────────────────────────────────────
log "Building contracts..."
./scripts/build.sh

# ── Deploy EphemeralAccount ─────────────────────────────────────────────
log "Deploying EphemeralAccount..."
EPHEMERAL_CONTRACT_ID=$(stellar contract deploy \
    --wasm "$WASM_DIR/ephemeral_account.wasm" \
    --source "$DEPLOYER_SECRET" \
    --network "$NETWORK" \
    --rpc-url "$SOROBAN_RPC_URL" \
    --network-passphrase "$PASSPHRASE" \
    --force) 2>/dev/null
log "EphemeralAccount deployed: $EPHEMERAL_CONTRACT_ID"

# ── Deploy SweepController ──────────────────────────────────────────────
log "Deploying SweepController..."
SWEEP_CONTRACT_ID=$(stellar contract deploy \
    --wasm "$WASM_DIR/sweep_controller.wasm" \
    --source "$DEPLOYER_SECRET" \
    --network "$NETWORK" \
    --rpc-url "$SOROBAN_RPC_URL" \
    --network-passphrase "$PASSPHRASE" \
    --force) 2>/dev/null
log "SweepController deployed: $SWEEP_CONTRACT_ID"

# ── Initialize SweepController ──────────────────────────────────────────
log "Initializing SweepController..."
stellar contract invoke \
    --id "$SWEEP_CONTRACT_ID" \
    --source "$DEPLOYER_SECRET" \
    --network "$NETWORK" \
    --rpc-url "$SOROBAN_RPC_URL" \
    --network-passphrase "$PASSPHRASE" \
    -- initialize \
    --creator "$DEPLOYER_ADDRESS" \
    --authorized_signer "$SIGNER_PUBLIC_KEY" \
    --authorized_destination null

# ── Smoke tests ─────────────────────────────────────────────────────────
log "Running smoke tests..."

# Deploy an EphemeralAccount instance via the contract
log "Deploying test EphemeralAccount instance..."
TEST_ACCOUNT_ID=$(stellar contract deploy \
    --wasm "$WASM_DIR/ephemeral_account.wasm" \
    --source "$DEPLOYER_SECRET" \
    --network "$NETWORK" \
    --rpc-url "$SOROBAN_RPC_URL" \
    --network-passphrase "$PASSPHRASE" \
    --force) 2>/dev/null

EXPIRY_LEDGER=$(( $(stellar ledger --network standalone --rpc-url "$SOROBAN_RPC_URL" 2>/dev/null | grep -o '[0-9]*' | head -1) + 10000 ))
EXPIRY_LEDGER=${EXPIRY_LEDGER:-10000}

log "Initializing test account (expiry: $EXPIRY_LEDGER)..."
stellar contract invoke \
    --id "$TEST_ACCOUNT_ID" \
    --source "$DEPLOYER_SECRET" \
    --network "$NETWORK" \
    --rpc-url "$SOROBAN_RPC_URL" \
    --network-passphrase "$PASSPHRASE" \
    -- initialize \
    --creator "$DEPLOYER_ADDRESS" \
    --expiry_ledger "$EXPIRY_LEDGER" \
    --recovery_address "$RECOVERY_ADDRESS" \
    --authorized_controller "$SWEEP_CONTRACT_ID" \
    --admin "$DEPLOYER_ADDRESS"

log "Checking is_expired (should be false)..."
EXPIRED=$(stellar contract invoke \
    --id "$TEST_ACCOUNT_ID" \
    --source "$DEPLOYER_SECRET" \
    --network "$NETWORK" \
    --rpc-url "$SOROBAN_RPC_URL" \
    --network-passphrase "$PASSPHRASE" \
    -- is_expired 2>/dev/null)
log "is_expired = $EXPIRED"

log "Checking get_status..."
STATUS=$(stellar contract invoke \
    --id "$TEST_ACCOUNT_ID" \
    --source "$DEPLOYER_SECRET" \
    --network "$NETWORK" \
    --rpc-url "$SOROBAN_RPC_URL" \
    --network-passphrase "$PASSPHRASE" \
    -- get_status 2>/dev/null)
log "get_status = $STATUS"

# ── Write sandbox output ────────────────────────────────────────────────
log "Writing $SANDBOX_OUTPUT..."
cat > "$SANDBOX_OUTPUT" <<EOF
{
  "network": "$NETWORK",
  "networkPassphrase": "$PASSPHRASE",
  "rpcUrl": "$SOROBAN_RPC_URL",
  "horizonUrl": "$HORIZON_URL",
  "deployedAt": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "contracts": {
    "ephemeralAccountTemplate": "$EPHEMERAL_CONTRACT_ID",
    "sweepController": "$SWEEP_CONTRACT_ID",
    "testAccount": "$TEST_ACCOUNT_ID"
  },
  "accounts": {
    "deployer": "$DEPLOYER_ADDRESS",
    "deployerSecret": "$DEPLOYER_SECRET",
    "signer": "$SIGNER_ADDRESS",
    "signerSecret": "$SIGNER_SECRET",
    "signerPublicKey": "$SIGNER_PUBLIC_KEY",
    "recovery": "$RECOVERY_ADDRESS",
    "recoverySecret": "$RECOVERY_SECRET",
    "destination": "$DESTINATION_ADDRESS",
    "destinationSecret": "$DESTINATION_SECRET"
  }
}
EOF

log "✅ Sandbox is ready!"
echo ""
echo "  EphemeralAccount template : $EPHEMERAL_CONTRACT_ID"
echo "  SweepController           : $SWEEP_CONTRACT_ID"
echo "  Test EphemeralAccount     : $TEST_ACCOUNT_ID"
echo ""
echo "  Config written to $SANDBOX_OUTPUT"
echo "  Tear down with: ./scripts/test-local.sh --clean"
