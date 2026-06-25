# Storage Layout

This document details the Soroban instance storage layout for the `EphemeralAccount` and `SweepController` contracts.

## EphemeralAccount Storage Layout

The `EphemeralAccount` contract stores all state in Soroban instance storage. The current keys are defined in `contracts/ephemeral_account/src/storage.rs`.

| Storage Key | Type | Purpose | Set By | Read By |
| --- | --- | --- | --- | --- |
| `Initialized` | `bool` | Tracks whether the contract has been initialized. | `initialize()` | All entry points |
| `Creator` | `Address` | Creator address of the ephemeral account. | `initialize()` | `get_info()`, audit logic |
| `ExpiryLedger` | `u32` | Ledger at which the account expires. | `initialize()` | `is_expired()`, `expire()` |
| `RecoveryAddress` | `Address` | Address to recover funds after expiry. | `initialize()` | `expire()`, `get_info()` |
| `Payments` | `Map<Address, Payment>` | Recorded payments keyed by asset address. Supports multiple assets. | `record_payment()` | `get_info()`, `sweep()`, `execute_transfers()` |
| `Status` | `AccountStatus` | Current account lifecycle state. | `initialize()`, `record_payment()`, `sweep()`, `expire()` | `get_status()`, state checks |
| `SweptTo` | `Address` | Destination address used for the sweep. | `sweep()` | `get_info()` |
| `BaseReserveRemaining` | `i128` | Remaining base reserve in stroops. | `init_reserve_tracking()`, reclaim logic | Reserve management |
| `AvailableReserve` | `i128` | Available reserve balance for reclaiming. | `init_reserve_tracking()` | Reserve management |
| `ReserveReclaimed` | `bool` | Indicates whether reserve reclamation is complete. | `init_reserve_tracking()` | Reserve management |
| `LastSweepId` | `u64` | Ledger-based sweep identifier. | `sweep()` | Audit and event tracking |
| `ReserveEventCount` | `u32` | Number of reserve reclaim events recorded. | `init_reserve_tracking()` | Audit and event tracking |
| `LastReserveEvent` | `ReserveReclaimed` | Details of the last reserve reclaim event. | Reserve reclaim logic | Audit and event tracking |
| `AuthorizedController` | `Address` | Controller authorized to manage the account. | `initialize()` | Authorization logic |
| `ContractVersion` | `u32` | Version of the contract at initialization. | `initialize()` | `version()` |

### Notes
- `Payments` uses a Soroban `Map` keyed by asset address, allowing multiple asset payments while preventing duplicate assets.
- Base reserve fields are initialized during account creation and updated during sweep/expiry operations.
- `Status` is the primary state machine driver for the account lifecycle.

## SweepController Storage Layout

The `SweepController` contract stores its authorization state in instance storage.

| Storage Key | Type | Purpose | Set By | Read By |
| --- | --- | --- | --- | --- |
| `AuthorizedSigner` | `BytesN<32>` | Ed25519 public key used for sweep authorization. | `initialize()` | `execute_sweep()`, `AuthContext::verify()` |
| `SweepNonce` | `u64` | Monotonic nonce to prevent replay attacks. | `init_sweep_nonce()`, `increment_nonce()` | `authorization::verify_sweep_auth()`, `execute_sweep()` |
| `AuthorizedDestination` | `Option<Address>` | Optional destination lock for sweeps. | `initialize()`, `update_authorized_destination()` | `execute_sweep()` |
| `Creator` | `Address` | Address that initialized the sweep controller. | `initialize()` | `update_authorized_destination()` |

### Notes
- `SweepNonce` must increase after each successful sweep authorization.
- `AuthorizedDestination` is optional. When set, all sweep requests must target the locked destination.
- The controller separates on-chain authorization from the `EphemeralAccount` contract's state logic.
