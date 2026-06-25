# Error Catalogue

This document lists every contract error variant for the `EphemeralAccount` and `SweepController` contracts.

## EphemeralAccount Contract Errors

The `EphemeralAccount` contract defines the following error codes in `contracts/ephemeral_account/src/errors.rs`.

| Code | Name | Meaning |
| --- | --- | --- |
| 1 | `AlreadyInitialized` | The contract was already initialized. |
| 2 | `NotInitialized` | The contract has not been initialized yet. |
| 3 | `PaymentAlreadyReceived` | Deprecated alias; payments are now tracked by asset. |
| 4 | `InvalidAmount` | The payment amount is zero or negative. |
| 5 | `InvalidExpiry` | The expiry ledger is not in the future. |
| 6 | `NotExpired` | Attempted to expire before the expiry ledger. |
| 7 | `AlreadySwept` | A sweep has already been executed. |
| 8 | `Unauthorized` | Authorization failed during sweep validation. |
| 9 | `InvalidSignature` | The provided signature is malformed or invalid. |
| 10 | `NoPaymentReceived` | No payment has been recorded yet. |
| 11 | `AccountExpired` | The account has already expired and cannot be swept. |
| 12 | `InvalidStatus` | The requested operation is invalid for the current account status. |
| 13 | `DuplicateAsset` | A payment for this asset already exists. |
| 14 | `TooManyPayments` | The maximum supported payment count (10) has been exceeded. |

### Notes
- `DuplicateAsset` replaces the earlier single-payment error handling model and supports multiple assets per account.
- `TooManyPayments` protects against gas and storage exhaustion by limiting the number of distinct assets.

## SweepController Contract Errors

The `SweepController` contract defines the following error codes in `contracts/sweep_controller/src/errors.rs`.

| Code | Name | Meaning |
| --- | --- | --- |
| 1 | `InvalidAccount` | The provided account is not in a valid state for sweeping. |
| 2 | `TransferFailed` | The token transfer step failed during sweep execution. |
| 3 | `AuthorizationFailed` | Signature verification or initialization failed. |
| 4 | `InsufficientBalance` | The source account does not have enough balance. |
| 5 | `AccountNotReady` | The account is not ready for sweep (no payment or invalid state). |
| 6 | `AccountExpired` | The account has already expired. |
| 7 | `AccountAlreadySwept` | The sweep operation has already been completed. |
| 8 | `InvalidSignature` | The signature format is invalid. |
| 9 | `SignatureVerificationFailed` | Ed25519 signature verification failed. |
| 10 | `AuthorizedSignerNotSet` | The controller has no authorized signer configured. |
| 11 | `InvalidNonce` | The sweep nonce is invalid or replayed. |
| 13 | `UnauthorizedDestination` | The destination does not match the locked authorized destination. |

### Notes
- `AuthorizedSignerNotSet` indicates the contract was never initialized with an authorized signer.
- `UnauthorizedDestination` only applies when a destination lock is set during initialization.
