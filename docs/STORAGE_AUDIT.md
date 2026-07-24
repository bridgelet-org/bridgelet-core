# SweepController Storage Key Audit

Issue #25.

## Scope

Review `contracts/sweep_controller/src/storage.rs` for storage-key naming and
confirm there is no risk of key collisions — either between multiple deployed
instances of the contract, or between keys within a single instance.

## Keys

All keys are variants of a single `#[contracttype]` enum, `DataKey`:

| Variant                 | Value type    | Access        |
| ----------------------- | ------------- | ------------- |
| `AuthorizedSigner`      | `BytesN<32>`  | instance      |
| `SweepNonce`            | `u64`         | instance      |
| `AuthorizedDestination` | `Address`     | instance      |
| `Creator`               | `Address`     | instance      |

Every read and write goes through `env.storage().instance()`.

## Findings

### Cross-instance collisions — not possible

Soroban scopes instance (and persistent/temporary) storage to the individual
deployed contract instance. The host keys every entry by the contract's own
address, so two separate deployments of `SweepController` have completely
disjoint storage. A manual contract-address prefix on the keys would be
redundant — the isolation is provided by the platform, not by the key name.

### Intra-instance collisions — not possible

Within one instance, each `DataKey` enum variant serialises to a distinct
key. The four variants are unique and carry no overlapping payload, so no two
logical values map to the same key.

## Conclusion

No changes to the key layout are required. The audit is recorded inline as
module-level rustdoc in `storage.rs` so the reasoning stays next to the code.
If future keys are added, keep them as distinct `DataKey` variants (rather than
raw symbols/strings) to preserve this guarantee.
