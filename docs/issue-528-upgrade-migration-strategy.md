# Contract Upgrade / Migration Strategy

Closes #528. Documents how contracts in *this* repo are actually upgraded today, based on what's wired in source — not an abstract policy. This is the source of truth `bridgelet-sdk`'s equivalent doc should reference.

## Current state: mixed, not a single policy

Grepping `contracts/*/src/lib.rs` for `update_current_contract_wasm` (the Soroban host call that swaps a deployed contract's executable in place) shows exactly **one** contract wires it to a callable entrypoint:

- `ephemeral_account::upgrade(env, new_wasm_hash)` — gated by `storage::get_admin(&env)` + `admin.require_auth()`, returns `Error::NotUpgradeAdmin` (1014) if no admin is set. It calls `env.deployer().update_current_contract_wasm(new_wasm_hash)` directly with **no version check or migration step** — the new WASM starts running against the old instance storage immediately.
- `sweep_controller` has a *storage schema* migration framework (`src/migration.rs`: `StorageVersion { major, minor, patch }`, `migrate()` run automatically from `initialize()` and also exposed as a public entrypoint) but **no `upgrade()` entrypoint** — no way to swap this contract's WASM in place today.
- Every other contract (`reserve_contract`, `timelock_controller`, `multisig_approval`, `fee_splitter`, `nonce_registry`, `allowlist_registry`, `audit_log`, `version_registry`, `asset_allowlist`, `access_controller`, `compliance_oracle`, `claimable_balance_registry`, `expiry_scheduler`, `notification_registry`, plus non-CI contracts) has neither mechanism: **new-deployment-only by default**.

## Implication for in-flight state

**In-flight ephemeral accounts:** because `upgrade()` has no migration step, a new WASM must keep every `DataKey` variant it still reads/writes byte-compatible — reordering or removing variants (`Initialized, Creator, ExpiryLedger, ...`) after accounts are live would desynchronize decoding for in-flight instances. Adding trailing variants is safe; anything else needs a `sweep_controller`-style version-tagged migration step, which `ephemeral_account::upgrade()` currently lacks — **this is a gap**.

**Active sweeps:** since `sweep_controller` has no in-place upgrade path, changing sweep logic requires deploying a new instance and re-pointing affected `ephemeral_account` instances at it. Each ephemeral account stores its controller as `DataKey::AuthorizedController` (an `Address` set at `initialize()`, not hardcoded), so redirecting is conceptually possible but **is not exposed as a callable entrypoint** on `ephemeral_account` (no `set_authorized_controller` in `lib.rs`) — accounts created before a redeploy stay bound to the old controller until that gap is closed.

## Recommendation

1. Treat `ephemeral_account::upgrade()` plus a new version-migration hook (adopting `sweep_controller::migration`'s pattern) as the template for any contract needing live upgradability.
2. All other contracts remain new-deployment-only; document this explicitly per contract.
3. Add an admin-gated `set_authorized_controller` to `ephemeral_account` for controller migration without full account redeploy.
