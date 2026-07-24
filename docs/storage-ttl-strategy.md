# Storage TTL Strategy

## Overview

Soroban storage entries are classified into three categories based on their
lifetime and TTL (time-to-live) behaviour.  This document classifies every
storage entry in bridgelet-core and documents the TTL extension strategy.

## Storage Classification

### Instance Storage

Instance storage lives as long as the contract instance itself.  TTL is
managed by the Soroban runtime and does not require manual extension.
Instance storage is the most cost-effective for data that should persist
for the contract's entire lifetime.

| Contract | Key | Purpose | TTL |
|----------|-----|---------|-----|
| EphemeralAccount | `Initialized` | Whether the contract has been initialized | Instance |
| EphemeralAccount | `Creator` | Address that created the account | Instance |
| EphemeralAccount | `ExpiryLedger` | Ledger at which account expires | Instance |
| EphemeralAccount | `RecoveryAddress` | Address to return funds on expiry | Instance |
| EphemeralAccount | `Payments` | Map of recorded payments | Instance |
| EphemeralAccount | `Status` | Current lifecycle status | Instance |
| EphemeralAccount | `SweptTo` | Destination of sweep/expiry | Instance |
| EphemeralAccount | `BaseReserveRemaining` | Remaining base reserve | Instance |
| EphemeralAccount | `AvailableReserve` | Available reserve for transfer | Instance |
| EphemeralAccount | `ReserveReclaimed` | Whether reserve fully reclaimed | Instance |
| EphemeralAccount | `LastSweepId` | Sequence of last sweep | Instance |
| EphemeralAccount | `ReserveEventCount` | Number of reserve events | Instance |
| EphemeralAccount | `LastReserveEvent` | Most recent reserve event | Instance |
| EphemeralAccount | `AuthorizedController` | Sweep controller address | Instance |
| EphemeralAccount | `Admin` | Upgrade authority | Instance |
| SweepController | `AuthorizedSigner` | Ed25519 public key | Instance |
| SweepController | `SweepNonce` | Replay-prevention nonce | Instance |
| SweepController | `AuthorizedDestination` | Locked destination (optional) | Instance |
| SweepController | `Creator` | Contract creator | Instance |
| SweepController | `PendingSigner` | Pending new signer | Instance |
| SweepController | `PendingSignerEffectiveLedger` | Effective ledger for pending signer | Instance |
| AccountFactory | `EphemeralAccountWasmHash` | WASM hash for deployments | Instance |

### Persistent Storage

Persistent storage survives across transactions and requires explicit TTL
extension.  Currently, bridgelet-core does **not** use persistent storage —
all entries are instance-scoped.

If persistent storage is added in the future (e.g., for historical sweep
logs), the following strategy applies:

- **TTL extension threshold**: When remaining TTL drops below 100,000
  ledgers (~6 days), extend by 500,000 ledgers (~35 days).
- **Extension trigger**: Called at the start of every transaction that
  reads or writes the persistent entry.
- **Cost awareness**: Each TTL extension costs 1 stroop per ledger
  extended.  Only extend entries that are actively queried.

### Temporary Storage

Temporary storage is automatically removed by the Soroban runtime when its
TTL expires.  Bridgelet-core does **not** use temporary storage.

## TTL Extension Logic

For future use, here is the recommended TTL extension pattern:

```rust
use soroban_sdk::Env;

const TTL_THRESHOLD: u32 = 100_000; // ~6 days
const TTL_EXTENSION: u32 = 500_000; // ~35 days

pub fn maybe_extend_ttl(env: &Env, key: &soroban_sdk::Val) {
    let current_ttl = env.storage().persistent().get_ttl(key);
    if current_ttl < TTL_THRESHOLD {
        env.storage().persistent().extend_ttl(key, TTL_EXTENSION);
    }
}
```

## Design Rationale

All bridgelet-core storage entries use **instance** storage because:

1. **Ephemeral accounts are short-lived** — they expire after a configurable
   ledger count (typically hours to days).  Instance storage is cheaper than
   persistent storage and automatically cleaned up when the contract
   instance is archived.

2. **No historical data needed** — sweep and expiry events are emitted as
   Soroban events and can be indexed off-chain.  There is no need to store
   historical data in contract storage.

3. **Simplicity** — instance storage requires no TTL management, reducing
   code complexity and storage costs.

4. **Cost efficiency** — instance storage is included in the contract's
   base rent, while persistent storage requires per-entry fees.
