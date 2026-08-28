# Storage Snapshot / Regression Test Plan

Closes #517.

## Why this matters here specifically

`contracts/sweep_controller/src/migration.rs` already implements a
versioned migration scheme: `StorageVersion { major, minor, patch }`
packed into a `u32`, stored under `DataKey::StorageVersion`, with
`migrate()` comparing against `CURRENT_VERSION` (currently `1.0.0`) and
running pending migration steps. This is exactly the mechanism a storage
snapshot test must protect — an accidental change to `DataKey` variants,
field order, or types in `storage.rs` for any contract would silently
break already-deployed instances even though `migrate()` "runs" without
error.

## Snapshot targets

One snapshot per contract's `DataKey` enum plus every `#[contracttype]`
struct persisted under it, e.g.:

- `reserve_contract::storage::DataKey` (`BaseReserve`, `Admin`) and the
  bare `i128`/`Address` shapes stored under them.
- `sweep_controller::storage` keys (authorized signer, destination,
  creator, sweep nonce, pending signer + effective ledger,
  `StorageVersion`).
- `sweep_controller::migration::StorageVersion` (`major: u32, minor: u32,
  patch: u32`) — its `packed()` layout is itself a candidate for a
  dedicated regression test independent of the general snapshot, since two
  different field values can collide if a field ever exceeds the 8-bit
  shift width silently.

## Mechanism

`contracts/*/tests/storage_snapshot.rs` (new, not added in this doc-only
PR) captures the XDR-serialized byte shape of each `DataKey` variant and
its associated value type, asserting it against a checked-in golden file
per contract. A future PR that changes a key's shape without updating the
golden file fails CI with a clear diff.

## Deliberate-update process

When a storage change is intentional, the PR must: (1) regenerate the
golden snapshot, (2) bump `CURRENT_VERSION` in `migration.rs` (or the
per-contract equivalent) and add a corresponding `migrate_vX` step, (3)
link the migration plan in the PR description. A snapshot diff with no
accompanying version bump is treated as a CI failure, not a rubber-stamp.

Cross-reference with the contract-upgrades ADR already proposed for
bridgelet-sdk's docs to keep both stories consistent.
