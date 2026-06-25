# Contract Events Reference

Bridgelet Core contracts emit Soroban events on all critical state transitions. Events are indexed by Horizon and queryable via the Events API, enabling off-chain systems to react to contract activity without polling contract storage.

---

## How to Decode Events from Horizon

### Querying Events

Soroban contract events are available through the Horizon `/events` endpoint. Filter by `contract_id` to scope results to a specific contract:

```bash
# Fetch recent events for a contract on testnet
curl "https://horizon-testnet.stellar.org/contracts/CCONTRACT_ID_HERE/events?limit=20&order=desc"
```

You can also retrieve events for a specific transaction:

```bash
curl "https://horizon-testnet.stellar.org/transactions/TX_HASH_HERE/effects"
```

### Decoding XDR

Each event's `topic` and `value` fields are base64-encoded XDR (`ScVal`). Use the Stellar SDK or soroban-client to decode them:

**JavaScript (stellar-sdk)**
```js
import { xdr, ScVal } from '@stellar/stellar-sdk';

// Decode a topic entry
const topic = xdr.ScVal.fromXDR(base64TopicString, 'base64');

// Decode the event data value
const value = xdr.ScVal.fromXDR(base64ValueString, 'base64');
console.log(value.toJSON());
```

**Rust (soroban-sdk / stellar-xdr)**
```rust
use stellar_xdr::curr::{ScVal, ReadXdr};

let bytes = base64::decode(base64_string)?;
let val = ScVal::from_xdr(&bytes)?;
```

The `topic` array always contains one entry: the `symbol_short!` string encoded as `ScVal::Symbol`. The `value` entry is a `ScVal::Map` containing the struct fields.

---

## EphemeralAccount Contract Events

### AccountCreated (topic: `"created"`)

**Emitted by:** `initialize`  
**Trigger:** A new ephemeral account is successfully initialized with its expiry and recovery configuration.  
**Topic:** `["created"]` (symbol_short)

**Data fields:**

| Field | Rust Type | XDR ScVal Type | Description |
|---|---|---|---|
| `creator` | `Address` | `ScVal::Address` | Address that initialized the ephemeral account |
| `expiry_ledger` | `u32` | `ScVal::U32` | Ledger sequence number at which this account expires |

**Example Horizon response:**
```json
{
  "type": "contract",
  "ledger": "12345678",
  "contract_id": "CEPHEMERALACCOUNTCONTRACTID000000000000000000000000000",
  "topic": ["AAAADwAAAAdjcmVhdGVkAA=="],
  "value": "AAAAEAAAAAIAAAAPAAAACGNyZWF0b3IAAAASAAAAAgAAAA8AAAAMZXhwaXJ5X2xlZGdlcgAAAAM="
}
```
> The base64 values above are illustrative. Actual values depend on the specific account and ledger.

---

### PaymentReceived (topic: `"payment"`)

**Emitted by:** `record_payment`  
**Trigger:** An inbound payment is recorded on the ephemeral account.  
**Topic:** `["payment"]` (symbol_short)

**Data fields:**

| Field | Rust Type | XDR ScVal Type | Description |
|---|---|---|---|
| `amount` | `i128` | `ScVal::I128` | Payment amount in the asset's smallest unit (stroops for XLM) |
| `asset` | `Address` | `ScVal::Address` | Contract address of the asset token |

**Example Horizon response:**
```json
{
  "type": "contract",
  "ledger": "12345679",
  "contract_id": "CEPHEMERALACCOUNTCONTRACTID000000000000000000000000000",
  "topic": ["AAAADwAAAAdwYXltZW50AA=="],
  "value": "AAAAEAAAAAIAAAAPAAAABmFtb3VudAAAAAgAAAAAAAAAAAAAAAAAAAAPAAAABWFzc2V0AAAAAAAS"
}
```
> The base64 values above are illustrative.

---

### MultiPaymentReceived (topic: `"multi_pay"`)

**Emitted by:** `record_payment` (multi-asset path)  
**Trigger:** Emitted per asset when a multi-asset payment is received. One event is emitted for each distinct asset in the payment.  
**Topic:** `["multi_pay"]` (symbol_short)

**Data fields:**

| Field | Rust Type | XDR ScVal Type | Description |
|---|---|---|---|
| `asset` | `Address` | `ScVal::Address` | Contract address of the asset token |
| `amount` | `i128` | `ScVal::I128` | Amount received for this specific asset |

**Example Horizon response:**
```json
{
  "type": "contract",
  "ledger": "12345680",
  "contract_id": "CEPHEMERALACCOUNTCONTRACTID000000000000000000000000000",
  "topic": ["AAAADwAAAAltdWx0aV9wYXkAAAA="],
  "value": "AAAAEAAAAAIAAAAPAAAABWFzc2V0AAAAAAAAEgAAAAIAAAAPAAAABmFtb3VudAAAAAgA"
}
```
> The base64 values above are illustrative.

---

### SweepExecutedMulti (topic: `"swept_mul"`)

**Emitted by:** `sweep`  
**Trigger:** A sweep to the permanent wallet is successfully executed, covering one or more asset payments.  
**Topic:** `["swept_mul"]` (symbol_short)

**Data fields:**

| Field | Rust Type | XDR ScVal Type | Description |
|---|---|---|---|
| `destination` | `Address` | `ScVal::Address` | Address of the permanent wallet that received the funds |
| `payments` | `Vec<Payment>` | `ScVal::Vec` | List of payment records swept; each entry is a `ScVal::Map` |

**`Payment` struct fields (each element of `payments`):**

| Field | Rust Type | XDR ScVal Type | Description |
|---|---|---|---|
| `asset` | `Address` | `ScVal::Address` | Asset contract address |
| `amount` | `i128` | `ScVal::I128` | Amount transferred |
| `timestamp` | `u64` | `ScVal::U64` | Ledger timestamp when the payment was recorded |

**Example Horizon response:**
```json
{
  "type": "contract",
  "ledger": "12345690",
  "contract_id": "CEPHEMERALACCOUNTCONTRACTID000000000000000000000000000",
  "topic": ["AAAADwAAAAlzd2VwdF9tdWwAAAA="],
  "value": "AAAAEAAAAAIAAAAPAAAABmRlc3RpbgAAAAASAAAAAgAAAA8AAAAIcGF5bWVudHMAAAAQ"
}
```
> The base64 values above are illustrative.

---

### AccountExpired (topic: `"expired"`)

**Emitted by:** `recover` / expiry handler  
**Trigger:** The ephemeral account has passed its `expiry_ledger` and funds are being returned to the recovery address.  
**Topic:** `["expired"]` (symbol_short)

**Data fields:**

| Field | Rust Type | XDR ScVal Type | Description |
|---|---|---|---|
| `recovery_address` | `Address` | `ScVal::Address` | Address that receives returned funds |
| `amount_returned` | `i128` | `ScVal::I128` | Total token amount returned to the recovery address |
| `reserve_amount` | `i128` | `ScVal::I128` | XLM base reserve reclaimed alongside the token return |

**Example Horizon response:**
```json
{
  "type": "contract",
  "ledger": "12346000",
  "contract_id": "CEPHEMERALACCOUNTCONTRACTID000000000000000000000000000",
  "topic": ["AAAADwAAAAdleHBpcmVkAA=="],
  "value": "AAAAEAAAAAMAAAAPAAAAEHJlY292ZXJ5X2FkZHJlc3MAAAASAAAAAgAAAA8AAAAPYWlvdW50X3JldHVybmVkAAAACAAAAA=="
}
```
> The base64 values above are illustrative.

---

### ReserveReclaimed (topic: `"reserve"`)

**Emitted by:** `reclaim_reserve`  
**Trigger:** The XLM base reserve held by the ephemeral account is reclaimed, either partially or fully.  
**Topic:** `["reserve"]` (symbol_short)

**Data fields:**

| Field | Rust Type | XDR ScVal Type | Description |
|---|---|---|---|
| `destination` | `Address` | `ScVal::Address` | Address that received the reclaimed reserve |
| `amount` | `i128` | `ScVal::I128` | Amount of XLM reclaimed in this operation |
| `sweep_id` | `u64` | `ScVal::U64` | Identifier of the sweep operation this reclaim is associated with |
| `fully_reclaimed` | `bool` | `ScVal::Bool` | `true` if the entire reserve was reclaimed; `false` if partial |
| `remaining_reserve` | `i128` | `ScVal::I128` | Reserve balance still held after this reclaim (0 if fully reclaimed) |

**Example Horizon response:**
```json
{
  "type": "contract",
  "ledger": "12346050",
  "contract_id": "CEPHEMERALACCOUNTCONTRACTID000000000000000000000000000",
  "topic": ["AAAADwAAAAdyZXNlcnZlAA=="],
  "value": "AAAAEAAAAAUAAAAPAAAACmRlc3RpbmF0aW9uAAAAABIAAAACAAAADwAAAAZhbW91bnQAAAAAAAgA"
}
```
> The base64 values above are illustrative.

---

## SweepController Contract Events

### SweepCompleted (topic: `"sweep"`)

**Emitted by:** `execute_sweep`  
**Trigger:** The SweepController has successfully executed an atomic sweep from an ephemeral account to a destination.  
**Topic:** `["sweep"]` (symbol_short)

**Data fields:**

| Field | Rust Type | XDR ScVal Type | Description |
|---|---|---|---|
| `ephemeral_account` | `Address` | `ScVal::Address` | The ephemeral account that was swept |
| `destination` | `Address` | `ScVal::Address` | The permanent wallet that received the funds |
| `amount` | `i128` | `ScVal::I128` | Total amount swept |

**Example Horizon response:**
```json
{
  "type": "contract",
  "ledger": "12345691",
  "contract_id": "CSWEEPCONTROLLERCONTRACTID0000000000000000000000000000",
  "topic": ["AAAADwAAAAVzd2VlcAAAAA=="],
  "value": "AAAAEAAAAAMAAAAPAAAAEWVwaGVtZXJhbF9hY2NvdW50AAAAEgAAAAIAAAAPAAAAC2Rlc3RpbmF0aW9uAAAAABI="
}
```
> The base64 values above are illustrative.

---

### DestinationAuthorized (topic: `"dest_auth"`)

**Emitted by:** `authorize_destination`  
**Trigger:** A sweep destination address is authorized for a given ephemeral account.  
**Topic:** `["dest_auth"]` (symbol_short)

**Data fields:**

| Field | Rust Type | XDR ScVal Type | Description |
|---|---|---|---|
| `destination` | `Address` | `ScVal::Address` | The address that was authorized as a sweep destination |

**Example Horizon response:**
```json
{
  "type": "contract",
  "ledger": "12345685",
  "contract_id": "CSWEEPCONTROLLERCONTRACTID0000000000000000000000000000",
  "topic": ["AAAADwAAAAlkZXN0X2F1dGgAAAA="],
  "value": "AAAAEAAAAAEAAAAPAAAACmRlc3RpbmF0aW9uAAAAABI="
}
```
> The base64 values above are illustrative.

---

### DestinationUpdated (topic: `"dest_upd"`)

**Emitted by:** `update_destination`  
**Trigger:** The authorized sweep destination for an ephemeral account is changed.  
**Topic:** `["dest_upd"]` (symbol_short)

**Data fields:**

| Field | Rust Type | XDR ScVal Type | Description |
|---|---|---|---|
| `old_destination` | `Option<Address>` | `ScVal::Void` or `ScVal::Address` | Previous destination; `ScVal::Void` if none was set |
| `new_destination` | `Address` | `ScVal::Address` | The newly authorized destination address |

**Example Horizon response:**
```json
{
  "type": "contract",
  "ledger": "12345688",
  "contract_id": "CSWEEPCONTROLLERCONTRACTID0000000000000000000000000000",
  "topic": ["AAAADwAAAAhkZXN0X3VwZAAAAA=="],
  "value": "AAAAEAAAAAIAAAAPAAAADm9sZF9kZXN0aW5hdGlvbgAAAAEAAAAPAAAAD25ld19kZXN0aW5hdGlvbgAAAAAS"
}
```
> The base64 values above are illustrative.

---

## XDR Type Reference

| Rust Type | XDR ScVal Type | Notes |
|---|---|---|
| `Address` | `ScVal::Address` | `AccountId` for `G...` addresses; `ContractId` for `C...` addresses |
| `i128` | `ScVal::I128` | 128-bit signed integer, encoded as `Int128Parts { hi, lo }` |
| `u32` | `ScVal::U32` | 32-bit unsigned integer |
| `u64` | `ScVal::U64` | 64-bit unsigned integer |
| `bool` | `ScVal::Bool` | `true` or `false` |
| `Vec<T>` | `ScVal::Vec` | Array of `ScVal`; each element encoded by its own type rule |
| `Option<T>` | `ScVal::Void` or `T` | `ScVal::Void` when `None`; the inner type's encoding when `Some(v)` |
