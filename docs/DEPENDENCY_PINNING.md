# Dependency Pinning: `soroban-sdk`

Issue #42.

## Policy

Every workspace member builds against the **same** `soroban-sdk` version.
The version is pinned once, at the workspace root, and members inherit it —
no member declares its own range.

- Root `Cargo.toml`:

  ```toml
  [workspace.dependencies]
  soroban-sdk = "22.0.0"
  ```

- Each contract (`shared`, `ephemeral_account`, `sweep_controller`,
  `reserve_contract`, `account_factory`):

  ```toml
  [dependencies]
  soroban-sdk = { workspace = true }

  [dev-dependencies]
  soroban-sdk = { workspace = true, features = ["testutils"] }
  ```

Two dependencies live outside the workspace inheritance and are pinned
directly to the same version, so they must be bumped together with the root:

- `contracts/sweep_controller/Cargo.toml` → `soroban-token-sdk = "22.0.0"`
- `tools/sweep-signer/Cargo.toml` → `soroban-sdk = "22.0.0"` (standalone
  workspace; its `Address::to_xdr()` output must match the deployed contracts,
  so it has to track the same version)

## No open ranges

The current pin is `22.0.0`. Do not widen it to a range (`>=`, `^` with a
looser floor, `*`, etc.): a silent minor bump can change host behaviour,
generated event schemas, or XDR serialization between the contract and the
off-chain signer.

## Upgrade process

Bumping `soroban-sdk` is a coordinated change, not a routine dependency update:

1. Change `soroban-sdk` in the root `[workspace.dependencies]`.
2. Change `soroban-token-sdk` (sweep_controller) and the `tools/sweep-signer`
   pin to the matching version in the same commit.
3. `cargo update -p soroban-sdk` and commit the updated `Cargo.lock`.
4. `stellar contract build` and run the full test suite locally.
5. **Deploy to testnet and exercise the full sweep flow end to end** before
   promoting the bump — verify signatures produced by `tools/sweep-signer`
   still verify on-chain, since XDR serialization is version-sensitive.
6. Only then merge and roll out to mainnet.
