# Bridgelet-Core API Reference

Complete rustdoc-style reference for all public contract functions. Intended for SDK developers integrating with Bridgelet contracts.

---

## EphemeralAccount Contract

Manages a single-use restricted account that accepts one or more token payments and enforces authorized sweep or expiry logic.

### Functions

#### `initialize`

Initializes the ephemeral account. Must be called exactly once. Subsequent calls return `AlreadyInitialized`.

```rust
fn initialize(
    env: Env,
    creator: Address,
    expiry_ledger: u32,
    recovery_address: Address,
    authorized_controller: Address,
) -> Result<(), Error>
```

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `creator` | `Address` | The account that created this contract. Must authorize this call. |
| `expiry_ledger` | `u32` | Ledger sequence number at which the account expires. Must be in the future. |
| `recovery_address` | `Address` | Address that receives funds if the account expires without being swept. |
| `authorized_controller` | `Address` | The `SweepController` contract address authorized to call `sweep()` on behalf of this account. |

**Returns:** `Ok(())` on success.

**Errors:**

| Error | Condition |
| :--- | :--- |
| `AlreadyInitialized` | `initialize` has already been called on this contract. |
| `InvalidExpiry` | `expiry_ledger` is less than or equal to the current ledger sequence. |

**Auth required:** `creator.require_auth()`

**Events emitted:** `AccountCreated { creator, expiry_ledger }`

---

#### `record_payment`

Records an inbound token payment. Supports multiple assets; each asset may only be recorded once. Maximum of 10 distinct assets.

```rust
fn record_payment(env: Env, amount: i128, asset: Address) -> Result<(), Error>
```

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `amount` | `i128` | Payment amount in the asset's base unit. Must be positive (> 0). |
| `asset` | `Address` | Token contract address (SEP-41 compatible). |

**Returns:** `Ok(())` on success.

**Errors:**

| Error | Condition |
| :--- | :--- |
| `NotInitialized` | `initialize` has not been called. |
| `InvalidAmount` | `amount` is zero or negative. |
| `DuplicateAsset` | A payment for `asset` has already been recorded. |
| `TooManyPayments` | 10 distinct assets are already recorded. |

**Auth required:** None. Any caller may record a payment.

**Events emitted:**
- First payment: `PaymentReceived { amount, asset }`
- Subsequent payments: `MultiPaymentReceived { asset, amount }`

---

#### `sweep`

Marks the account as swept and authorizes fund transfers to `destination`. All recorded payments are included. The actual token transfers are executed by `SweepController` after this call completes.

```rust
fn sweep(
    env: Env,
    destination: Address,
    auth_signature: BytesN<64>,
) -> Result<(), Error>
```

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `destination` | `Address` | Recipient wallet address for all recorded funds. |
| `auth_signature` | `BytesN<64>` | Ed25519 signature covering `destination + nonce + contract_id`. In the current MVP this parameter is accepted but verification is delegated to `authorized_controller.require_auth()`. |

**Returns:** `Ok(())` on success.

**Errors:**

| Error | Condition |
| :--- | :--- |
| `NotInitialized` | `initialize` has not been called. |
| `AlreadySwept` | Sweep has already been executed. |
| `NoPaymentReceived` | No payments have been recorded. |
| `AccountExpired` | Current ledger ≥ `expiry_ledger`. |
| `Unauthorized` | `authorized_controller` did not authorize this call. |

**Auth required:** `authorized_controller.require_auth()` — enforced via `SweepController`'s `authorize_as_current_contract()`.

**State update:** Sets `status = Swept` **before** any further work, preventing reentrancy.

**Events emitted:** `SweepExecutedMulti { destination, payments }`, `ReserveReclaimed { ... }`

---

#### `expire`

Marks the account as expired and routes funds to `recovery_address`. Can only be called after `expiry_ledger` is reached.

```rust
fn expire(env: Env) -> Result<(), Error>
```

**Returns:** `Ok(())` on success.

**Errors:**

| Error | Condition |
| :--- | :--- |
| `NotInitialized` | `initialize` has not been called. |
| `InvalidStatus` | Account is already `Swept` or `Expired`. |
| `NotExpired` | Current ledger < `expiry_ledger`. |

**Auth required:** None. Any caller may trigger expiry once the ledger threshold is passed.

**Events emitted:** `AccountExpired { recovery_address, total_amount, reserve_amount }`, `ReserveReclaimed { ... }`

---

#### `is_expired`

Returns `true` if the current ledger sequence has reached or passed `expiry_ledger`.

```rust
fn is_expired(env: Env) -> bool
```

---

#### `get_status`

Returns the current lifecycle status of the account.

```rust
fn get_status(env: Env) -> AccountStatus
```

```rust
enum AccountStatus {
    Active = 0,         // Initialized, no payment yet
    PaymentReceived = 1, // At least one payment recorded
    Swept = 2,          // Sweep executed
    Expired = 3,        // Account expired, funds sent to recovery
}
```

---

#### `get_info`

Returns the complete state of the account.

```rust
fn get_info(env: Env) -> Result<AccountInfo, Error>
```

**Errors:** `NotInitialized` if `initialize` has not been called.

```rust
struct AccountInfo {
    creator: Address,
    status: AccountStatus,
    expiry_ledger: u32,
    recovery_address: Address,
    payment_received: bool,      // true if payment_count > 0
    payment_count: u32,
    payments: Vec<Payment>,
    swept_to: Option<Address>,   // set after sweep or expire
}

struct Payment {
    asset: Address,
    amount: i128,
    timestamp: u64,  // ledger timestamp at time of record_payment
}
```

---

#### `reclaim_reserve`

Reclaims any remaining base reserve (1 XLM denominated in stroops) that has not yet been transferred. Safe to call repeatedly; returns `0` once fully reclaimed.

```rust
fn reclaim_reserve(env: Env) -> Result<i128, Error>
```

**Returns:** Amount reclaimed in this call (in stroops).

**Errors:**

| Error | Condition |
| :--- | :--- |
| `NotInitialized` | `initialize` has not been called. |
| `InvalidStatus` | Account is neither `Swept` nor `Expired`. |

---

#### `get_reserve_remaining`

Returns the reserve amount (stroops) still awaiting reclaim.

```rust
fn get_reserve_remaining(env: Env) -> i128
```

---

#### `get_reserve_available`

Returns the reserve amount (stroops) currently available for transfer.

```rust
fn get_reserve_available(env: Env) -> i128
```

---

#### `is_reserve_reclaimed`

Returns `true` if the full base reserve has been reclaimed.

```rust
fn is_reserve_reclaimed(env: Env) -> bool
```

---

#### `get_last_reserve_event`

Returns the most recently emitted `ReserveReclaimed` event payload, or `None`.

```rust
fn get_last_reserve_event(env: Env) -> Option<ReserveReclaimed>
```

---

#### `get_reserve_reclaim_event_count`

Returns the total number of `ReserveReclaimed` events emitted by this contract.

```rust
fn get_reserve_reclaim_event_count(env: Env) -> u32
```

---

### Events

| Topic | Struct | Trigger |
| :--- | :--- | :--- |
| `created` | `AccountCreated { creator, expiry_ledger }` | `initialize` success |
| `payment` | `PaymentReceived { amount, asset }` | First `record_payment` call |
| `multi_pay` | `MultiPaymentReceived { asset, amount }` | Second and subsequent `record_payment` calls |
| `swept_mul` | `SweepExecutedMulti { destination, payments }` | `sweep` success |
| `expired` | `AccountExpired { recovery_address, amount_returned, reserve_amount }` | `expire` success |
| `reserve` | `ReserveReclaimed { destination, amount, sweep_id, fully_reclaimed, remaining_reserve }` | After each sweep or expire that transfers reserve |

---

### Error Codes

| Code | Variant | Description |
| :--- | :--- | :--- |
| 1 | `AlreadyInitialized` | Contract already initialized. |
| 2 | `NotInitialized` | Contract not initialized. |
| 3 | `PaymentAlreadyReceived` | Deprecated. Use `DuplicateAsset` (code 13). |
| 4 | `InvalidAmount` | Payment amount is zero or negative. |
| 5 | `InvalidExpiry` | `expiry_ledger` is not in the future. |
| 6 | `NotExpired` | Attempted to expire before `expiry_ledger`. |
| 7 | `AlreadySwept` | Account already swept. |
| 8 | `Unauthorized` | `authorized_controller` did not authorize the call. |
| 9 | `InvalidSignature` | Cryptographic signature format is invalid. |
| 10 | `NoPaymentReceived` | Cannot sweep without a recorded payment. |
| 11 | `AccountExpired` | Cannot sweep an expired account. |
| 12 | `InvalidStatus` | Action is invalid for the current account status. |
| 13 | `DuplicateAsset` | Asset already has a recorded payment. |
| 14 | `TooManyPayments` | Maximum of 10 distinct assets reached. |

---

## SweepController Contract

Orchestrates sweep authorization using Ed25519 signature verification and executes atomic token transfers.

### Functions

#### `initialize`

Sets up the controller with an authorized Ed25519 signer and an optional locked destination address. Can only be called once.

```rust
fn initialize(
    env: Env,
    creator: Address,
    authorized_signer: BytesN<32>,
    authorized_destination: Option<Address>,
) -> Result<(), Error>
```

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `creator` | `Address` | Address that owns this controller instance. Required to authorize future `update_authorized_destination` calls. Must authorize this call. |
| `authorized_signer` | `BytesN<32>` | Ed25519 public key used to verify all sweep authorization signatures. |
| `authorized_destination` | `Option<Address>` | If `Some(addr)`, the controller operates in **locked mode**: sweeps can only transfer to this specific address. If `None`, any destination is accepted (**flexible mode**). |

**Returns:** `Ok(())` on success.

**Errors:**

| Error | Condition |
| :--- | :--- |
| `AuthorizationFailed` | `initialize` has already been called. |

**Auth required:** `creator.require_auth()`

**Events emitted:** `DestinationAuthorized { destination }` (only when `authorized_destination` is `Some`).

---

#### `execute_sweep`

Verifies the Ed25519 authorization signature, then calls `EphemeralAccount::sweep()` and executes the token transfers to `destination`.

```rust
fn execute_sweep(
    env: Env,
    ephemeral_account: Address,
    destination: Address,
    auth_signature: BytesN<64>,
) -> Result<(), Error>
```

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `ephemeral_account` | `Address` | Address of the `EphemeralAccount` contract to sweep. |
| `destination` | `Address` | Recipient wallet address for all swept funds. |
| `auth_signature` | `BytesN<64>` | Ed25519 signature over `SHA256(destination_xdr \|\| nonce_u64_be \|\| contract_id_xdr)`. Must be signed by the key in `authorized_signer`. |

**Returns:** `Ok(())` on success.

**Errors:**

| Error | Condition |
| :--- | :--- |
| `UnauthorizedDestination` | Controller is in locked mode and `destination` ≠ `authorized_destination`. |
| `AuthorizationFailed` | `authorized_signer` is not set (controller not initialized). |
| `AuthorizedSignerNotSet` | Ed25519 public key has not been stored. |
| `SignatureVerificationFailed` | Signature does not verify against the current nonce and destination. |
| `AccountNotReady` | Ephemeral account has no recorded payments or zero total amount. |
| `TransferFailed` | A SEP-41 token `transfer()` call failed. |

**Signature message format:**

```
message = SHA256(
    destination.to_xdr()
    || nonce as u64 big-endian (8 bytes)
    || controller_contract_address.to_xdr()
)
```

The nonce is incremented after each successful `execute_sweep` call to prevent replay attacks.

**Events emitted:** `SweepCompleted { ephemeral_account, destination, amount }`

---

#### `claim`

Gas-free claim path for the recipient. The recipient signs a Soroban auth entry for `claim(recipient, ephemeral_account)` only; a relayer or SDK submits the transaction and pays fees.

Internally the controller uses `authorize_as_current_contract()` to satisfy `authorized_controller.require_auth()` inside `EphemeralAccount::sweep()`.

```rust
fn claim(env: Env, recipient: Address, ephemeral_account: Address) -> Result<(), Error>
```

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `recipient` | `Address` | The address claiming the funds. Must authorize this call. |
| `ephemeral_account` | `Address` | Address of the `EphemeralAccount` contract to sweep. |

**Returns:** `Ok(())` on success.

**Errors:**

| Error | Condition |
| :--- | :--- |
| `UnauthorizedDestination` | Controller is in locked mode and `recipient` ≠ `authorized_destination`. |

**Auth required:** `recipient.require_auth()`

**Events emitted:** `SweepCompleted { ephemeral_account, destination: recipient, amount }`

---

#### `can_sweep`

Returns `true` if the ephemeral account has a recorded payment, is in `PaymentReceived` status, and has not expired.

```rust
fn can_sweep(env: Env, ephemeral_account: Address) -> bool
```

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `ephemeral_account` | `Address` | Address of the `EphemeralAccount` contract to check. |

---

#### `update_authorized_destination`

Allows the creator to update the locked destination before any sweep has occurred. Fails if a sweep has already been executed (nonce > 0).

```rust
fn update_authorized_destination(env: Env, new_destination: Address) -> Result<(), Error>
```

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `new_destination` | `Address` | The new address sweeps will be restricted to. |

**Returns:** `Ok(())` on success.

**Errors:**

| Error | Condition |
| :--- | :--- |
| `AuthorizationFailed` | Caller is not the creator or controller is not initialized. |
| `AccountAlreadySwept` | At least one sweep has been executed (nonce > 0); destination is now immutable. |

**Auth required:** `creator.require_auth()`

**Events emitted:** `DestinationUpdated { old_destination, new_destination }`

---

### Events

| Topic | Struct | Trigger |
| :--- | :--- | :--- |
| `sweep` | `SweepCompleted { ephemeral_account, destination, amount }` | `execute_sweep` or `claim` success |
| `dest_auth` | `DestinationAuthorized { destination }` | `initialize` with a non-`None` `authorized_destination` |
| `dest_upd` | `DestinationUpdated { old_destination, new_destination }` | `update_authorized_destination` success |

---

### Error Codes

| Code | Variant | Description |
| :--- | :--- | :--- |
| 1 | `InvalidAccount` | Account is not in a valid state for the requested operation. |
| 2 | `TransferFailed` | A SEP-41 token transfer failed. |
| 3 | `AuthorizationFailed` | Signature invalid, caller not authorized, or already initialized. |
| 4 | `InsufficientBalance` | Reserved for future use. |
| 5 | `AccountNotReady` | Account has no payments or zero total amount. |
| 6 | `AccountExpired` | Account has expired. |
| 7 | `AccountAlreadySwept` | A sweep has already been executed; destination cannot be changed. |
| 8 | `InvalidSignature` | Signature format is invalid. |
| 9 | `SignatureVerificationFailed` | Ed25519 verification failure. |
| 10 | `AuthorizedSignerNotSet` | Controller was not initialized with an authorized signer. |
| 11 | `InvalidNonce` | Security nonce is invalid or out of sequence. |
| 13 | `UnauthorizedDestination` | Destination does not match the locked `authorized_destination`. |

---

## Usage Examples

### Rust SDK Integration — Single Asset

```rust
use soroban_sdk::{Address, BytesN, Env};
use ephemeral_account::EphemeralAccountContractClient as EphemeralClient;
use sweep_controller::SweepControllerClient;

fn example_single_asset(
    env: &Env,
    controller_id: &Address,
    ephemeral_id: &Address,
    creator: &Address,
    recovery: &Address,
    authorized_controller: &Address,
    usdc_addr: &Address,
    destination: &Address,
    auth_sig: BytesN<64>,
) {
    let ephemeral = EphemeralClient::new(env, ephemeral_id);
    let controller = SweepControllerClient::new(env, controller_id);

    // 1. Initialize ephemeral account, referencing this SweepController
    ephemeral.initialize(
        creator,
        &(env.ledger().sequence() + 1000),
        recovery,
        authorized_controller,
    );

    // 2. Record incoming USDC payment (called by off-chain watcher)
    ephemeral.record_payment(&100_000_000, usdc_addr); // 100 USDC (7 decimals)

    // 3. Execute sweep via the controller (signature generated off-chain)
    controller.execute_sweep(ephemeral_id, destination, &auth_sig);
}
```

### Rust SDK Integration — Multi-Asset

```rust
fn example_multi_asset(
    env: &Env,
    ephemeral_id: &Address,
    controller_id: &Address,
    usdc_addr: &Address,
    xlm_addr: &Address,
    destination: &Address,
    auth_sig: BytesN<64>,
) {
    let ephemeral = EphemeralAccountContractClient::new(env, ephemeral_id);
    let controller = SweepControllerClient::new(env, controller_id);

    // Record multiple asset payments
    ephemeral.record_payment(&100_000_000, usdc_addr);
    ephemeral.record_payment(&5_000_000_000, xlm_addr); // 500 XLM in stroops

    // Sweep transfers ALL recorded assets atomically
    controller.execute_sweep(ephemeral_id, destination, &auth_sig);
}
```

### Gas-Free Claim Flow

```rust
fn example_claim(
    env: &Env,
    controller_id: &Address,
    ephemeral_id: &Address,
    recipient: &Address,
) {
    // recipient only signs the `claim` auth entry; relayer pays fees
    let controller = SweepControllerClient::new(env, controller_id);
    controller.claim(recipient, ephemeral_id);
}
```

### CLI Invocation

**Initialize ephemeral account:**
```bash
soroban contract invoke \
    --id <EPHEMERAL_CONTRACT_ID> \
    --network testnet \
    --source <CREATOR_SECRET> \
    -- \
    initialize \
    --creator <CREATOR_ADDRESS> \
    --expiry_ledger 123456 \
    --recovery_address <RECOVERY_ADDRESS> \
    --authorized_controller <SWEEP_CONTROLLER_ID>
```

**Initialize sweep controller (locked mode):**
```bash
soroban contract invoke \
    --id <CONTROLLER_CONTRACT_ID> \
    --network testnet \
    --source <CREATOR_SECRET> \
    -- \
    initialize \
    --creator <CREATOR_ADDRESS> \
    --authorized_signer <ED25519_PUBLIC_KEY_HEX> \
    --authorized_destination <DESTINATION_ADDRESS>
```

**Record payment:**
```bash
soroban contract invoke \
    --id <EPHEMERAL_CONTRACT_ID> \
    --network testnet \
    --source <ANY_SOURCE> \
    -- \
    record_payment \
    --amount 100000000 \
    --asset <TOKEN_CONTRACT_ID>
```

**Execute sweep:**
```bash
soroban contract invoke \
    --id <CONTROLLER_CONTRACT_ID> \
    --network testnet \
    --source <RELAYER_SECRET> \
    -- \
    execute_sweep \
    --ephemeral_account <EPHEMERAL_CONTRACT_ID> \
    --destination <DESTINATION_ADDRESS> \
    --auth_signature <64_BYTE_ED25519_SIG_HEX>
```

**Expire account:**
```bash
soroban contract invoke \
    --id <EPHEMERAL_CONTRACT_ID> \
    --network testnet \
    --source <ANY_SOURCE> \
    -- \
    expire
```
