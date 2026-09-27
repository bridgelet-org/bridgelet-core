# Simulating Batch Account Creation

`account_factory` is not currently available to integrators on the target testnet. Integrators that need to create multiple ephemeral accounts can simulate batch account creation by deploying and initializing individual `EphemeralAccount` contracts in a loop.

This example shows the pattern.

## Overview

Instead of calling `account_factory`:

```text
AccountFactory
    ├── EphemeralAccount 1
    ├── EphemeralAccount 2
    ├── EphemeralAccount 3
    └── ...
```

create each `EphemeralAccount` individually:

```text
Loop
 ├── Deploy EphemeralAccount
 ├── Initialize EphemeralAccount
 ├── Record contract ID
 │
 ├── Deploy EphemeralAccount
 ├── Initialize EphemeralAccount
 ├── Record contract ID
 │
 └── ...
```

This is not an atomic batch transaction. Each account deployment and initialization is a separate operation.

## Prerequisites

Make sure you have:

- Rust installed
- Stellar CLI installed and configured for testnet
- A funded Stellar testnet identity
- The `EphemeralAccount` WASM built

From the repository root, build the contracts:

```bash
./scripts/build.sh
```

The `EphemeralAccount` WASM is produced at:

```text
target/wasm32v1-none/release/ephemeral_account.wasm
```

For identity setup, see
`testnet/config/soroban-cli-identity-setup.md`.

## Simulating Batch Creation

The following shell example deploys and initializes one `EphemeralAccount` at a time.

Set the required values before running it:

```bash
export STELLAR_IDENTITY="testnet-identity"
export CREATOR_ADDRESS="YOUR_CREATOR_ADDRESS"
export RECOVERY_ADDRESS="YOUR_RECOVERY_ADDRESS"
export AUTHORIZED_CONTROLLER="YOUR_CONTROLLER_ADDRESS"
export ADMIN_ADDRESS="YOUR_ADMIN_ADDRESS"
export EXPIRY_LEDGER="YOUR_EXPIRY_LEDGER"
```

Then run:

```bash
#!/usr/bin/env bash

set -euo pipefail

COUNT=3
WASM_PATH="target/wasm32v1-none/release/ephemeral_account.wasm"

: "${STELLAR_IDENTITY:?STELLAR_IDENTITY must be set}"
: "${CREATOR_ADDRESS:?CREATOR_ADDRESS must be set}"
: "${RECOVERY_ADDRESS:?RECOVERY_ADDRESS must be set}"
: "${AUTHORIZED_CONTROLLER:?AUTHORIZED_CONTROLLER must be set}"
: "${ADMIN_ADDRESS:?ADMIN_ADDRESS must be set}"
: "${EXPIRY_LEDGER:?EXPIRY_LEDGER must be set}"

for i in $(seq 1 "$COUNT"); do
  echo "Creating ephemeral account $i/$COUNT..."

  ACCOUNT_ID=$(stellar contract deploy \
    --wasm "$WASM_PATH" \
    --source "$STELLAR_IDENTITY" \
    --network testnet)

  echo "Created account: $ACCOUNT_ID"

  stellar contract invoke \
    --id "$ACCOUNT_ID" \
    --source "$STELLAR_IDENTITY" \
    --network testnet \
    -- \
    initialize \
    --creator "$CREATOR_ADDRESS" \
    --expiry_ledger "$EXPIRY_LEDGER" \
    --recovery_address "$RECOVERY_ADDRESS" \
    --authorized_controller "$AUTHORIZED_CONTROLLER" \
    --admin "$ADMIN_ADDRESS"

  echo "Initialized account $i: $ACCOUNT_ID"
done
```

The `initialize` arguments correspond to the `EphemeralAccount` initialization flow documented for testnet interactions.

## Important Difference From `account_factory`

This approach is only a simulation of batch account creation.

Each account is deployed and initialized independently. Therefore:

- the operations are not atomic;
- a later account can fail after earlier accounts have already been created;
- there is no single transaction containing all account creations;
- callers should record successful contract IDs;
- callers should handle partial failures and retries.

When `account_factory` is available on the target testnet, integrators can replace this loop with the factory-based batch operation.

## Example Output

A successful run could look like:

```text
Creating ephemeral account 1/3...
Created account: CAAA...
Initialized account 1: CAAA...

Creating ephemeral account 2/3...
Created account: CBBB...
Initialized account 2: CBBB...

Creating ephemeral account 3/3...
Created account: CCCC...
Initialized account 3: CCCC...
```

The resulting contract IDs can then be recorded and used by the integrator as individual ephemeral accounts.

## When to Use This

Use this approach when:

- `account_factory` is not available on the target testnet;
- you need to create multiple ephemeral accounts;
- atomic batch creation is not required;
- you want to test an integration before `account_factory` is available.

Once `account_factory` is available on the target testnet, migrate to the factory-based flow for actual batch creation.