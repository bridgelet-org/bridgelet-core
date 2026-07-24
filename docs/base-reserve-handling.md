# Stellar Base Reserve Handling in Sweep Operations

## Background

On the Stellar network, every account must maintain a minimum **base reserve**
(currently 0.5 XLM / 5,000,000 stroops per sub-entry, with a minimum
account balance of 2 base reserves = 1 XLM / 10,000,000 stroops).  When an
ephemeral account is created, the SDK funds it with enough XLM to cover
the base reserve plus any payment amounts.

The base reserve **cannot be transferred** using a normal `payment()` call.
It can only be released via one of:

1. **AccountMerge** (`merge_account`) — merges the entire account balance
   into another account, closing the source account and releasing the reserve.
2. **Soroban contract-level tracking** — the contract tracks the reserve
   amount logically and reclaims it by authorising a separate Stellar
   operation after the sweep is complete.

## Implementation in bridgelet-core

### EphemeralAccount reserve lifecycle

The `EphemeralAccountContract` tracks the base reserve through four
storage entries:

| Key                       | Description                                    |
|---------------------------|------------------------------------------------|
| `BaseReserveRemaining`    | Total base reserve still unreclaimed (stroops) |
| `AvailableReserve`        | Portion of the reserve available for immediate |
|                           | transfer (may be less than remaining if tokens |
|                           | are still locked in sub-entries)               |
| `ReserveReclaimed`        | `true` once `BaseReserveRemaining == 0`        |
| `LastSweepId`             | Sequence number of the last sweep/expiry       |

### When is the reserve reclaimed?

The reserve is reclaimed as part of **every sweep or expiry operation**:

1. **`sweep()` / `sweep_claim()`** — After the account status transitions
   to `Swept`, `reclaim_reserve_to()` is called to transfer the available
   reserve to the sweep destination.  The event `ReserveReclaimed` is
   emitted with the reclaimed amount and remaining balance.

2. **`expire()` / `recover()`** — After the account status transitions
   to `Expired`, the same `reclaim_reserve_to()` function transfers the
   available reserve to the recovery address.

3. **`reclaim_reserve()`** — A standalone function that can be called
   separately to reclaim any remaining reserve that could not be
   transferred during the initial sweep (e.g., if the available reserve
   was less than the remaining reserve due to sub-entry locks).  This
   function is idempotent — calling it after full reclaim transfers 0.

### Partial reclaim

If the `AvailableReserve` is less than `BaseReserveRemaining` (which
happens when some of the reserve is locked by sub-entries that haven't
been released yet), only the available portion is transferred.  The
caller can invoke `reclaim_reserve()` again later to collect the
remainder once sub-entries are cleared.

### Atomicity

All reserve state transitions happen within the same Soroban transaction
as the payment transfers.  If any step fails, the entire transaction
reverts — including the reserve reclaim — ensuring no funds are lost
or double-spent.

### Cross-contract flow

```
SDK                    SweepController          EphemeralAccount
 │                          │                        │
 ├─ execute_sweep() ───────>│                        │
 │                          ├─ verify auth           │
 │                          ├─ increment nonce       │
 │                          ├─ authorize_ephemeral ─>│
 │                          │                        ├─ sweep()
 │                          │                        │   ├─ verify controller auth
 │                          │                        │   ├─ status → Swept
 │                          │                        │   ├─ reclaim_reserve_to(dest)
 │                          │                        │   └─ emit events
 │                          ├─ transfers::execute ──>│   (token transfers)
 │                          └─ emit sweep_completed  │
 │                                                  │
```

### Integration test verification

The test `test_sweep_reclaims_base_reserve_success_lifecycle` in
`ephemeral_account/src/test.rs` verifies that:

- After sweep, `get_reserve_remaining()` returns 0
- `is_reserve_reclaimed()` returns `true`
- A `ReserveReclaimed` event is emitted with the full base reserve amount
- The event's `sweep_id` matches the current ledger sequence
- `get_reserve_reclaim_event_count()` is 1

The test `test_reserve_double_claim_prevention` verifies that calling
`reclaim_reserve()` after a full reclaim returns 0 and emits a zero-amount
event without changing the on-chain state.
