# Worked Example: Multi-Asset Sweep via `SweepController::execute_sweep`

## Overview

An `EphemeralAccount` can record **one payment per SEP-41 asset** — up to 10 distinct assets. A sweep then moves every recorded payment in a single transaction. This walkthrough exercises that multi-asset path end to end, and covers the part that reliably confuses integrators: **how to read the resulting `SweepExecutedMulti` event's payment list**, and which of the two sweep events to believe.

> The test tokens and amounts to use are catalogued in `testnet/config/multi-asset-testing.md`. If that document is not yet present in your checkout, the placeholders below are enough to run the flow with any two SEP-41 tokens on testnet.

---

## The Rules That Shape This Flow

Three constraints interact here, and all of them are enforced in `EphemeralAccount::record_payment()`:

| Rule | Limit | Error |
|---|---|---|
| One payment per asset | Second call for the same `asset` address is rejected | `DuplicateAsset` (13) |
| Hard cap on distinct assets | 11th distinct asset rejected | `TooManyPayments` (14) |
| Amount must be positive | `amount <= 0` rejected | `InvalidAmount` (4) |

"Multi-asset" means one payment **per asset**, not multiple payments of the same asset. If you need to accumulate several payments of one token, that is not what this design does — use one `EphemeralAccount` per payment. Full behaviour table: `testnet/runbooks/multi-payment-edge-cases.md`.

The duplicate check runs before the amount check, so re-submitting a bad amount for an already-recorded asset still returns `DuplicateAsset`.

---

## Step 1: Fund and Record Two Assets

Get two SEP-41 token contract IDs and the account's address:

```bash
EPHEMERAL_ID=<EPHEMERAL_ACCOUNT_ID>
XLM_ASSET=CA3D5KRYM6CB7OWQ6TWYRRULZAT3TCJ2Q6Q7HGFAXUVK2QMRMDQX3T5Y
USDC_ASSET=<USDC_SEP41_CONTRACT_ID>
```

The ephemeral account itself must hold a balance of each token for the transfer to succeed. Recording a payment does not move anything — it is a bookkeeping entry that the sweep later acts on. If the account has no balance, the sweep reaches step 4 and then fails with `TransferFailed`.

```bash
# First asset → PaymentReceived (topic "payment"), status flips Active → PaymentReceived
stellar contract invoke \
  --id "$EPHEMERAL_ID" --network testnet --source testnet-payer \
  -- record_payment --amount 100000000 --asset "$USDC_ASSET"

# Second asset → MultiPaymentReceived (topic "multi_pay"), status unchanged
stellar contract invoke \
  --id "$EPHEMERAL_ID" --network testnet --source testnet-payer \
  -- record_payment --amount 2500000000 --asset "$XLM_ASSET"
```

Note the asymmetry in event names: the **first** payment emits `PaymentReceived`, every subsequent one emits `MultiPaymentReceived`. An indexer that only watches `payment` will miss every asset after the first. Full topic list: `testnet/registry/event-topics.md`.

### Confirm What Was Recorded

```bash
stellar contract invoke \
  --id "$EPHEMERAL_ID" --network testnet --source testnet-investigator \
  -- get_info
# payment_received: true
# payment_count: 2
# payments: [ { asset: USDC, amount: 100000000, timestamp: ... },
#             { asset: XLM,  amount: 2500000000, timestamp: ... } ]

# Cheaper alternative when you only need the count
stellar contract invoke \
  --id "$EPHEMERAL_ID" --network testnet --source testnet-investigator \
  -- get_payment_count
# → 2
```

`get_info()` returns every payment unpaginated. `get_info_paginated()` is the bounded variant — it clamps `limit` to 1..100 and uses `NO_CURSOR` (`4294967295`) for "first page":

```bash
stellar contract invoke \
  --id "$EPHEMERAL_ID" --network testnet --source testnet-investigator \
  -- get_info_paginated \
  --params '{"limit": 50, "cursor_index": 4294967295}'
```

With a 10-asset cap, pagination will never matter in practice — but the paginated path is the one to prefer in a client that might be pointed at a future contract with a higher cap.

---

## Step 2: Check Readiness

```bash
stellar contract invoke \
  --id "$SWEEP_CONTROLLER_ID" --network testnet --source testnet-investigator \
  -- can_sweep --ephemeral_account "$EPHEMERAL_ID"
# → true

# See exactly what would move, and whether anything blocks it
stellar contract invoke \
  --id "$EPHEMERAL_ID" --network testnet --source testnet-investigator \
  -- simulate_sweep --destination "$DESTINATION"
# → (payments, error_code)   error_code == 0 means unblocked
```

`can_sweep()` requires `payment_received == true` **and** status exactly `PaymentReceived` **and** not expired. Note it is a strict equality on status, so it returns `false` for an account that has already been swept or expired even though a sweep would also fail there. It is a readiness check, not an error code.

---

## Step 3: Sign and Sweep

Identical mechanics to the single-asset case — the digest is `SHA256(destination ‖ nonce ‖ controller_id)` and knows nothing about assets or amounts. One signature authorises a sweep of **whatever is currently recorded**, including assets recorded after the signature was produced.

That is worth stating plainly: **the signed message does not commit to the payment set.** A signature obtained at nonce 7 authorises a sweep of any account state reachable at nonce 7. Do not treat a signed sweep as authorisation of a specific amount or asset list. Full detail: `testnet/examples/signed-sweep-walkthrough.md`.

```bash
NONCE=$(stellar contract invoke \
  --id "$SWEEP_CONTROLLER_ID" --network testnet --source testnet-investigator \
  -- get_nonce)

SIG=$(cargo run --quiet --manifest-path tools/sweep-signer/Cargo.toml -- sign \
  --contract-id "$SWEEP_CONTROLLER_ID" \
  --destination "$DESTINATION" \
  --nonce "$NONCE" \
  --signer-seed-hex "$SEED" \
  | grep '^auth_signature' | awk '{print $NF}')

stellar contract invoke \
  --id "$SWEEP_CONTROLLER_ID" --network testnet --source testnet-relayer \
  -- execute_sweep \
  --ephemeral_account "$EPHEMERAL_ID" \
  --destination "$DESTINATION" \
  --auth_signature "$SIG"
```

### Atomicity

`transfers::execute_transfers()` loops over the payments calling SEP-41 `TokenClient::transfer()` for each. Soroban transactions are atomic, so if the third of four transfers fails, the whole transaction reverts — no partial sweep, no partially-drained account. The observable effect of a failed multi-asset sweep is **total failure**, not partial movement.

The practical consequence: adding a low-balance asset to an account makes the *entire* sweep fail, including the well-funded ones. There is no "best effort" mode.

---

## Step 4: Read `SweepExecutedMulti.payments`

Two events fire, on **different contracts**, and they do not contain the same information.

| Event | Contract | Payload | Contains per-asset detail? |
|---|---|---|---|
| `SweepExecutedMulti` | `EphemeralAccount` | `destination`, `payments: Vec<Payment>` | **Yes** |
| `SweepCompleted` | `SweepController` | `ephemeral_account`, `destination`, `amount: i128` | **No** |

`swept_mul` is the one to read for multi-asset accounting. `sweep` carries only a summed `i128`, which across different assets is not a meaningful figure — never render it to a user as "amount swept."

```rust
struct Payment {
    asset: Address,     // which token
    amount: i128,       // in that token's base units
    timestamp: u64,     // ledger timestamp at record_payment, not sweep time
}
```

### Fetching It

```bash
stellar contract events \
  --contract-id "$EPHEMERAL_ID" \
  --start-ledger "$START_LEDGER" \
  --filter '{"topics": [["swept_mul"]]}' \
  --network testnet
```

Or via RPC, scoped to the sweep ledger:

```bash
curl -X POST https://soroban-testnet.stellar.org \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0", "id": 1, "method": "getEvents",
    "params": {
      "startLedger": '"$START_LEDGER"',
      "endLedger": '"$END_LEDGER"',
      "filters": [{
        "type": "contract",
        "contractIds": ["'"$EPHEMERAL_ID"'"],
        "topics": [["swept_mul"]]
      }]
    }
  }'
```

### Decoding the Payment List

```typescript
import { Horizon, scValToNative, StrKey, Contract } from '@stellar/stellar-sdk';

interface DecodedPayment { asset: string; assetType: string; amount: string; timestamp: number; }

function decodeSweepExecutedMulti(data: unknown, networkPassphrase: string): {
  destination: string;
  payments: DecodedPayment[];
} {
  const value = scValToNative(data) as any;
  const { destination, payments } = value;

  return {
    destination,
    payments: payments.map((p: any) => ({
      asset: p.asset,
      // asset is itself a contract address -> resolve to a human symbol
      assetType: 'contract',
      // i128 arrives as a string; never coerce through Number.
      amount: BigInt(p.amount).toString(),
      timestamp: Number(p.timestamp),
    })),
  };
}
```

Three decoding traps:

1. **`i128` fields decode to strings, not numbers.** `amount` above 2^53 loses precision through `Number()`. Keep it as `BigInt` and format at the edge. The same applies to `SweepCompleted.amount` and `AccountExpired.total_amount`.
2. **`asset` is an `Address`, not a ticker.** You get a `C...` contract ID. Resolving it to `USDC` requires a SEP-41 `name()` call or your own token registry — the event does not carry a symbol.
3. **`timestamp` is when the payment was recorded, not when the sweep ran.** The field is copied from the stored `Payment`. Do not use it as the sweep time.

### Resolving Asset Symbols

```typescript
async function assetSymbol(contractId: string, networkPassphrase: string): Promise<string> {
  const client = new Contract(contractId);
  try {
    return await client.name();   // e.g. "USDC"
  } catch {
    return contractId;            // not all SEP-41 tokens implement name()
  }
}
```

Cache these. One RPC round trip per token per sweep is wasteful, and the token list per account is stable.

### Expected Output Shape

```
SweepExecutedMulti {
  destination: GBEST...RECIPIENT,
  payments: [
    { asset: CA3D5...XLM,  amount: 2500000000, timestamp: 1789... },   // 250 XLM
    { asset: CCUSDC...,    amount:  100000000, timestamp: 1789... },   // 100 USDC (7 dp)
  ]
}
```

Order follows the contract's internal payment map iteration, not the order you called `record_payment()`. **Do not assert on ordering.** Match on `asset` instead.

---

## Step 5: Verify Balances

```bash
# Status is Swept
stellar contract invoke --id "$EPHEMERAL_ID" --network testnet \
  --source testnet-investigator -- get_status
# → 2 (Swept)

# Confirm each token actually arrived at the destination
```

Reading the `ephemeral_id` contract's `get_info()` after the sweep still shows the recorded payments — recording is a permanent ledger entry, and the sweep does not clear it. Sweep state is `status` plus `swept_to`; the payment list is history, not a live balance.

---

## Edge Cases

### Exceeding the 10-Asset Cap

```bash
# 11th distinct asset
stellar contract invoke --id "$EPHEMERAL_ID" --network testnet --source testnet-payer \
  -- record_payment --amount 1000 --asset "$ELEVENTH_ASSET"
# → Error::TooManyPayments (14)
```

Every sweep now fails until the account is retired. There is no way to remove a recorded payment.

### One Asset With Insufficient Balance

The sweep reverts in full with `TransferFailed` (controller, code 2) — the mapped error for *any* transfer failure. The contract's `InsufficientBalance` variant is declared but not what surfaces here; `execute_transfers()` maps every token error to `TransferFailed`. Diagnose by checking each asset's actual balance on the ephemeral account, not by expecting a balance-specific error code.

### Mixed Decimal Precision

Amounts are raw base units, and precision varies per token: 7 decimals for XLM, 6 for USDC, and whatever a given token declares. Two payments summed by `SweepCompleted.amount` combine incommensurable units. Format per asset, never in aggregate.

### `SweepExecutedMulti` Fires Even on the Direct Path

Both `sweep()` and `sweep_claim()` emit `swept_mul`, so you cannot distinguish a controller-driven sweep from a direct `claim()` by that event alone. Correlate with `SweepCompleted` on the controller to tell them apart.

---

## Related Documentation

- `testnet/config/multi-asset-testing.md` — test tokens and amounts
- `testnet/runbooks/multi-payment-edge-cases.md` — duplicate-asset behaviour in depth
- `testnet/registry/event-topics.md` — `payment` vs `multi_pay` vs `swept_mul` topics
- `testnet/examples/signed-sweep-walkthrough.md` — signing mechanics
- `testnet/examples/gas-free-claim-walkthrough.md` — the alternative sweep path
- `docs/api-reference.md` — `record_payment`, `get_info_paginated`, `simulate_sweep`

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-26 | Initial multi-asset sweep walkthrough |
