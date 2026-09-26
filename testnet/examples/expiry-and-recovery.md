# Worked Example: Letting an Account Expire and Verifying the Recovery Path

## Overview

This walkthrough takes an `EphemeralAccount` from creation with a short expiry, lets the ledger pass it, triggers the recovery path, and verifies what the `AccountExpired` event actually tells you.

> **Read the "What the event does and does not prove" section before you rely on any of this for real funds.** The current MVP implementation performs the *state transition and event emission* for the recovery path but does **not** move SEP-41 tokens. This is the single most important thing to understand about expiry recovery today.

The near-future-expiry technique that makes this testable in minutes rather than hours is in `testnet/config/expiry-ledger-testing.md`.

---

## Preconditions

- An `EphemeralAccount` WASM hash and the ability to deploy a fresh instance
- A **funded** `recovery_address` — it must exist on-ledger and hold XLM for rent, or you will not be able to receive or re-deploy anything with it
- A funded identity to submit transactions
- Contract IDs from `testnet/registry/contract-status.md`

---

## Step 1: Create an Account With a ~5-Minute Expiry

`expiry_ledger` is a **ledger sequence number**, not a timestamp. Testnet closes a ledger roughly every 5 seconds, so ~5 minutes is about 60 ledgers.

```bash
# Read the live ledger
CURRENT_LEDGER=$(stellar network current-ledger \
  --rpc-url https://soroban-testnet.stellar.org)
echo "current ledger: $CURRENT_LEDGER"

# ~5 minutes out
EXPIRY=$((CURRENT_LEDGER + 60))
echo "expiry ledger:  $EXPIRY"
```

Two rules govern the value:

- `initialize()` rejects `expiry_ledger <= current_ledger` with `InvalidExpiry` (5). It must be **strictly** in the future.
- `is_expired()` returns `current_ledger >= expiry_ledger` — **greater than or equal**. So at exactly `expiry_ledger` the account is already expired. Do not plan to sweep in the final ledger.

Deploy and initialize:

```bash
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/ephemeral_account.wasm \
  --network testnet --source testnet-deployer

stellar contract invoke \
  --id "$NEW_EPH_ID" \
  --network testnet --source testnet-deployer \
  -- initialize \
  --creator "$CREATOR" \
  --expiry_ledger "$EXPIRY" \
  --recovery_address "$RECOVERY" \
  --authorized_controller "$SWEEP_CONTROLLER_ID" \
  --admin "$ADMIN"
```

All six parameters are required and set exactly once — `initialize()` returns `AlreadyInitialized` (1) on any second call, and there is **no way to change `recovery_address` or `expiry_ledger` afterwards**. Get them right at creation.

### The Recovery Address Is the Whole Point

Everything in this document depends on one value chosen at `initialize()` time. If it is wrong, the account has no recovery path at all:

| Mistake | Consequence |
|---|---|
| Points at an unfunded/nonexistent address | Funds have nowhere to go |
| Points at a key you do not control | Funds are unrecoverable by you |
| Points at a per-account throwaway | You must fund N addresses to support N accounts |
| Points at the ephemeral account itself | Funds stay exactly where they were |

`recovery_address` is also **not** `swept_to`. Before a sweep or expiry, `swept_to` is `None`. After either, it holds whichever address received the routing.

---

## Step 2: Fund It and Record a Payment

```bash
# Actual token balance must exist on the ephemeral account
# (transfer the token in from a funded account first)

stellar contract invoke \
  --id "$NEW_EPH_ID" --network testnet --source testnet-payer \
  -- record_payment --amount 100000000 --asset "$USDC_ASSET"
```

Confirm the pre-expiry state:

```bash
stellar contract invoke --id "$NEW_EPH_ID" --network testnet \
  --source testnet-investigator -- is_expired
# → false

stellar contract invoke --id "$NEW_EPH_ID" --network testnet \
  --source testnet-investigator -- get_status
# → 1 (PaymentReceived)
```

---

## Step 3: Wait for the Ledger to Pass

Poll rather than sleeping a fixed amount — network conditions vary:

```bash
while true; do
  L=$(stellar network current-ledger --rpc-url https://soroban-testnet.stellar.org)
  if [ "$L" -ge "$EXPIRY" ]; then echo "expired at ledger $L"; break; fi
  echo "ledger $L / $EXPIRY ..."; sleep 10
done
```

Then confirm the contract agrees:

```bash
stellar contract invoke --id "$NEW_EPH_ID" --network testnet \
  --source testnet-investigator -- is_expired
# → true
```

There is no keeper job and no automatic action at the expiry ledger. **Expiry is a condition, not an event.** Nothing happens on-chain until someone calls a function. That is deliberate — `expire()` is permissionless precisely so funds cannot be stranded waiting for a keeper.

---

## Step 4: Trigger the Recovery Path

There are two entry points. They are equivalent in effect and both emit the same event.

### `expire()` — permissionless

Anyone may call it, including an identity that has no relationship to the account. No auth required.

```bash
stellar contract invoke \
  --id "$NEW_EPH_ID" --network testnet --source testnet-investigator \
  -- expire
```

### `recover()` — creator or recovery address only

```bash
stellar contract invoke \
  --id "$NEW_EPH_ID" --network testnet --source testnet-creator \
  -- recover --caller "$CREATOR"
```

`recover()` checks `caller == creator || caller == recovery_address` and then calls `caller.require_auth()`. So `--caller` must be an address whose secret the submitting identity holds, and a third party calling with someone else's address gets `Unauthorized` (8).

**Use `expire()` unless you specifically need the auth-based variant.** Requiring auth here buys nothing and adds a failure mode.

### Failure Modes

| Call | Condition | Result |
|---|---|---|
| `expire()` | Before `expiry_ledger` | `NotExpired` (6) |
| `expire()` | Status already `Swept` or `Expired` | `InvalidStatus` (12) |
| `recover()` | `caller` is neither creator nor recovery address | `Unauthorized` (8) |
| `recover()` | Before `expiry_ledger` | `NotExpired` (6) |
| Either | Status already `Swept` or `Expired` | `InvalidStatus` (12) |
| Either | Never initialized | `NotInitialized` (2) |

Recovery is **one-shot**. Once status is `Expired`, neither call can be repeated.

---

## Step 5: Read the `AccountExpired` Event

```bash
stellar contract events \
  --contract-id "$NEW_EPH_ID" \
  --start-ledger "$START_LEDGER" \
  --filter '{"topics": [["expired"]]}' \
  --network testnet
```

### Payload

```rust
struct AccountExpired {
    recovery_address: Address,   // the configured recovery_address
    amount_returned: i128,       // sum of all recorded payment amounts
    reserve_amount: i128,        // reserve bookkeeping reclaimed in THIS call
}
```

Immediately after, a `ReserveReclaimed` event (topic `reserve`) follows, since `expire()` reclaims the tracked base reserve:

```rust
struct ReserveReclaimed {
    destination: Address,
    amount: i128,
    sweep_id: u64,              // = the ledger sequence of the expire() call
    fully_reclaimed: bool,
    remaining_reserve: i128,
}
```

`sweep_id` on an expiry is the ledger in which `expire()` landed. That is the anchor for correlating the two events.

### Decoding It

```typescript
import { scValToNative } from '@stellar/stellar-sdk';

function decodeAccountExpired(data: unknown) {
  const { recovery_address, amount_returned, reserve_amount } =
    scValToNative(data) as any;

  return {
    recoveryAddress: recovery_address as string,
    // i128 decodes to a string. Keep BigInt; do not pass through Number().
    amountReturned: BigInt(amount_returned),
    reserveAmount: BigInt(reserve_amount),
  };
}
```

Verify the address matches what you configured — this is the check that matters most:

```typescript
if (decoded.recoveryAddress !== EXPECTED_RECOVERY) {
  throw new Error(`recovery routing mismatch: ${decoded.recoveryAddress}`);
}
```

`amount_returned` is a **sum across assets**, and a multi-asset account makes it meaningless as a monetary figure. It is the arithmetic total of the recorded `Payment` amounts, not a verified transfer. See `testnet/examples/multi-asset-sweep.md`.

---

## What the Event Does and Does Not Prove

This is the critical section.

### What it proves

- The account was `Active`/`PaymentReceived` and is now `Expired`.
- The ledger had passed `expiry_ledger` when the call landed.
- The contract's `swept_to` is now set to `recovery_address`.
- The reserve bookkeeping was decremented and a `ReserveReclaimed` event was recorded.
- The one-shot recovery transition has been consumed.

### What it does not prove

**No SEP-41 tokens were moved.** `EphemeralAccount` contains no `TokenClient::transfer()` call anywhere. `expire()` and `recover()` set state and emit events; the reserve reclaim is likewise bookkeeping, not a transfer — `reclaim_reserve_to()` only updates counters and emits. The contract's own comment is explicit: *"Actual token transfers happen in the SDK via Stellar SDK. This contract enforces authorization/state transitions and reserve lifecycle."*

So after a successful `expire()`:

- The ephemeral account **still holds** the token balances it held before.
- The `recovery_address` balance is **unchanged**.
- `SweepCompleted` was **not** emitted — that event comes from `SweepController`, and only `execute_sweep()`/`claim()` emit it.

Concretely:

```bash
# The funds are still sitting on the ephemeral account
stellar contract invoke --id "$NEW_EPH_ID" --network testnet \
  --source testnet-investigator -- get_info
# status: 3 (Expired)
# swept_to: GAEG3...  ← the recovery address, as a routing record
# payments: unchanged — recording is permanent history
```

Treat `AccountExpired` as *"the contract has marked this account as recovered-in-principle and recorded the intended destination"*, not as *"funds arrived"*. The only component that actually moves on-chain in the MVP is nothing.

### Practical Consequences

- **Do not reconcile balances off `AccountExpired`.** Verify balances directly at both addresses.
- **Do not treat expiry as a safety net for real funds.** In the MVP, real token movement happens only through `SweepController::execute_sweep()` / `::claim()`, and both refuse to run once an account is expired. An expired account is effectively a dead end for the sweep path.
- **Sweep before expiry.** Design your integration to sweep well inside the window, and use the recovery path as a diagnostic and accounting signal rather than a funds-recovery mechanism.

A detailed checklist for confirming where balances actually ended up is in `testnet/runbooks/expired-account-recovery-check.md` (if it is not in your checkout yet, the verification steps are reproduced in Step 6 above).

---

## Step 6: Verify End State

```bash
stellar contract invoke --id "$NEW_EPH_ID" --network testnet \
  --source testnet-investigator -- get_status
# → 3 (Expired)

stellar contract invoke --id "$NEW_EPH_ID" --network testnet \
  --source testnet-investigator -- get_info
# swept_to == recovery_address   (routing record; not a balance transfer)

# Reserve bookkeeping
stellar contract invoke --id "$NEW_EPH_ID" --network testnet \
  --source testnet-investigator -- get_reserve_remaining
# → 0        (1_000_000_000 stroops tracked and fully reclaimed)

stellar contract invoke --id "$NEW_EPH_ID" --network testnet \
  --source testnet-investigator -- is_reserve_reclaimed
# → true

stellar contract invoke --id "$NEW_EPH_ID" --network testnet \
  --source testnet-investigator -- get_last_reserve_event
# → ReserveReclaimed { destination: <recovery>, amount: 1000000000,
#                      sweep_id: <ledger>, fully_reclaimed: true,
#                      remaining_reserve: 0 }
```

`initialize()` seeds reserve tracking at `BASE_RESERVE_STROOPS = 1_000_000_000` stroops (1 XLM), and `expire()` reclaims it in the same call, so `reserve_amount` in `AccountExpired` is normally the full 1 XLM. This is a **counter**, not a balance — no XLM is transferred.

### Repeated Reclaim Is Safe

```bash
# Idempotent: returns 0 rather than erroring
stellar contract invoke --id "$NEW_EPH_ID" --network testnet \
  --source testnet-investigator -- reclaim_reserve
# → 0
```

`reclaim_reserve()` is valid for any `Swept` or `Expired` account and transfers 0 once fully reclaimed. It still emits a `ReserveReclaimed` event with `amount: 0` and `fully_reclaimed: true`, so **event count is not a reliable proxy for "how much moved"** — filter on `amount`, not on cardinality.

---

## Post-Expiry: What No Longer Works

| Operation | Result |
|---|---|
| `sweep()` via controller | `AccountExpired` (11) |
| `claim()` via controller | Fails — `sweep_claim()` hits the same expiry guard |
| `expire()` / `recover()` again | `InvalidStatus` (12) |
| `record_payment()` | **Still succeeds** — it has no status or expiry guard |
| `simulate_sweep()` | Returns `(empty, 11)` |
| `can_sweep()` | `false` |

`record_payment()` accepting new payments on an expired account is a real gap: entries can be appended to an account that can no longer be swept. If you are monitoring an account you care about, gate on `is_expired()` before recording, in your own code.

---

## Common Pitfalls

| Pitfall | Symptom | Fix |
|---|---|---|
| Passing a Unix timestamp as `expiry_ledger` | `InvalidExpiry`, or instant expiry | Ledger sequence only |
| Off-by-one at the boundary | Sweep works, then suddenly `AccountExpired` | `is_expired()` is `>=`; leave a wide margin |
| Assuming funds moved | `AccountExpired` seen, balance unchanged | Expected in the MVP — see the section above |
| Unfunded `recovery_address` | Nothing to receive, nothing to re-deploy with | Fund it before creating accounts |
| Sleeping a fixed duration | Test flakes as ledger timing drifts | Poll `is_expired()` |
| Expecting an automatic event at expiry | No events until you call something | `expire()` is permissionless and manual |
| `swept_to` read as "funds went here" | Wrong conclusion | It is a routing record only |

---

## Related Documentation

- `testnet/config/expiry-ledger-testing.md` — near-future expiry computation
- `testnet/runbooks/expired-account-recovery-check.md` — verifying where funds actually ended up
- `testnet/runbooks/stuck-ephemeral-account.md` — accounts that will not expire or sweep
- `testnet/registry/event-topics.md` — `expired` and `reserve` topic encoding
- `testnet/registry/known-test-accounts.md` — long-lived fixtures with far-future expiry
- `docs/api-reference.md` — `expire`, `recover`, `reclaim_reserve` references
- `docs/security.md` — expiration threat model

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-26 | Initial expiry and recovery walkthrough |
