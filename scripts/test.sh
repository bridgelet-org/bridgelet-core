#!/bin/bash
# test.sh – Run all Bridgelet Core Soroban contract tests
# Usage:   ./scripts/test.sh [contract_name]
#   If no argument is supplied, the full workspace is tested.
# Requires: Rust toolchain (cargo)
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "🧪 Running Bridgelet Core contract tests …"
echo ""

if [[ "${1-}" != "" ]]; then
  # Single contract mode
  CONTRACT="$1"
  echo "▶ Testing $CONTRACT …"
  cargo test -p "$CONTRACT" --verbose
else
  # Full workspace mode (mirrors CI)
  echo "▶ Testing bridgelet-shared …"
  cargo test -p bridgelet-shared --verbose

  echo ""
  echo "▶ Testing ephemeral_account …"
  cargo test -p ephemeral_account --verbose

  echo ""
  echo "▶ Testing reserve_contract …"
  cargo test -p reserve_contract --verbose

  echo ""
  echo "▶ Testing sweep_controller (unit + integration) …"
  cargo test -p sweep_controller --verbose
fi

echo ""
echo "✅ All tests passed!"