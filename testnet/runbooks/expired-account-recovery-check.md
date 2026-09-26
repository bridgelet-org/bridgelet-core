# Runbook: Verify Expired-Account Recovery

## Purpose

Verify an expired `EphemeralAccount` transition by reconciling:

1. pre- and post-expiry contract state;
2. the `AccountExpired` event payload;
3. reserve bookkeeping; and
4. actual SEP-41 balance changes for every recorded asset.

An event is an audit record. It is not automatically proof that funds reached
the recovery address.

> This runbook does not assert that an account expired or that a transfer
> occurred. The state-changing example below is an operator action.

## Important current-source limitation

In the checked-in implementation, `record_payment` stores payment metadata but
does not verify an inbound asset transfer. `expire()` and `recover()` set the
terminal state, calculate the sum of recorded amounts, update internal reserve
bookkeeping, and emit `AccountExpired`; they do not call the SEP-41 transfer
helper for those recorded payments.

As a result, a current-source `AccountExpired` event can be emitted while the
recorded asset balance remains at the ephemeral account. The `ReserveReclaimed`
event and `reserve_amount` field are internal reserve bookkeeping and are not
proof of a SEP-41 payment transfer.

A positive destination balance delta must be correlated with the expiry
transaction or another identified transaction before it is attributed to
recovery.

## Event field compatibility

The issue and some existing documents call the event's amount field
`total_amount`. The current source event type is:

```text
AccountExpired {
    recovery_address: Address,
    amount_returned: i128,
    reserve_amount: i128,
}
```

The local calculation variable is named `total_amount`, but the serialized
field is `amount_returned`. Inspect the deployed contract interface before
decoding a live event. Support the field present in the deployed ABI; do not
silently rename an unknown field.

## Required inputs

| Placeholder | Meaning |
|---|---|
| `<EA_CONTRACT_ID>` | initialized ephemeral account under investigation |
| `<RECOVERY_ADDRESS>` | recovery address stored at initialization |
| `<READER_IDENTITY>` | funded identity for read-only queries |
| `<CALLER_IDENTITY>` | creator or recovery identity if invoking `recover` |
| `<ASSET_CONTRACT_ID>` | one SEP-41 asset per recorded payment |
| `<EXPIRY_LEDGER>` | account's configured expiry ledger |
| `<TX_LEDGER_BEFORE>` | latest ledger before the terminal transaction |
| `<EXPIRY_TX_HASH>` | final expiry/recovery transaction hash |
| `<TX_LEDGER_AFTER>` | ledger containing that transaction |

Never record secret keys, seed phrases, or auth signatures.

## 1. Confirm the account and deployed version

Read the account before taking a terminal action:

```bash
stellar contract invoke \
  --simulate-only \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  get_info

stellar contract invoke \
  --simulate-only \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  get_status

stellar contract invoke \
  --simulate-only \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  is_expired

stellar network current-ledger \
  --rpc-url https://soroban-testnet.stellar.org
```

Before the terminal transition, normally expect:

- `status` is `0` (`Active`) or `1` (`PaymentReceived`);
- `is_expired` is `true` when `current_ledger >= expiry_ledger`;
- `get_info.recovery_address` equals `<RECOVERY_ADDRESS>`; and
- `swept_to` is absent.

If the deployed interface or code differs from current source, use
[`upgrade-verification.md`](upgrade-verification.md) and
[`../registry/upgrade-history.md`](../registry/upgrade-history.md) before
interpreting behavior.

## 2. Capture pre-expiry state and balances

Record the output of:

```bash
stellar contract invoke \
  --simulate-only \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  get_reserve_remaining

stellar contract invoke \
  --simulate-only \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  get_reserve_available

stellar contract invoke \
  --simulate-only \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  is_reserve_reclaimed

stellar contract invoke \
  --simulate-only \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  get_reserve_reclaim_event_count
```

For every asset in `get_info.payments`, query the SEP-41 balance at both the
ephemeral account and recovery address:

```bash
stellar contract invoke \
  --simulate-only \
  --id <ASSET_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  balance \
  --id <EA_CONTRACT_ID>

stellar contract invoke \
  --simulate-only \
  --id <ASSET_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  balance \
  --id <RECOVERY_ADDRESS>
```

Use integer base units. Do not use a classic Horizon XLM balance as a
substitute for a SEP-41 token balance.

Record one row per asset:

| Asset | Recorded amount | EA balance before | Recovery balance before |
|---|---:|---:|---:|
| `<ASSET_CONTRACT_ID>` | `<AMOUNT>` | `<EA_BEFORE>` | `<RECOVERY_BEFORE>` |

## 3. Trigger exactly one terminal transition

After `is_expired` is true, the permissionless path is available:

```bash
stellar contract invoke \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  expire
```

Alternatively, the creator or recovery address may call `recover`:

```bash
stellar contract invoke \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <CALLER_IDENTITY> \
  -- \
  recover \
  --caller <RECOVERY_ADDRESS>
```

The source identity must authorize the `caller` argument. Record the returned
transaction hash and finalized ledger. Do not retry blindly after a terminal
state is reached; a second call should fail with `InvalidStatus`.

## 4. Verify terminal state

After the transaction finalizes:

```bash
stellar contract invoke \
  --simulate-only \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  get_info

stellar contract invoke \
  --simulate-only \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  get_status

stellar contract invoke \
  --simulate-only \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  get_reserve_remaining

stellar contract invoke \
  --simulate-only \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  get_reserve_available

stellar contract invoke \
  --simulate-only \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  is_reserve_reclaimed
```

Expected state from current source:

- `status == 3` (`Expired`);
- `swept_to` equals `<RECOVERY_ADDRESS>`;
- the stored `recovery_address` is unchanged;
- payment records remain visible; and
- reserve counters reflect the reclaim operation.

State correctness is not balance correctness.

## 5. Locate the matching event

Query a narrow range around the terminal transaction:

```bash
stellar contract events \
  --contract-id <EA_CONTRACT_ID> \
  --start-ledger <TX_LEDGER_BEFORE> \
  --filter '{"topics":[["expired"]]}' \
  --network testnet

stellar contract events \
  --contract-id <EA_CONTRACT_ID> \
  --start-ledger <TX_LEDGER_BEFORE> \
  --filter '{"topics":[["reserve"]]}' \
  --network testnet
```

Match events by contract ID, ledger, transaction hash, and event index. Do not
accept an older event merely because its payload looks similar.

The current implementation calls `reclaim_reserve_to` before emitting
`AccountExpired`, so the matching `ReserveReclaimed` event should be in the
same transaction or ledger range.

## 6. Reconcile event fields

| Check | Expected relationship |
|---|---|
| `recovery_address` | Equals stored recovery address and post-transition `swept_to` |
| `amount_returned` or deployed `total_amount` | Equals the integer sum of recorded payment amounts |
| `reserve_amount` | Equals the matching `ReserveReclaimed.amount` |
| reserve counter delta | `before_remaining - after_remaining == reserve_amount` |
| reserve available delta | `after_available == before_available - reserve_amount` |
| reserve destination | `ReserveReclaimed.destination` equals the recovery address |
| `fully_reclaimed` | Agrees with `after_remaining == 0` |

The current source calculates:

```text
expected_reserve_amount = min(before_remaining, before_available)
```

Do not hard-code the initial reserve constant. A prior or partial reclaim can
change the result, and zero is valid when no reserve is available.

### Multi-asset warning

The event's aggregate amount is not denomination-aware. It can sum raw amounts
from different token base units. Reconcile every asset separately; never
compare that aggregate with one token balance.

## 7. Reconcile actual balances

For each asset, calculate:

```text
recovery_delta = recovery_balance_after - recovery_balance_before
account_debit  = account_balance_before - account_balance_after
```

If a deployed implementation actually transfers recorded payments, the desired
invariant is:

```text
recovery_delta == recorded_payment_amount
account_debit  == recorded_payment_amount
```

The invariant must hold independently for every asset.

For the current source's `expire`/`recover` path, no SEP-41 transfer call is
present. Therefore:

- a zero payment delta is consistent with the current code, but it does not
  prove recovery;
- a positive delta must be tied to a different deployed version or another
  identified transaction; and
- `reserve_amount` must not be added to a recorded payment amount as if both
  represented the same transferred balance.

Native XLM fees, rent, and contract-ledger funding can affect an account
balance independently. Reconcile per asset and preserve unrelated evidence.

## 8. Classify the result

| Classification | Meaning |
|---|---|
| `RECOVERY_BALANCE_VERIFIED` | Deployed code and transaction evidence show every expected per-asset delta |
| `METADATA_AND_STATE_VERIFIED` | Event and contract state reconcile, but current source did not move payment balances |
| `EVENT_MISMATCH` | Event address/amount differs from state, reserve event, or deployed ABI |
| `BALANCE_MISMATCH` | Observed deltas do not match the intended transfer and no concurrent activity explains them |
| `UNVERIFIED` | Transaction, event, balance, or deployed-version evidence is incomplete |

Only `RECOVERY_BALANCE_VERIFIED` should be reported as **funds landed at the
recovery address**.

## Evidence record

```text
EA contract:             <EA_CONTRACT_ID>
Recovery address:        <RECOVERY_ADDRESS>
Expiry transaction:      <EXPIRY_TX_HASH>
Transaction ledger:      <TX_LEDGER_AFTER>
Deployed WASM/hash:      <VERIFIED_HASH_OR_UNKNOWN>
Event amount field:      <amount_returned_OR_total_amount>
Event recovery address:  <EVENT_RECOVERY_ADDRESS>
Event aggregate amount:  <EVENT_AMOUNT>
Event reserve amount:    <EVENT_RESERVE_AMOUNT>
Reserve before/after:    <BEFORE_REMAINING> / <AFTER_REMAINING>
Per-asset deltas:        <ATTACHED_TABLE>
Classification:          <CLASSIFICATION>
Operator:                <OPERATOR_PUBLIC_IDENTITY>
Review date:             <UTC_DATE>
```

Attach raw event output, `get_info`, reserve queries, and per-asset balance
snapshots. Use `UNKNOWN`, not estimates, for missing evidence.

## Escalation

Escalate when:

- the event recovery address differs from initialization evidence;
- the aggregate event amount does not match recorded metadata;
- `reserve_amount` does not match the reserve event or counter delta;
- a deployed implementation claims to transfer funds but the per-asset delta
  is absent;
- a positive delta cannot be tied to a transaction;
- the transaction failed or the event belongs to another account; or
- the current source's missing payment-transfer behavior is confirmed live.

Attach the contract ID, transaction hash, raw event, state snapshots,
per-asset balances, and verified deployed code identity. Use
[`upgrade-verification.md`](upgrade-verification.md) if the deployed WASM may
differ from the checkout.

## Related documents

- [`expiry-ledger-testing.md`](../config/expiry-ledger-testing.md)
- [`stuck-ephemeral-account.md`](stuck-ephemeral-account.md)
- [`upgrade-verification.md`](upgrade-verification.md)
- [`../registry/event-topics.md`](../registry/event-topics.md)
- [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md)
- [`../monitoring/README.md`](../monitoring/README.md)
