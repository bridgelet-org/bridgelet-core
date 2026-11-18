# Sample Event Payloads for Webhook Simulation (Follow-Up)

Concrete JSON payloads for each contract event, for building a webhook
relay/consumer test.

> **Important: these are NOT captured from live testnet.** The issue asks for
> payloads generated from real observed events. The Bridgelet contracts are not
> currently recorded as deployed (`testnet/registry/contract-status.md`) and no
> events have been observed or recorded in this repository, so none could be
> captured. The samples below are **synthesized from the Rust event structs** in
> `contracts/ephemeral_account/src/events.rs` and
> `contracts/sweep_controller/src/lib.rs`, with placeholder values. **Replace them
> with real captures** (instructions below) and delete this notice when you do.
>
> Also: `testnet/integration/webhook-simulation.md`, which this document is meant
> to follow up, does not exist in the repository.

## Representation used

Decoded the way `scValToNative` (stellar-sdk) presents values:

- `Address` → string (`G...` or `C...`);
- `i128` / `u64` → **string** in JSON (they exceed JS safe-integer range; encode as strings in your envelope);
- `u32` → number;
- `Option<T>` → value or `null`;
- `Vec<T>` → array.

Envelope (your relay's choice; shown for context):

```json
{
  "contractId": "<C...>",
  "ledger": 0,
  "txHash": "<hex>",
  "topic": ["<event symbol>"],
  "data": { }
}
```

### A note on field order

A Soroban `contracttype` struct is encoded on-chain as a map, and the encoding
orders entries by **key name**, not by the field order declared in Rust. Decoded
objects can therefore arrive with keys alphabetically ordered. **Parse by key,
never by position.** (Expected from Soroban's encoding; verify against a real
event when you capture one.)

## EphemeralAccount events

### `created`
```json
{ "topic": ["created"],
  "data": { "creator": "G...CREATOR", "expiry_ledger": 1234567 } }
```

### `payment` (first payment)
```json
{ "topic": ["payment"],
  "data": { "amount": "100000000", "asset": "C...XLM_SAC" } }
```

### `multi_pay` (each additional asset)
```json
{ "topic": ["multi_pay"],
  "data": { "asset": "C...OTHER_TOKEN", "amount": "5000000" } }
```

### `swept_mul`
```json
{ "topic": ["swept_mul"],
  "data": {
    "destination": "G...DESTINATION",
    "payments": [
      { "asset": "C...XLM_SAC", "amount": "100000000", "timestamp": "1760000000" },
      { "asset": "C...OTHER_TOKEN", "amount": "5000000", "timestamp": "1760000100" }
    ] } }
```

### `expired`
```json
{ "topic": ["expired"],
  "data": { "recovery_address": "G...RECOVERY", "amount_returned": "100000000", "reserve_amount": "1000000000" } }
```
(The registry document `testnet/registry/event-topics.md` names the second field
`total_amount`; the source struct calls it `amount_returned`. Use the source name
until a real capture settles it.)

### `reserve`
```json
{ "topic": ["reserve"],
  "data": { "destination": "G...DESTINATION", "amount": "1000000000", "sweep_id": "1234570",
            "fully_reclaimed": true, "remaining_reserve": "0" } }
```

## SweepController events

### `sweep`
```json
{ "topic": ["sweep"],
  "data": { "ephemeral_account": "C...ACCOUNT", "destination": "G...DESTINATION", "amount": "105000000" } }
```
Note: `amount` is the **sum across all payments**, mixing assets; do not treat it
as a single-asset value (`testnet/examples/multi-asset-sweep.md`).

### `dest_auth`
```json
{ "topic": ["dest_auth"], "data": { "destination": "G...LOCKED" } }
```

### `dest_upd`
```json
{ "topic": ["dest_upd"],
  "data": { "old_destination": "G...OLD", "new_destination": "G...NEW" } }
```
`old_destination` is `null` when the controller was previously in flexible mode.

## How to replace these with real captures

1. Run the walkthroughs in `testnet/examples/` on a verified deployment.
2. Capture events:
   ```bash
   stellar contract events --id "$CONTRACT_ID" --network testnet \
     --start-ledger <ledger> --output json > capture.json
   ```
3. Remove addresses you don't want published (public addresses are fine; never
   include secrets), keep real ledger numbers/hashes if desired.
4. Record per sample: tool version, date, ledger, and whether decoded by the CLI
   or SDK; note any differences from the synthesized shape above (key order,
   integer encoding, `Option` representation).
5. Update this file and `testnet/registry/event-topics.md` if the real capture
   disagrees with either.
