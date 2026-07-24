# Stellar Testnet Deployment Guide

## Overview

This document describes how to deploy and verify both bridgelet-core
contracts on Stellar testnet.  It should be completed **after** Issues
#69, #70, and #71 are resolved (build scripts, deployment scripts, and
contract verification).

## Prerequisites

- `stellar-cli` installed (`cargo install --locked stellar-cli`)
- A funded testnet account (use the [Stellar Laboratory](https://laboratory.stellar.org/) friendbot)
- Environment variables configured (see below)

## Environment Setup

```bash
export SIGNER_SECRET_KEY="S..."          # Deployer/admin secret key
export AUTHORIZED_SIGNER_PUBLIC_KEY="..." # Ed25519 pubkey for off-chain sweep signing
export RECOVERY_ADDRESS="G..."           # Organization's recovery wallet
export CREATOR_ADDRESS="G..."            # Creator for SweepController::initialize
```

## Deployment Steps

### 1. Build Contracts

```bash
./scripts/build.sh
```

This produces WASM artifacts in `target/wasm32v1-none/release/`:
- `ephemeral_account.wasm`
- `sweep_controller.wasm`
- `reserve_contract.wasm`
- `account_factory.wasm`

### 2. Deploy via Script

```bash
./scripts/deploy-testnet.sh
```

The script will:
1. Deploy EphemeralAccount and record its contract ID
2. Deploy SweepController and initialize it with the authorized signer
3. Deploy ReserveContract and AccountFactory
4. Write all contract IDs to `deployments/testnet.json`
5. Write a `deployment-artifacts/contract-ids.txt` for CI consumption

### 3. Verify Deployment

After deployment, run these smoke tests via `stellar-cli`:

```bash
# Set contract IDs from the deployment output
EPHEMERAL_ID=<from deployments/testnet.json>
SWEEP_ID=<from deployments/testnet.json>

# Test EphemeralAccount.is_expired()
stellar contract invoke \
  --id "$EPHEMERAL_ID" \
  --source "$SIGNER_SECRET_KEY" \
  --network testnet \
  -- is_expired

# Test SweepController.get_nonce()
stellar contract invoke \
  --id "$SWEEP_ID" \
  --source "$SIGNER_SECRET_KEY" \
  --network testnet \
  -- get_nonce
```

### 4. Record Contract IDs

After successful deployment, update the SDK configuration:

```
EPHEMERAL_ACCOUNT_CONTRACT_ID=<from deployment output>
SWEEP_CONTROLLER_CONTRACT_ID=<from deployment output>
```

These IDs should be shared with the SDK team for integration testing.

## Contract Initialization Parameters

### EphemeralAccount
- `creator`: Deployer address
- `expiry_ledger`: Current ledger + 10,000 (approximately 14 hours)
- `recovery_address`: Organization's recovery wallet
- `authorized_controller`: SweepController contract address
- `admin`: Deployer address (for upgrades)

### SweepController
- `creator`: Deployer address
- `authorized_signer`: Ed25519 public key for off-chain sweep authorization
- `authorized_destination`: `None` for flexible mode, or a specific address for locked mode

## Post-Deployment Verification Checklist

- [ ] Both contracts deployed and contract IDs recorded
- [ ] SweepController initialized with correct authorized signer
- [ ] EphemeralAccount template deployed (for factory use)
- [ ] Basic smoke tests pass via CLI
- [ ] Contract IDs shared with SDK team
- [ ] Deployment record saved in `deployments/testnet.json`

## Rollback / Recovery

If a deployment fails or produces incorrect results:

1. The deployer account retains admin rights over all contracts
2. Use `upgrade()` on each contract if a WASM fix is needed
3. For catastrophic failures, the recovery address on each
   EphemeralAccount can reclaim funds after expiry
