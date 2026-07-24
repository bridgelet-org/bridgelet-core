# Glossary Entry: Reserve Reclaim Lifecycle

**Path:** `bridgelet-audit/glossary/reserve-reclaim.md`  
**Component:** `EphemeralAccount`  
**Related Functions:** `reclaim_reserve()`, `reclaim_reserve_to()`, `get_reserve_remaining()`, `get_reserve_available()`, `is_reserve_reclaimed()`

---

## Overview

The **Reserve Reclaim Lifecycle** governs how base-reserve XLM funds allocated to an `EphemeralAccount` contract upon deployment are tracked and recovered after the account's primary token lifecycle completes (via a sweep or expiration).

In Stellar/Soroban, smart contracts hold a minimum balance (base reserve) denominated in stroops ($1 \text{ XLM} = 10^7 \text{ stroops}$, or $1_000_000_000$ stroops depending on contract configuration; `EphemeralAccount` uses `BASE_RESERVE_STROOPS = 1_000_000_000`). Once an account transitions to `Swept` or `Expired`, its base reserve is returned to the designated destination address.

---

## State Machine: `base_reserve_remaining` vs `available_reserve`

The contract maintains two internal state variables for reserve accounting:

1. **`base_reserve_remaining` (Total Left to Reclaim)**:
   - Represents the total un-reclaimed reserve liability of the account.
   - Initialized to `BASE_RESERVE_STROOPS` ($1_000_000_000$ stroops) upon contract `initialize()`.
   - Monotonically decreases toward `0` as reserve payouts succeed.
   - When `base_reserve_remaining == 0`, the flag `is_reserve_reclaimed` becomes `true`.

2. **`available_reserve` (Currently Reclaimable)**:
   - Represents the liquid XLM balance stroops currently present in the contract's local storage/balance buffer available for immediate payout.
   - Initialized to `BASE_RESERVE_STROOPS` ($1_000_000_000$ stroops) upon contract `initialize()`.
   - In partial-payout scenarios (e.g. if liquid balance was partially drawn or locked), `available_reserve` may temporarily be less than `base_reserve_remaining`.

### Operational Formula for Reclaim Amount
When `reclaim_reserve_to(env, destination, sweep_id)` is invoked:
$$\text{reclaim\_amount} = \min(\text{available\_reserve}, \text{base\_reserve\_remaining})$$

State updates applied atomically:
$$\text{new\_available} = \text{available\_reserve} - \text{reclaim\_amount}$$
$$\text{new\_remaining} = \text{base\_reserve\_remaining} - \text{reclaim\_amount}$$
$$\text{fully\_reclaimed} = (\text{new\_remaining} == 0)$$

---

## Worked Example: Reclaim Lifecycle Walkthrough

### Scenario
An `EphemeralAccount` is initialized with $1_000_000_000$ stroops ($100 \text{ XLM}$ base reserve equivalent in test units).

```
Initial State:
- status: Active
- base_reserve_remaining: 1,000,000,000
- available_reserve:      1,000,000,000
- is_reserve_reclaimed:   false
```

### Call 1: Partial Reclaim (e.g., initial sweep with restricted available balance)
Suppose `available_reserve` is capped at $400_000_000$ stroops at the time of sweep.
- `sweep()` is called. `reclaim_reserve_to()` executes:
  - $\text{reclaim\_amount} = \min(400_000_000, 1_000_000_000) = 400_000_000$
  - `new_available` = $400_000_000 - 400_000_000 = 0$
  - `new_remaining` = $1_000_000_000 - 400_000_000 = 600_000_000$
  - `fully_reclaimed` = `false`
- Event Emitted: `ReserveReclaimed { destination: G..., amount: 400000000, sweep_id: 101, fully_reclaimed: false, remaining_reserve: 600000000 }`
- Result: Returns `Ok(400000000)`.

### Call 2: Secondary Top-Up & Full Reclaim
Additional reserve balance becomes available ($600_000_000$ stroops added to `available_reserve`).
- `reclaim_reserve()` is invoked manually or by off-chain sweeper:
  - $\text{reclaim\_amount} = \min(600_000_000, 600_000_000) = 600_000_000$
  - `new_available` = $600_000_000 - 600_000_000 = 0$
  - `new_remaining` = $600_000_000 - 600_000_000 = 0$
  - `fully_reclaimed` = `true`
- Event Emitted: `ReserveReclaimed { destination: G..., amount: 600000000, sweep_id: 101, fully_reclaimed: true, remaining_reserve: 0 }`
- Result: Returns `Ok(600000000)`.

### Call 3: Subsequent Call (No-Op)
- `reclaim_reserve()` is called again on the fully reclaimed account:
  - `base_reserve_remaining` is `0`.
  - Event Emitted: `ReserveReclaimed { destination: G..., amount: 0, sweep_id: 101, fully_reclaimed: true, remaining_reserve: 0 }`
  - Result: Returns `Ok(0)` without error or state mutation. Safe to call idempotently.

---

## Off-Chain Reconciliation: `ReserveReclaimed` Event Fields

Each reserve reclaim execution publishes a `ReserveReclaimed` event for indexers and accounting services:

| Field Name | Type | Description for Off-Chain Reconciliation |
| :--- | :--- | :--- |
| `destination` | `Address` | Wallet or contract address receiving the reclaimed XLM reserve stroops. Matches `swept_to` or `recovery_address`. |
| `amount` | `i128` | Stroops transferred in *this specific transaction*. Used by bookkeepers to record exact incremental inflow. |
| `sweep_id` | `u64` | Ledger sequence number at the time of sweep/expiration execution. Links the reserve payout to the specific sweep event. |
| `fully_reclaimed` | `bool` | Flag indicating whether the reserve liability is completely cleared (`true`) or if outstanding reserve remains (`false`). |
| `remaining_reserve` | `i128` | Outstanding reserve balance (in stroops) remaining on-chain after this call completes. |

---

## Key Security & Architecture Takeaways

1. **Reentrancy Guard**: `EphemeralAccount` updates `status = Swept` / `Expired` *before* invoking `reclaim_reserve_to()`.
2. **Idempotence**: `reclaim_reserve()` can be retried safely if network issues interrupt a multi-step settlement.
3. **Auditability**: Total events emitted can be inspected via `get_reserve_reclaim_event_count()` and last event retrieved via `get_last_reserve_event()`.
