# Reproducible Build Verification

Closes #532.

## Problem

Nothing in this repo confirms that the WASM hash live on testnet/mainnet for
a given contract corresponds to an auditable commit here. For a project
whose deployed bytecode controls real fund custody (e.g. `reserve_contract`,
`escrow_vault`, `sweep_controller`), that is a supply-chain integrity gap.

## Proposed script: `scripts/verify-deployment.sh`

```
Usage: scripts/verify-deployment.sh <contract> <commit-or-tag> <network>

1. git worktree add against <commit-or-tag> in a clean temp dir
2. cd contracts/<contract> && cargo build --target wasm32v1-none --release
   (pinned toolchain 1.89.0, matching .github/workflows/test.yml)
3. sha256sum the resulting .wasm
4. soroban contract fetch --id <deployed-address> --network <network>
   to pull the on-chain WASM, then sha256sum it
5. Diff the two hashes; exit 0 only on exact match, printing both hashes
   and both shas either way
```

Because `cargo build` is deterministic for a pinned toolchain/target/profile
and this repo already checks in `Cargo.lock` per contract, rebuilding at a
specific commit should reproduce byte-identical output.

## Process requirement

This script must be run, and must pass, before any deployment is marked
"confirmed good" in release notes or in the deployment manifest proposed in
#533 — i.e. it is the verification step that manifest publishing depends on,
not an optional afterthought.

## Relationship to #533 and version-registry

The manifest in #533 records the WASM hash claimed for each network; this
script is how that claim gets independently checked against both the
on-chain bytecode and the `version_registry` / `contracts/version_registry`
contract's recorded hash before anyone trusts either.
