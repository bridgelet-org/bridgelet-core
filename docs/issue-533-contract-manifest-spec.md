# Machine-Readable Contract Manifest Spec

Closes #533.

## Problem

`bridgelet-sdk` and the frontend both need a reliable way to know which
contract addresses and interface (spec/ABI) versions are live per network
(testnet/futurenet/mainnet). No single manifest is the source of truth today,
short of the not-yet-deployed on-chain `MigrationCoordinator` contract.

## Proposed manifest: `manifest.json`

Generated at the repo root by a new `scripts/publish-manifest.sh`, run as
the final step of the deployment flow (alongside `scripts/deploy-testnet.sh`):

```json
{
  "generated_at": "2026-08-27T00:00:00Z",
  "networks": {
    "testnet": {
      "reserve_contract": {
        "address": "C...",
        "wasm_hash": "sha256:...",
        "spec_version": "0.1.0"
      }
    },
    "mainnet": { "...": "same shape, only contracts actually deployed" }
  }
}
```

- One entry per deployed contract per network; a contract only appears once
  it is actually live there.
- `wasm_hash` is the same sha256 that `scripts/verify-deployment.sh` (#532)
  independently checks against the on-chain bytecode, not hand-typed.
- `spec_version` comes from each contract's exported spec (soroban-cli's
  `contract inspect` / XDR spec entries), not `Cargo.toml`, since those drift.

## Generation, not hand-maintenance

`scripts/publish-manifest.sh` runs automatically at the end of
`scripts/deploy-testnet.sh` (and any future mainnet deploy script), so
`manifest.json` is always regenerated as a byproduct of deployment.

## Consumers

`bridgelet-sdk` and the frontend/coordination repo's compatibility-matrix
work (already proposed there) both read `manifest.json` directly instead of
each maintaining their own copy of contract addresses and versions.
