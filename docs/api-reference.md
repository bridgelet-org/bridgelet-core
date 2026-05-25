# Bridgelet-Core API Reference

## EphemeralAccount Contract

Implements a single-use account that can accept one or multiple payments and must be swept or expired.

### Functions

#### `initialize`
Initializes the contract with ownership and expiry rules. Can only be called once.

```rust
fn initialize(
    env: Env,
    creator: Address,
    expiry_ledger: u32,
    recovery_address: Address,
) -> Result<(), Error>
```

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `creator` | `Address` | The account that created this contract. |
| `expiry_ledger` | `u32` | The ledger sequence number at which the account expires. |
| `recovery_address` | `Address` | Where funds are sent if the account expires. |

#### `record_payment`
Records an inbound payment. Supports multiple payments of different assets.

```rust
fn record_payment(
    env: Env, 
    amount: i128, 
    asset: Address
) -> Result<(), Error>
```

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `amount` | `i128` | The amount of the payment. Must be positive. |
| `asset` | `Address` | The address of the asset contract (token). |

#### `sweep`
Authorizes a transfer of all assets to the destination and updates the account state to `Swept`.

```rust
fn sweep(
    env: Env,
    destination: Address,
    auth_signature: BytesN<64>
) -> Result<(), Error>
```

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `destination` | `Address` | The recipient address for the funds. |
| `auth_signature` | `BytesN<64>` | Off-chain signature authorizing the weep. |

#### `expire`
Expire the account and return funds to the recovery address. Can only be called after `expiry_ledger`.

```rust
fn expire(env: Env) -> Result<(), Error>
```

#### `is_expired`
Checks if the account has passed its expiry ledger.

```rust
fn is_expired(env: Env) -> bool
```

#### `get_status`
Returns the current status of the account (Active, PaymentReceived, Swept, Expired).

```rust
fn get_status(env: Env) -> AccountStatus
```

#### `get_info`
Returns the full state of the account.

```rust
fn get_info(env: Env) -> Result<AccountInfo, Error>
```

Returns `AccountInfo`:
```rust
struct AccountInfo {
    creator: Address,
    status: AccountStatus,
    expiry_ledger: u32,
    recovery_address: Address,
    payment_received: bool,
    payment_count: u32,
    payments: Vec<Payment>,
    swept_to: Option<Address>,
}
```

Where `Payment` is defined as:
```rust
struct Payment {
    asset: Address,
    amount: i128,
    timestamp: u64,
}
```

### Events

| Event | Data Structure | Trigger |
| :--- | :--- | :--- |
| `created` | `AccountCreated { creator, expiry_ledger }` | `initialize` success. |
| `payment` | `PaymentReceived { amount, asset }` | First `record_payment`. |
| `multi_pay` | `MultiPaymentReceived { asset, amount }` | Subsequent `record_payment` calls. |
| `swept_mul` | `SweepExecutedMulti { destination, payments }` | `sweep` success. |
| `expired` | `AccountExpired { recovery_address, amount_returned }` | `expire` success. |

### Error Codes

| Code | Name | Description |
| :--- | :--- | :--- |
| 1 | `AlreadyInitialized` | Contract already initialized. |
| 2 | `NotInitialized` | Contract not initialized. |
| 3 | `PaymentAlreadyReceived` | Deprecated. Replaced by `DuplicateAsset` |
| 4 | `InvalidAmount` | Payment amount is zero or negative. |
| 5 | `InvalidExpiry` | Expiry ledger is in the past. |
| 6 | `NotExpired` | Attempted to expire before expiry ledger. |
| 7 | `AlreadySwept` | Account already swept. |
| 8 | `Unauthorized` | Signature verification failed. |
| 9 | `InvalidSignature` | Cryptographic signature is invalid. |
| 10 | `NoPaymentReceived` | Cannot sweep without funds. |
| 11 | `AccountExpired` | Cannot sweep, account is expired. |
| 12 | `InvalidStatus` | Action invalid for current status. |
| 13 | `DuplicateAsset` | Asset already has a recorded payment. |
| 14 | `TooManyPayments` | Max payment limit (10) reached. |

---

## SweepController Contract

Orchestrates the sweeping process by verifying authorization signatures.

### Functions

#### `initialize`
Sets the authorized signer for the controller.

```rust
fn initialize(
    env: Env, 
    authorized_signer: BytesN<32>
) -> Result<(), Error>
```

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `authorized_signer` | `BytesN<32>` | Ed25519 public key for verifying sweep signatures. |

#### `execute_sweep`
Verifies authorization and triggers the sweep on the ephemeral account.

```rust
fn execute_sweep(
    env: Env,
    ephemeral_account: Address,
    destination: Address,
    auth_signature: BytesN<64>
) -> Result<(), Error>
```

#### `can_sweep`
Checks if an account is in a valid state to be swept.

```rust
fn can_sweep(env: Env, ephemeral_account: Address) -> bool
```

### Events

| Event | Data Structure | Trigger |
| :--- | :--- | :--- |
| `sweep` | `SweepCompleted { ephemeral_account, destination, amount }` | `execute_sweep` success. |

### Error Codes

| Code | Name | Description |
| :--- | :--- | :--- |
| 1 | `InvalidAccount` | Account not in valid state. |
| 2 | `TransferFailed` | Not yet implemented |
| 3 | `AuthorizationFailed` | Signature invalid or signer not set. |
| 4 | `InsufficientBalance` | Not yet implemented |
| 5 | `AccountNotReady` | Account has no payments or is not ready. |
| 6 | `AccountExpired` | Account has expired. |
| 7 | `AccountAlreadySwept` | Account has already been swept. |
| 8 | `InvalidSignature` | Signature format is invalid. |
| 9 | `SignatureVerificationFailed` | Crypto verification failure. |
| 10 | `AuthorizedSignerNotSet` | Controller not initialized with signer. |
| 11 | `InvalidNonce` | Security nonce is invalid. |

---

## Usage Examples

### Rust SDK Integration

```rust
use soroban_sdk::{Address, BytesN, Env};
use ephemeral_account::{Client as EphemeralClient};

fn example_flow(env: &Env, contract_id: &Address) {
    let client = EphemeralClient::new(env, contract_id);
    
    // 1. Initialize
    client.initialize(
        &creator_addr, 
        &(env.ledger().sequence() + 1000), 
        &recovery_addr
    );

    // 2. Record Payment (called by watcher)
    client.record_payment(&100_000, &usdc_addr);
    
    // 3. Sweep
    // Signature generated off-chain using the SweepController's authorized key
    let signature = BytesN::from_array(env, &[/* 64 bytes */]); 
    client.sweep(&destination_addr, &signature);
}
```

### CLI Invocation

**Initialize Account:**
```bash
soroban contract invoke \
    --id C... \
    --network testnet \
    --source S... \
    -- \
    initialize \
    --creator G... \
    --expiry_ledger 123456 \
    --recovery_address G...
```

**Record Payment:**
```bash
soroban contract invoke \
    --id C... \
    --network testnet \
    --source S... \
    -- \
    record_payment \
    --amount 10000000 \
    --asset C...
```

**Sweep:**
```bash
soroban contract invoke \
    --id C... \
    --network testnet \
    --source S... \
    -- \
    sweep \
    --destination G... \
    --auth_signature 0000...
```
