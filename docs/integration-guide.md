# Integration Guide: How bridgelet-sdk Calls the Contracts

This guide documents the exact sequence of contract calls that `bridgelet-sdk` makes when creating an ephemeral account, monitoring for payment, and sweeping funds to a permanent wallet. Follow these steps in order when integrating against the deployed Soroban contracts.

## Prerequisites

- Both `SweepController` and `EphemeralAccount` contracts are deployed on-chain
- `SweepController` is initialized with the authorized signer's Ed25519 public key
- The SDK has access to the corresponding authorized signer private key for signature generation

---

## Contract Call Sequence

### Step 1: Deploy and Initialize SweepController

The `SweepController` is a singleton per SDK operator. Deploy it once and reuse its address across all ephemeral accounts.

**Function signature:**
```rust
fn initialize(
    env: Env,
    creator: Address,
    authorized_signer: BytesN<32>,   // Ed25519 public key (32 bytes)
    authorized_destination: Option<Address>,
) -> Result<(), Error>
```

**Parameters:**

| Parameter                | Type               | Description                                                                 |
|--------------------------|--------------------|-----------------------------------------------------------------------------|
| `creator`                | `Address`          | The account that owns and administers this controller                       |
| `authorized_signer`      | `BytesN<32>`       | Ed25519 public key whose private key will sign sweep authorizations         |
| `authorized_destination` | `Option<Address>`  | If `Some`, restricts all sweeps to this destination. `None` = any address   |

**stellar-cli example:**
```bash
stellar contract invoke \
  --id <SWEEP_CONTROLLER_CONTRACT_ID> \
  --source <CREATOR_SECRET_KEY> \
  --network testnet \
  -- initialize \
  --creator <CREATOR_ADDRESS> \
  --authorized_signer <ED25519_PUBLIC_KEY_HEX_32_BYTES> \
  --authorized_destination null
```

---

### Step 2: Deploy and Initialize EphemeralAccount

A new `EphemeralAccount` contract is deployed for each inbound payment session. The `authorized_controller` must be set to the `SweepController` contract address so that only `SweepController.execute_sweep()` can trigger a sweep.

**Function signature:**
```rust
fn initialize(
    env: Env,
    creator: Address,
    expiry_ledger: u32,
    recovery_address: Address,
    authorized_controller: Address,  // Must be SweepController contract address
) -> Result<(), Error>
```

**Parameters:**

| Parameter               | Type      | Description                                                                  |
|-------------------------|-----------|------------------------------------------------------------------------------|
| `creator`               | `Address` | SDK operator address that created this ephemeral account                     |
| `expiry_ledger`         | `u32`     | Ledger number after which the account is considered expired                  |
| `recovery_address`      | `Address` | Destination for funds if the account expires before being swept              |
| `authorized_controller` | `Address` | **Set to the SweepController contract address**                              |

**stellar-cli example:**
```bash
stellar contract invoke \
  --id <EPHEMERAL_ACCOUNT_CONTRACT_ID> \
  --source <CREATOR_SECRET_KEY> \
  --network testnet \
  -- initialize \
  --creator <CREATOR_ADDRESS> \
  --expiry_ledger 1234567 \
  --recovery_address <RECOVERY_ADDRESS> \
  --authorized_controller <SWEEP_CONTROLLER_CONTRACT_ID>
```

> **Note:** `expiry_ledger` should be set based on current ledger + desired TTL. Stellar produces roughly 5 ledgers per second, so a 24-hour window is approximately `current_ledger + 432000`.

---

### Step 3: Monitor for Incoming Payment (Off-chain)

After the ephemeral account is initialized, share the contract address with the payer. The SDK monitors Horizon for the `PaymentReceived` contract event emitted by `record_payment()`.

**Horizon event stream:**
```
GET https://horizon-testnet.stellar.org/contracts/<EPHEMERAL_ACCOUNT_CONTRACT_ID>/events
```

**`PaymentReceived` event body:**
```json
{
  "type": "contract",
  "topic": ["PaymentReceived"],
  "value": {
    "amount": "1000000000",
    "asset": "<ASSET_CONTRACT_ADDRESS>"
  }
}
```

The `Payment` struct carried by this event:
```
Payment { asset: Address, amount: i128, timestamp: u64 }
```

When this event is observed, proceed to Step 4.

---

### Step 4: Verify Account is Ready to Sweep

Before generating a signature, confirm the account is in the `PaymentReceived` state. Use either method:

**Option A — via SweepController (recommended):**
```rust
fn can_sweep(env: Env, ephemeral_account: Address) -> bool
```
Returns `true` only when the ephemeral account status is `PaymentReceived(1)` and the account is not expired.

**Option B — via EphemeralAccount directly:**
```rust
fn get_status(env: Env) -> AccountStatus
```

**`AccountStatus` values:**

| Value | Meaning                                  |
|-------|------------------------------------------|
| `0`   | `Active` — initialized, no payment yet  |
| `1`   | `PaymentReceived` — ready to sweep      |
| `2`   | `Swept` — funds already transferred     |
| `3`   | `Expired` — expiry ledger has passed    |

Only proceed with the sweep if the status is `PaymentReceived(1)`.

---

### Step 5: Generate Sweep Authorization Signature (Off-chain)

The signature is computed entirely off-chain by the SDK using the authorized signer private key.

**Message construction:**
```
message = SHA256(destination_xdr_bytes || nonce_u64_big_endian || contract_id_xdr_bytes)
```

- `destination_xdr_bytes` — XDR-encoded bytes of the destination `Address`
- `nonce_u64_big_endian` — current nonce from `SweepController`, encoded as 8-byte big-endian
- `contract_id_xdr_bytes` — XDR-encoded bytes of the `SweepController` contract `Address`

The nonce increments after each successful sweep, preventing replay attacks.

**TypeScript pseudocode:**
```typescript
import * as ed from '@noble/ed25519';
import { sha256 } from '@noble/hashes/sha256';
import { Address } from '@stellar/stellar-sdk';

async function buildSweepSignature(
  destinationAddress: string,
  nonce: bigint,
  sweepControllerContractId: string,
  signerPrivateKey: Uint8Array,
): Promise<Uint8Array> {
  // XDR-encode the destination address
  const destXdr = Address.fromString(destinationAddress).toScVal().toXDR();

  // Encode nonce as 8-byte big-endian
  const nonceBytes = new Uint8Array(8);
  new DataView(nonceBytes.buffer).setBigUint64(0, nonce, false /* big-endian */);

  // XDR-encode the SweepController contract address
  const contractXdr = Address.contract(
    Buffer.from(sweepControllerContractId, 'hex')
  ).toScVal().toXDR();

  // Concatenate and hash
  const message = sha256(
    new Uint8Array([...destXdr, ...nonceBytes, ...contractXdr])
  );

  // Sign with Ed25519
  return ed.sign(message, signerPrivateKey);
}
```

The returned 64-byte signature is passed directly to `execute_sweep()`.

---

### Step 6: Execute Sweep

Call `SweepController.execute_sweep()` with the ephemeral account address, destination, and the signature from Step 5.

**Function signature:**
```rust
fn execute_sweep(
    env: Env,
    ephemeral_account: Address,
    destination: Address,
    auth_signature: BytesN<64>,
) -> Result<(), Error>
```

**Parameters:**

| Parameter          | Type           | Description                                         |
|--------------------|----------------|-----------------------------------------------------|
| `ephemeral_account`| `Address`      | The ephemeral account contract to sweep from        |
| `destination`      | `Address`      | Wallet address to receive the swept funds           |
| `auth_signature`   | `BytesN<64>`   | Ed25519 signature produced in Step 5                |

**What happens internally:**
1. `SweepController` reconstructs the message from `destination`, current nonce, and its own contract ID
2. Verifies the Ed25519 signature against the stored `authorized_signer` public key
3. If `authorized_destination` was set at initialization, asserts `destination` matches
4. Increments the nonce (prevents replay)
5. Calls `EphemeralAccount.sweep(destination, auth_signature)` as a cross-contract call
6. `EphemeralAccount` executes SAC token transfers for all recorded payments to `destination`
7. A `SweepExecutedMulti` event is emitted

**stellar-cli example:**
```bash
stellar contract invoke \
  --id <SWEEP_CONTROLLER_CONTRACT_ID> \
  --source <INVOKER_SECRET_KEY> \
  --network testnet \
  -- execute_sweep \
  --ephemeral_account <EPHEMERAL_ACCOUNT_CONTRACT_ID> \
  --destination <DESTINATION_ADDRESS> \
  --auth_signature <64_BYTE_SIGNATURE_HEX>
```

---

### Step 7: Handle Expiry (Alternative Path)

If the `expiry_ledger` is reached before a sweep occurs, the account becomes `Expired(3)`. In this case, call `expire()` to release funds to the `recovery_address`.

**Function signature:**
```rust
fn expire(env: Env) -> Result<(), Error>
```

This function:
- Asserts `env.ledger().sequence() >= expiry_ledger`
- Transfers all recorded payment assets to `recovery_address`
- Emits an `AccountExpired` event
- Sets account status to `Expired(3)`

**stellar-cli example:**
```bash
stellar contract invoke \
  --id <EPHEMERAL_ACCOUNT_CONTRACT_ID> \
  --source <ANY_INVOKER_SECRET_KEY> \
  --network testnet \
  -- expire
```

> Anyone can call `expire()` once the expiry ledger is passed. The SDK should monitor expiry and call this to reclaim funds.

---

## XDR Examples

### Decoding a `SweepExecutedMulti` event from Horizon

**Raw Horizon response (abbreviated):**
```json
{
  "type": "contract",
  "ledger": "1234570",
  "contract_id": "<EPHEMERAL_ACCOUNT_CONTRACT_ID>",
  "topic": [
    "AAAADwAAAA9Td2VlcEV4ZWN1dGVkTXVsdGk=",
    "AAAAA..."
  ],
  "value": "AAAAB..."
}
```

**Decoded event fields:**
```
Event: SweepExecutedMulti
  destination:  G... (Stellar address)
  payments:     [
    Payment { asset: <CONTRACT_ID>, amount: 1000000000, timestamp: 1750000000 }
  ]
```

Decode using the Stellar SDK:
```typescript
import { xdr, scValToNative } from '@stellar/stellar-sdk';

const value = xdr.ScVal.fromXDR(Buffer.from(rawValueBase64, 'base64'));
const native = scValToNative(value);
// native = { destination: "G...", payments: [{ asset: "C...", amount: 1000000000n, timestamp: 1750000000n }] }
```

---

## Error Handling

| Error                    | Contract         | Cause                                                      | SDK Handling                                      |
|--------------------------|------------------|------------------------------------------------------------|---------------------------------------------------|
| `AlreadyInitialized`     | Both             | `initialize()` called more than once                       | Skip — check init state before calling            |
| `NotInitialized`         | Both             | Function called before `initialize()`                      | Fatal — redeploy contract                         |
| `AlreadySwept`           | EphemeralAccount | `sweep()` or `execute_sweep()` called after status = `Swept`| Skip — check status before calling               |
| `AlreadyExpired`         | EphemeralAccount | `sweep()` called after `expiry_ledger` passed              | Fall back to expiry path (Step 7)                 |
| `NotExpired`             | EphemeralAccount | `expire()` called before `expiry_ledger`                   | Wait for expiry ledger, retry                     |
| `NoPaymentReceived`      | EphemeralAccount | `sweep()` called with status still `Active`                | Wait for `PaymentReceived` event, retry           |
| `InvalidSignature`       | SweepController  | Signature doesn't verify against `authorized_signer`       | Regenerate signature with correct key/nonce       |
| `InvalidDestination`     | SweepController  | `destination` doesn't match `authorized_destination`       | Use the correct pre-configured destination        |
| `Unauthorized`           | EphemeralAccount | `sweep()` called by an address other than `authorized_controller` | Always call sweep via `SweepController`      |

---

## Sequence Diagram

```
SDK (off-chain)          SweepController          EphemeralAccount         Horizon / Payer
      |                        |                         |                        |
      |-- deploy & initialize->|                         |                        |
      |   (authorized_signer,  |                         |                        |
      |    authorized_dest?)   |                         |                        |
      |                        |                         |                        |
      |-- deploy & initialize-------------------------->|                        |
      |   (creator, expiry_ledger, recovery_address,    |                        |
      |    authorized_controller=SweepController)       |                        |
      |                        |                         |                        |
      |<-- share ephemeral account address with payer ----------------------->|  |
      |                        |                         |                        |
      |                        |                         |<-- SAC payment --------|
      |                        |                         |    (record_payment     |
      |                        |                         |     called on-chain)   |
      |                        |                         |                        |
      |<============ PaymentReceived event (Horizon) =========================-|  |
      |                        |                         |                        |
      |-- can_sweep(ephemeral_account) ->|               |                        |
      |<-- true ---------------|                         |                        |
      |                        |                         |                        |
      | [build message: SHA256(dest_xdr||nonce_be||contract_id_xdr)]             |
      | [sign with Ed25519 private key]                                          |
      |                        |                         |                        |
      |-- execute_sweep(ephemeral_account, destination, auth_signature) -------> |
      |                        | verify signature        |                        |
      |                        | increment nonce         |                        |
      |                        |-- sweep(destination, auth_signature) ---------->|
      |                        |                         | transfer tokens        |
      |                        |                         | emit SweepExecutedMulti|
      |                        |                         |                        |
      |<============ SweepExecutedMulti event (Horizon) ======================|  |
      |                        |                         |                        |
      :                        :                         :                        :
      : --- EXPIRY PATH (if expiry_ledger reached before sweep) ---              :
      :                        :                         :                        :
      |-- expire() ----------------------------------------->|                  |
      |                        |                         | transfer to            |
      |                        |                         | recovery_address       |
      |                        |                         | emit AccountExpired    |
```
