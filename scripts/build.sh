#!/bin/bash
# build.sh – Build all Bridgelet Core Soroban contracts to WASM
# Usage:   ./scripts/build.sh [contract_name]
#   If no argument is supplied, all contracts are built.
# Requires: rustup, wasm32-unknown-unknown target
set -euo pipefail

CONTRACTS=(ephemeral_account reserve_contract sweep_controller)
TARGET="wasm32-unknown-unknown"
PROFILE="release"

# Optionally build only one contract
if [[ "${1-}" != "" ]]; then
  CONTRACTS=("$1")
fi

# Ensure WASM target is installed
if ! rustup target list --installed | grep -q "$TARGET"; then
  echo "⚙️  Adding Rust target $TARGET …"
  rustup target add "$TARGET"
fi

echo "🔨 Building Bridgelet Core contracts (profile: $PROFILE) …"
echo ""

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

for CONTRACT in "${CONTRACTS[@]}"; do
  CONTRACT_DIR="$REPO_ROOT/contracts/$CONTRACT"
  if [[ ! -d "$CONTRACT_DIR" ]]; then
    echo "⚠️  Contract directory not found: $CONTRACT_DIR – skipping"
    continue
  fi

  echo "▶ Building $CONTRACT …"
  (cd "$CONTRACT_DIR" && cargo build --target "$TARGET" --profile "$PROFILE")
  WASM_PATH="$CONTRACT_DIR/target/$TARGET/$PROFILE/${CONTRACT//-/_}.wasm"
  if [[ -f "$WASM_PATH" ]]; then
    SIZE=$(du -sh "$WASM_PATH" | cut -f1)
    echo "  ✅ $WASM_PATH ($SIZE)"
  else
    echo "  ⚠️  WASM not found at expected path: $WASM_PATH"
  fi
  echo ""
done

echo "✅ Build complete!"
echo ""
echo "WASM artifacts:"
find "$REPO_ROOT/target/$TARGET/$PROFILE" -maxdepth 1 -name "*.wasm" \
  ! -name "*.d.wasm" 2>/dev/null | while read -r f; do
  echo "  $(du -sh "$f" | cut -f1)  $f"
done