# Semantic versioning scheme for contract interfaces

Closes #551.

## Current state

Every contract's `Cargo.toml` (`contracts/*/Cargo.toml`) is pinned at
`version = "0.1.0"` uniformly — this is the crate version, not an interface
version, and gives `bridgelet-sdk` / the frontend no signal about
compatibility. `VersionRegistry` (`contracts/version_registry/src/lib.rs`)
already exists: an on-chain, admin-gated registry mapping a contract name to
a `publish`-ed WASM hash plus a version label, with `current`/`history`
lookups. It is a natural home for interface version, but nothing currently
defines what the label means.

## Proposed policy

Adopt semver (`MAJOR.MINOR.PATCH`) applied specifically to the exported
`#[contractimpl]` interface (function names, argument types/order, return
types), independent of the crate's Cargo.toml version:

- **MAJOR** — removing/renaming an exported function, changing an argument
  type or order, changing a return type, or changing error variant numeric
  values that clients match on.
- **MINOR** — adding a new exported function, or adding a new field to a
  struct/enum returned by an existing function in an additive (non-breaking)
  way.
- **PATCH** — internal logic, storage, or gas/instruction-cost changes with
  no change to the exported interface surface.

## Recording the version

- Each contract's interface version is published to `VersionRegistry` via
  `publish(contract_name, wasm_hash, version_label)` at deploy time,
  alongside the existing WASM hash tracking — no new storage primitive
  needed, only a convention for what `version_label` contains (a semver
  string per the policy above, not the Cargo.toml crate version).
- `VersionRegistry::current` becomes the machine-readable source of truth
  `bridgelet-sdk` and the frontend query before trusting a deployed
  contract's interface.

## Documentation

This policy should be copied into `CONTRIBUTING.md` (not modified as part of
this docs-only change) so future contract interface changes are classified
and versioned consistently before merge.
