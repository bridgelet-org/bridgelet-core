# soroban-sdk Version Policy (Issue #42)

## Single pinned version

`soroban-sdk` is pinned in exactly one place — the root `Cargo.toml`
`[workspace.dependencies]` table:

```toml
[workspace.dependencies]
soroban-sdk = "22.0.0"
```

Every workspace member (`ephemeral_account`, `sweep_controller`, `shared`,
`reserve_contract`, `account_factory`) consumes it by inheritance, so no
individual crate declares its own version:

```toml
# in each contract Cargo.toml
[dependencies]
soroban-sdk = { workspace = true }

[dev-dependencies]
soroban-sdk = { workspace = true, features = ["testutils"] }
```

This guarantees all contracts, the shared crate, and the test/`testutils`
builds compile against **one identical** SDK version. Changing the version in
the workspace table changes it everywhere at once.

## Upgrade process

Do **not** bump the pin casually. Follow these steps for any SDK upgrade:

1. Open a dedicated branch for the upgrade — never bundle it with feature work.
2. Update `soroban-sdk` in the root `Cargo.toml` `[workspace.dependencies]`
   only. Run `cargo update -p soroban-sdk` to refresh `Cargo.lock`.
3. Build every contract: `stellar contract build`.
4. Run the full suite: `cargo test` in every contract, including the
   `sweep_controller` integration tests.
5. Run `cargo fmt -- --check` and `cargo clippy -- -D warnings` per contract.
6. **Deploy the built WASM to testnet and exercise the full lifecycle**
   (initialize → record payment → sweep/claim → expire/recover) before the
   change is allowed to reach `main`. A green local build is not sufficient
   evidence for an SDK bump.
7. Note the SDK version and testnet contract IDs used for verification in the
   PR description so the upgrade is auditable.

## Rationale

The SDK defines the host ABI, serialization, and auth semantics the contracts
rely on. A silent minor bump can change generated WASM and on-chain behaviour,
so the version is centralised and every bump is gated on an explicit testnet
verification step.
