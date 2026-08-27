# Cross-network contract ID consistency validation

Closes #549.

## Current state

`scripts/deploy-testnet.sh` is the only deployment script in the repo (the
issue references `scripts/deploy-contracts.sh` / `verify-deployment.sh`,
which do not yet exist). It hardcodes `NETWORK="testnet"` and
`NETWORK_PASSPHRASE`, deploys `EphemeralAccount` then `SweepController`, and
initializes `SweepController` inline — but nothing validates that any
cross-contract address passed as a constructor/init argument was itself
deployed against the same `NETWORK_PASSPHRASE`. Note `EphemeralAccount`
already enforces its own passphrase at `initialize` time via
`bridgelet_shared::passphrase::require_network`, so a single-contract
mismatch already fails; the gap is specifically *composed* deployments where
one contract's ID is passed as configuration into another's constructor
(e.g. a `reserve_contract` or future `asset_allowlist` address).

## Proposed design

- Each per-network deployment record (`deployments/<network>.json`, already
  produced by `deploy-testnet.sh`) is tagged with its `network` field, as
  today.
- Add a `scripts/validate-cross-network.sh` helper: before any deploy step
  that consumes a *previously deployed* contract ID as a constructor
  argument, look up that ID's origin deployment record and assert its
  `network` field matches the current `$NETWORK`. Abort with a non-zero exit
  and a clear message on mismatch.
- Wire this validation into `deploy-testnet.sh` (and any future
  `deploy-mainnet.sh` / `deploy-futurenet.sh`) immediately before each
  `stellar contract invoke ... initialize` call that references another
  contract's address.
- Add a `--dry-run` flag that runs only the validation step against an
  existing `deployments/*.json` set without submitting transactions, so the
  mismatch path is exercisable in CI without live network access.

## Test coverage required

- Dry-run against two fixture deployment records on the same network passes.
- Dry-run against fixture records from different networks fails with a
  non-zero exit and a descriptive error identifying the mismatched contract.
