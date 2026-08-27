# Storage Key Namespacing Convention

Closes #527. Documents the storage-key pattern used by every contract under `contracts/*/src/storage.rs`, generalizing the single-contract audit already recorded in `docs/STORAGE_AUDIT.md` (Issue #25, `sweep_controller` only) to the whole repo.

## The convention

Every contract defines one `#[contracttype] pub enum DataKey { ... }` in its `storage.rs`, read/written mainly through `env.storage().instance()`. Several contracts (`allowlist_registry`, `audit_log`, `escrow_vault`, `notification_registry`, `multisig_approval`, `expiry_scheduler`, `claimable_balance_registry`, `compliance_oracle`, `metrics_aggregator`) also use `.persistent()` for entries that must outlive instance-TTL eviction, e.g. `audit_log::DataKey::Entry(u64)`.

Two namespacing layers apply:

1. **Cross-instance**: Soroban's host scopes all storage to the deployed contract's own address. Two deployments of the same contract, or two different contracts, can never collide — enforced by the platform, not by key naming. No contract here prefixes keys with its own contract ID.
2. **Intra-instance**: each `DataKey` variant serializes to a distinct key. Parameterized variants carry disambiguating data as enum payload, not string concatenation, e.g. `nonce_registry::DataKey::Consumed(Address, u64)` (per signer+nonce), `multisig_approval::DataKey::Approved(u64, Address)` (per proposal+signer), `access_controller::DataKey::Role(Symbol, Address)` (per role+address).

## Per-contract key inventory (representative)

| Contract | Key variants |
|---|---|
| ephemeral_account | `Initialized, Creator, ExpiryLedger, RecoveryAddress, Payments, Status, SweptTo, BaseReserveRemaining, AvailableReserve, ReserveReclaimed, LastSweepId, ReserveEventCount, LastReserveEvent, AuthorizedController, AuthorizedSigner, Admin` |
| sweep_controller | `AuthorizedSigner, SweepNonce, AuthorizedDestination, Creator, PendingSigner, PendingSignerEffectiveLedger, StorageVersion` |
| reserve_contract | `BaseReserve, Admin` |
| access_controller | `SuperAdmin, Role(Symbol, Address)` |
| audit_log | `Admin, Counter, Writer(Address), Entry(u64)` |
| pause_guardian | `Guardians, Threshold, Paused(Symbol), PauseApproval(Symbol, Address), UnpauseApproval(Symbol, Address)` |

No shared or cross-contract storage access exists in `contracts/`: every read/write goes through the owning contract's own `storage.rs`; cross-contract effects happen only via typed calls (e.g. `ephemeral_account` calling `reserve_contract`), never by reading another contract's raw storage.

## Confirmation: no collision risk

Given platform-level per-instance isolation plus verified enum-variant key uniqueness per contract, no two contracts — and no two keys within one contract — can collide today. `sweep_controller` additionally tracks a `StorageVersion` key (see `docs/issue-528-upgrade-migration-strategy.md`) for explicit layout-change tracking; other contracts haven't adopted that pattern yet.
