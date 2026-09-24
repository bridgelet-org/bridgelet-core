# Testnet Contract Event Topics Reference

## Overview

This document specifies the exact event topic encoding for all events emitted by Bridgelet contracts deployed on Stellar Testnet. Indexers should use these topics to filter and subscribe to contract events via Horizon/RPC without reverse-engineering from contract source.

## Event Topic Format

All events use the standard Soroban event format:
- **Topics**: Array of `ScVal` where the first element is the event name (symbol)
- **Data**: Event payload as `ScVal` (contracttype-serialized struct)

For filtering in Horizon/RPC, use the **first topic** (event name symbol) as the primary filter.

---

## EphemeralAccount Contract Events

**Contract ID**: See `testnet/registry/contract-status.md` for deployed address

| Event Name (Topic[0]) | Symbol | Payload Struct | Trigger |
|---|---|---|---|
| `created` | `created` | `AccountCreated { creator: Address, expiry_ledger: u32 }` | `initialize()` success |
| `payment` | `payment` | `PaymentReceived { amount: i128, asset: Address }` | First `record_payment()` call |
| `multi_pay` | `multi_pay` | `MultiPaymentReceived { asset: Address, amount: i128 }` | Subsequent `record_payment()` calls |
| `swept_mul` | `swept_mul` | `SweepExecutedMulti { destination: Address, payments: Vec<Payment> }` | `sweep()` / `sweep_claim()` success |
| `expired` | `expired` | `AccountExpired { recovery_address: Address, total_amount: i128, reserve_amount: i128 }` | `expire()` / `recover()` success |
| `reserve` | `reserve` | `ReserveReclaimed { destination: Address, amount: i128, sweep_id: u64, fully_reclaimed: bool, remaining_reserve: i128 }` | After each sweep/expire that transfers reserve |

### Example: Filtering AccountCreated Events

```bash
# Using stellar-cli to get events for a specific contract
stellar contract events \
  --contract-id <EPHEMERAL_ACCOUNT_ID> \
  --start-ledger 1000000 \
  --filter '{"topics": [["created"]]}' \
  --network testnet
```

### Payload Structures

```rust
// AccountCreated
struct AccountCreated {
    creator: Address,        // 32-byte account ID
    expiry_ledger: u32,      // Ledger sequence when account expires
}

// PaymentReceived
struct PaymentReceived {
    amount: i128,            // Payment amount in asset base units
    asset: Address,          // Token contract address
}

// MultiPaymentReceived
struct MultiPaymentReceived {
    asset: Address,          // Token contract address
    amount: i128,            // Payment amount in asset base units
}

// SweepExecutedMulti
struct SweepExecutedMulti {
    destination: Address,    // Recipient address
    payments: Vec<Payment>,  // All payments swept
}

// Payment (nested)
struct Payment {
    asset: Address,
    amount: i128,
    timestamp: u64,          // Ledger timestamp at record_payment
}

// AccountExpired
struct AccountExpired {
    recovery_address: Address,
    total_amount: i128,      // Sum of all payments
    reserve_amount: i128,    // Reserve reclaimed in this call
}

// ReserveReclaimed
struct ReserveReclaimed {
    destination: Address,    // Where reserve was sent
    amount: i128,            // Amount reclaimed this call (stroops)
    sweep_id: u64,           // Ledger sequence of sweep/expire
    fully_reclaimed: bool,   // True if reserve fully reclaimed
    remaining_reserve: i128, // Reserve still awaiting reclaim
}
```

---

## SweepController Contract Events

**Contract ID**: See `testnet/registry/contract-status.md` for deployed address

| Event Name (Topic[0]) | Symbol | Payload Struct | Trigger |
|---|---|---|---|
| `sweep` | `sweep` | `SweepCompleted { ephemeral_account: Address, destination: Address, amount: i128 }` | `execute_sweep()` or `claim()` success |
| `dest_auth` | `dest_auth` | `DestinationAuthorized { destination: Address }` | `initialize()` with non-None `authorized_destination` |
| `dest_upd` | `dest_upd` | `DestinationUpdated { old_destination: Option<Address>, new_destination: Address }` | `update_authorized_destination()` success |

### Payload Structures

```rust
// SweepCompleted
struct SweepCompleted {
    ephemeral_account: Address,  // EphemeralAccount contract ID
    destination: Address,        // Recipient address
    amount: i128,                // Total amount swept (all assets summed)
}

// DestinationAuthorized
struct DestinationAuthorized {
    destination: Address,        // Locked destination address
}

// DestinationUpdated
struct DestinationUpdated {
    old_destination: Option<Address>,  // Previous destination (None if first)
    new_destination: Address,          // New locked destination
}
```

---

## ReserveContract Events

**Contract ID**: Not currently deployed on testnet (see `testnet/registry/reserve-contract-status.md`)

| Event Name (Topic[0]) | Symbol | Payload Struct | Trigger |
|---|---|---|---|
| `initialized` | `initialized` | `ContractInitialized { admin: Address }` | `initialize()` success |
| `base_reserve_updated` | `base_reserve_updated` | `BaseReserveUpdated { old_value: i128, new_value: i128, admin: Address }` | `set_base_reserve()` success |

### Payload Structures

```rust
// ContractInitialized
struct ContractInitialized {
    admin: Address,
}

// BaseReserveUpdated
struct BaseReserveUpdated {
    old_value: i128,    // Previous base reserve (stroops), 0 if first set
    new_value: i128,    // New base reserve (stroops)
    admin: Address,     // Admin who made the change
}
```

---

## AccountFactory Events

**Contract ID**: Not currently deployed on testnet (see `testnet/registry/account-factory-status.md`)

AccountFactory currently emits **no events** directly. Account creation is tracked via the `AccountCreated` events emitted by each deployed `EphemeralAccount` instance.

---

## Indexer Integration Guide

### Horizon Event Streaming

```javascript
// JavaScript/TypeScript example using @stellar/stellar-sdk
import { Horizon } from '@stellar/stellar-sdk';

const horizon = new Horizon.Server('https://horizon-testnet.stellar.org');

// Stream events for EphemeralAccount contract
const es = horizon.events()
  .forContract('<EPHEMERAL_ACCOUNT_ID>')
  .cursor('now')
  .stream({
    onmessage: (event) => {
      const topic = event.topic[0]; // First topic is event name
      switch (topic) {
        case 'created':
          handleAccountCreated(event);
          break;
        case 'payment':
        case 'multi_pay':
          handlePaymentReceived(event);
          break;
        case 'swept_mul':
          handleSweepExecuted(event);
          break;
        case 'expired':
          handleAccountExpired(event);
          break;
        case 'reserve':
          handleReserveReclaimed(event);
          break;
      }
    },
    onerror: (err) => console.error(err),
  });
```

### Soroban RPC getEvents

```bash
# Get events for a contract within a ledger range
curl -X POST https://soroban-testnet.stellar.org \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getEvents",
    "params": {
      "startLedger": 1000000,
      "endLedger": 1001000,
      "filters": [{
        "type": "contract",
        "contractIds": ["<EPHEMERAL_ACCOUNT_ID>"],
        "topics": [["created"], ["payment"], ["multi_pay"]]
      }]
    }
  }'
```

### Topic Encoding Details

For programmatic filtering, topics are XDR-encoded `ScVal` arrays. The first topic is always a `ScSymbol` (event name):

```
Topic[0] = ScSymbol("created")     // 0x0e + "created" (7 bytes) = 8 bytes
Topic[0] = ScSymbol("payment")     // 0x0e + "payment" (7 bytes)
Topic[0] = ScSymbol("multi_pay")   // 0x0e + "multi_pay" (9 bytes)
Topic[0] = ScSymbol("swept_mul")   // 0x0e + "swept_mul" (9 bytes)
Topic[0] = ScSymbol("expired")     // 0x0e + "expired" (7 bytes)
Topic[0] = ScSymbol("reserve")     // 0x0e + "reserve" (7 bytes)

Topic[0] = ScSymbol("sweep")       // 0x0e + "sweep" (5 bytes)
Topic[0] = ScSymbol("dest_auth")   // 0x0e + "dest_auth" (9 bytes)
Topic[0] = ScSymbol("dest_upd")    // 0x0e + "dest_upd" (8 bytes)
```

---

## Testnet Deployment Status

| Contract | Deployed | Contract ID | Last Verified |
|---|---|---|---|
| EphemeralAccount | See `contract-status.md` | See `contract-status.md` | See `contract-status.md` |
| SweepController | See `contract-status.md` | See `contract-status.md` | See `contract-status.md` |
| ReserveContract | **No** | N/A | N/A |
| AccountFactory | **No** | N/A | N/A |

Check `testnet/registry/contract-status.md` for current deployment status and contract IDs.

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-24 | Initial event topic documentation for testnet |