# Monitoring a Single Account's Lifecycle

A template for building a status tracker (dashboard row, webhook, CLI tail) for
**one** `EphemeralAccount` from creation through payment to sweep or expiry.

> **Status of this document.** The event names, payloads and ordering rules
> below are taken from the contract source (`contracts/ephemeral_account/src/events.rs`,
> `contracts/sweep_controller/src/lib.rs`) and `testnet/registry/event-topics.md`.
> They have **not** been captured from a live testnet run: per
> `testnet/registry/contract-status.md` the contracts are not currently verified
> as deployed. Re-check ordering against real events once they are.

## The state machine you are tracking

`AccountStatus` (`contracts/shared/src/types.rs`): `Active (0)` →
`PaymentReceived (1)` → `Swept (2)`, or `Active/PaymentReceived` → `Expired (3)`.

| Step | Triggering call | Contract | Event topic[0] | Payload |
|---|---|---|---|---|
| 1. Created | `initialize` | EphemeralAccount | `created` | `AccountCreated { creator, expiry_ledger }` |
| 2. Paid | `record_payment` (first) | EphemeralAccount | `payment` | `PaymentReceived { amount, asset }` |
| 2b. More assets | `record_payment` (later) | EphemeralAccount | `multi_pay` | `MultiPaymentReceived { asset, amount }` |
| 3a. Swept | `sweep` / `sweep_claim` | EphemeralAccount | `swept_mul` | `SweepExecutedMulti { destination, payments }` |
| 3a. Swept (controller side) | `execute_sweep` / `claim` | SweepController | `sweep` | `SweepCompleted { ephemeral_account, destination, amount }` |
| 3b. Expired | `expire` / `recover` | EphemeralAccount | `expired` | `AccountExpired { recovery_address, amount_returned, reserve_amount }` |
| Reserve | after a sweep/expiry that reclaims reserve | EphemeralAccount | `reserve` | `ReserveReclaimed { .. }` |

Terminal states are `Swept` and `Expired`. Once either is seen, stop tracking
for that account.

## Ordering rules

1. **Order by ledger sequence, then by position within the ledger/transaction,
   not by arrival time.** RPC/Horizon pages can be delivered out of order
   relative to your own polling.
2. `created` is always first for a given contract ID. If you see any other
   event first, you subscribed too late (use `--start-ledger` at or before the
   creation ledger).
3. A `sweep` (controller) event is emitted only **after** the account-side sweep
   succeeded, so for one transaction expect `swept_mul` and `sweep` together.
   Do not assume which of `swept_mul` / `reserve` / `sweep` comes first inside
   the transaction; key your state machine on **which events have appeared**,
   not on their relative position.
4. `payment` appears once; every further asset produces `multi_pay`.
5. `expired` and `swept_mul` are mutually exclusive for one account; if both
   appear, treat it as an anomaly and open an incident
   (`testnet/runbooks/stuck-ephemeral-account.md`).
6. De-duplicate on `(contract_id, ledger, transaction hash, event index)`;
   event streams can replay after a restart.

## Minimal tracker (pseudocode)

```text
state = UNKNOWN
for event in events(contract = ACCOUNT_ID, from_ledger = creation_ledger):
    switch event.topic0:
        "created":             state = ACTIVE;            expiry = event.expiry_ledger
        "payment","multi_pay": state = PAYMENT_RECEIVED   (record amount/asset)
        "swept_mul":           state = SWEPT;             destination = event.destination
        "expired":             state = EXPIRED
        "reserve":             note reserve movement (does not change state)
    if state in {SWEPT, EXPIRED}: stop

# Also poll once per N ledgers, independent of events:
if latest_ledger > expiry and state != SWEPT:   flag "should be expired; call expire()"
```

## Cross-check against on-chain state

Events can lag (see `testnet/runbooks/rpc-outage.md`). For any row that looks
stuck, confirm with read-only calls (see `testnet/examples/checking-account-status.md`):

```bash
stellar contract invoke --id "$EPHEMERAL_ACCOUNT_ID" --source "$READER" \
  --network testnet --send=no -- get_status
stellar contract invoke --id "$EPHEMERAL_ACCOUNT_ID" --source "$READER" \
  --network testnet --send=no -- is_expired
```

Treat `get_status` as authoritative and the event stream as the notification
path. If they disagree, trust the contract state and investigate the stream.

## Dashboard row suggestion

| Column | Source |
|---|---|
| Account ID | contract ID |
| Status | latest of the state machine above |
| Created at / expiry ledger | `created` |
| Paid assets & amounts | `payment` + `multi_pay` |
| Terminal outcome + destination | `swept_mul` / `expired` |
| Last event ledger | max ledger seen |
| Drift flag | `get_status` ≠ event-derived status |
