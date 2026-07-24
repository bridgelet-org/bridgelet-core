#!/bin/bash
set -e

echo "🔨 Building Bridgelet Core contracts..."

# NOTE: previously used `cargo build --target wasm32-unknown-unknown --release`
# per contract. As of Rust 1.82+, that target emits WASM using
# reference-types/multivalue features Soroban's runtime rejects
# (HostError: Error(WasmVm, InvalidAction) / "reference-types not enabled").
# Stellar's own guidance: build contracts with `stellar contract build`
# (targets wasm32v1-none, the only target the Soroban runtime supports),
# not with a raw `cargo build`. See:
# https://docs.rs/soroban-sdk/latest/soroban_sdk/
#
# `stellar contract build` run at the workspace root builds every contract
# crate in the workspace in one pass — no need to cd into each directory.

echo "Checking stellar-cli is available..."
if ! command -v stellar &> /dev/null; then
    echo "❌ stellar-cli not found. Install it first:"
    echo "   cargo install --locked stellar-cli --version 23.4.1"
    echo "   (or a newer stellar-cli release — check 'stellar --version')"
    exit 1
fi

echo "Building all workspace contracts (ephemeral_account, sweep_controller, reserve_contract, account_factory)..."
stellar contract build

# Build sweep_controller contract
echo "Building sweep_controller..."
cd contracts/sweep_controller
cargo build --target wasm32-unknown-unknown --release
cd ../..

# Build reserve_contract contract
echo "Building reserve_contract..."
cd contracts/reserve_contract
cargo build --target wasm32-unknown-unknown --release
cd ../..

echo "✅ Build complete!"
echo "Contracts location: contracts/*/target/wasm32-unknown-unknown/release/"
ls -lh contracts/*/target/wasm32-unknown-unknown/release/*.wasm
