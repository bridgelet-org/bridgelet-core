# Pagination and History for a Testnet Contract

This document shows how to page through all Soroban events emitted by one testnet
contract instance for audit trails and dashboards, and how to correlate those
events with Horizon classic history. It is read-only integration guidance; no
network calls were made while writing it.

> **Deployment caveat.** The canonical
> [contract status registry](../registry/contract-status.md) records two TBD and
> two N/A IDs (EphemeralAccount and SweepController: `TBD`;
> ReserveContract and AccountFactory: `N/A`), all Last Verified `Never`. That
> table is stale relative to the current `scripts/deploy-testnet.sh`, which
> builds and deploys all four. The placeholders below are intentional. Verify
> an ID on-chain with
> [verify-contract-live.md](../registry/verify-contract-live.md) before paging
> it, and record that verification in your run.

"Over the lifetime of an instance" means **from the first ledger you have
retained for that verified instance to the current head**. RPC and Horizon
serve rolling windows; neither is an archive. If you need lifetime history,
start persisting events from first verification, keep your own index, and
replay from it when a provider's window has aged out.

## Terminology

| Concept | Soroban RPC v23 field |
|---|---|
| Ledger range | `startLedger`, optional `endLedger` (**exclusive**) |
| Page size | `pagination.limit` |
| Resume | request `pagination.cursor`; response top-level `cursor` and per-event `cursor` |
| Record identity | event `id`; use it for deduplication |

The response cursor is returned at the top level of `result`, and every event
also carries its own `cursor` and `id`. Use the top-level value to request the
next page. Keep each event's `cursor` if you need to resume from an exact event
boundary. Pages can overlap, so always deduplicate by event `id`.

## Two histories, not one

| Source | Contains | Use it for | Do not use it for |
|---|---|---|---|
| Soroban RPC `getEvents` | Contract events in ledger order, with topics and event value XDR | Bridgelet audit trails, payment/sweep/expiry events, interface drift | Proving a token moved; classic payment history |
| Horizon `/payments`, `/operations`, `/transactions` | Classic Stellar ledger history | Verifying actual transfers and transaction outcomes | Listing Soroban contract events |

A Bridgelet `payment` event records application metadata. It is not by itself a
SEP-41 transfer. Conversely, a Horizon payment does not prove that
`record_payment` was called. Correlate both; see
[expired-account-recovery-check.md](../runbooks/expired-account-recovery-check.md).

## Before you page

1. Set network values. The passphrase is exactly
   `Test SDF Network ; September 2015`.

   ```bash
   export NETWORK=testnet
   export RPC_URL=https://soroban-testnet.stellar.org
   export HORIZON_URL=https://horizon-testnet.stellar.org
   export PASSPHRASE="Test SDF Network ; September 2015"
   export EA_ID=<VERIFIED_EPHEMERAL_ACCOUNT_ID>
   export START_LEDGER=<FIRST_LEDGER_YOU_RETAIN>
   ```

2. Verify the ID and write down the WASM hash and interface you observed.
3. Pick a start ledger. Use `getLatestLedger` and a finalized head, not a
   wall-clock timestamp. Ledger sequence is the ordering key.
4. Decide whether you are scanning one event name or all events from the
   instance. Omit `topics` for all events from that contract ID.
5. Store a reset epoch with the scan. If the head ledger decreases or the ID
   stops resolving, treat it as a new epoch; see
   [reset-policy.md](../policies/reset-policy.md).

## Topic filters from the event registry

`topics[0]` is the event name. The table below comes from
[event-topics.md](../registry/event-topics.md) and the checked-in source. A
deployed ABI may differ; confirm with `stellar contract info interface` before
relying on a symbol.

RPC `topics` are arrays of **base64-encoded ScVal strings**. Do not send plain
symbol strings or ScVal JSON objects as topic values.

| Contract | Decoded symbol | Base64 ScVal topic value | Notes |
|---|---|---|---|
| EphemeralAccount | `created` | `AAAADwAAAAdjcmVhdGVkAA==` | `initialize()` |
| EphemeralAccount | `payment` | `AAAADwAAAAdwYXltZW50AA==` | First `record_payment()` |
| EphemeralAccount | `multi_pay` | `AAAADwAAAAltdWx0aV9wYXkAAAA=` | Later `record_payment()` calls |
| EphemeralAccount | `swept_mul` | `AAAADwAAAAlzd2VwdF9tdWwAAAA=` | Multi-payment sweep |
| EphemeralAccount | `expired` | `AAAADwAAAAdleHBpcmVkAA==` | `expire()` / `recover()` |
| EphemeralAccount | `reserve` | `AAAADwAAAAdyZXNlcnZlAA==` | Reserve reclaim |
| SweepController | `sweep` | `AAAADwAAAAVzd2VlcAAAAA==` | `execute_sweep()` / `claim()` |
| SweepController | `dest_auth` | `AAAADwAAAAlkZXN0X2F1dGgAAAA=` | Locked destination at initialize |
| SweepController | `dest_upd` | `AAAADwAAAAhkZXN0X3VwZA==` | Locked destination change |
| ReserveContract | `initialized` | `AAAADwAAAAtpbml0aWFsaXplZAA=` | Registry name; source emits shorter `init` |
| ReserveContract | `init` | `AAAADwAAAARpbml0` | Source-derived shorter name |
| ReserveContract | `base_reserve_updated` | `AAAADwAAABRiYXNlX3Jlc2VydmVfdXBkYXRlZA==` | Registry name |
| AccountFactory | none | none | No factory-level events are documented; filter child EphemeralAccount IDs by their own `created` events once known |

The ReserveContract naming conflict must be resolved against the deployed ABI
before building a filter.

Generate additional values locally with `stellar xdr encode --type ScVal
--output single-base64`. Only the encoded output string belongs in an RPC topic
filter.

## `getEvents`: first page

One filter for all events from one verified instance:

```bash
curl -sS "$RPC_URL" \
  -H 'content-type: application/json' \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getEvents",
    "params": {
      "startLedger": 1000000,
      "endLedger": 1001000,
      "pagination": { "limit": 200 },
      "filters": [
        {
          "type": "contract",
          "contractIds": ["<CONTRACT_ID>"]
        }
      ]
    }
  }'
```

`endLedger` is **exclusive**: this request covers ledgers 1000000 through
1000999.

One filter for a specific event name:

```bash
curl -sS "$RPC_URL" \
  -H 'content-type: application/json' \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getEvents",
    "params": {
      "startLedger": 1000000,
      "endLedger": 1001000,
      "pagination": { "limit": 200 },
      "filters": [
        {
          "type": "contract",
          "contractIds": ["<CONTRACT_ID>"],
          "topics": [["AAAADwAAAAdjcmVhdGVkAA=="]]
        }
      ]
    }
  }'
```

Rules to keep the result set well-defined:

- `contractIds` accepts up to 5 IDs; `topics` accepts up to 5 topic filters;
  each topic filter has 1–4 base64 ScVal segment strings.
- Multiple filters are OR'ed; `contractIds` and `topics` inside one filter are
  AND'ed.
- `startLedger` is inclusive and `endLedger` is exclusive. Do not combine a
  ledger range with `pagination.cursor` in the same request.
- Keep the same filters on every page. Changing a filter mid-scan invalidates
  the cursor sequence.

## `getEvents`: next page

Use the top-level `cursor` from the previous response as `pagination.cursor`:

```bash
curl -sS "$RPC_URL" \
  -H 'content-type: application/json' \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "getEvents",
    "params": {
      "pagination": {
        "limit": 200,
        "cursor": "<CURSOR_FROM_PREVIOUS_RESPONSE>"
      },
      "filters": [
        {
          "type": "contract",
          "contractIds": ["<CONTRACT_ID>"],
          "topics": [["AAAADwAAAAdjcmVhdGVkAA=="]]
        }
      ]
    }
  }'
```

A minimal response to record looks like this; adapt to your client's response
wrapper:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "latestLedger": 1000500,
    "cursor": "0000000000001000-0000000000",
    "events": [
      {
        "id": "0000000000000999-0000000002",
        "cursor": "0000000000000999-0000000002",
        "type": "contract",
        "ledger": 1000001,
        "contractId": "<CONTRACT_ID>",
        "value": { "xdr": "AAAA..." }
      }
    ]
  }
}
```

Handling rules:

- Persist `result.cursor` as the resume point for the following request.
- Persist each event's `id` and `cursor` with the event record.
- Deduplicate by event `id` on every write.
- Stop when the page is empty or the cursor stops advancing. Do not depend on
  a record count to decide that a scan is complete.

## CLI equivalent

In `stellar` 23.4.1 the command is top-level `stellar events`; older docs in
this tree refer to `stellar contract events`, which is not a valid subcommand
in the installed CLI.

```bash
# First page, all events from one instance.
stellar events \
  --id "$EA_ID" \
  --start-ledger "$START_LEDGER" \
  --count 100 \
  --output json \
  --network "$NETWORK" \
  --rpc-url "$RPC_URL"

# Next page using the RPC cursor from the previous response.
stellar events \
  --id "$EA_ID" \
  --cursor "$RPC_CURSOR" \
  --count 100 \
  --output json \
  --network "$NETWORK" \
  --rpc-url "$RPC_URL"

# Filter by one topic. The value is a base64 ScVal string.
stellar events \
  --id "$EA_ID" \
  --topic "AAAADwAAAAdjcmVhdGVkAA==" \
  --start-ledger "$START_LEDGER" \
  --count 100 \
  --output json \
  --network "$NETWORK" \
  --rpc-url "$RPC_URL"
```

CLI details verified from local help:

- `--count` is the maximum events returned and defaults to 10.
- `--id` and `--topic` accept at most 5 values each.
- `--topic` accepts comma-separated base64 ScVal segments; `*` matches one
  segment and `**` matches a flexible final segment.
- `--cursor` and `--start-ledger` are alternative resume strategies; do not
  mix them in one scan.

## Ledgers, cursors, and retention

| Concern | Rule |
|---|---|
| Ordering | Ledger sequence, not timestamp. Persist `ledger`, event `id`, and event `cursor`. |
| Resume | Use the top-level response `cursor` for the next request; keep event cursors for exact boundaries. Expect overlap and dedupe by `id`. |
| Cursor scope | Opaque and provider-specific. Do not reuse it across a different filter, contract, reset epoch, or network. |
| Retention | RPC keeps a recent window whose size is provider policy; Horizon prunes classic history. No archive guarantee is made here. |
| Missing old history | If a start ledger is older than the provider window, record `history_unavailable` and fail loudly. Do not report "no events". |
| Reset | A lower head ledger or an ID that no longer resolves starts a new epoch; see [reset-policy.md](../policies/reset-policy.md). |

Dedupe key, namespaced to a deployment epoch:

```text
(network, reset_epoch, event.id)
```

An event present in the window is an audit record, not proof of a token
balance change; see [event-watchlist.md](../monitoring/event-watchlist.md).

## Horizon classic history

Use Horizon for the asset-moving leg and for transaction outcomes. Horizon's
cursor is its own value and is unrelated to the RPC event cursor.

```bash
# First page of classic payments involving an address.
curl -sS "$HORIZON_URL/accounts/$ADDRESS/payments?order=asc&limit=200"

# Next page using the Horizon cursor from the previous response.
curl -sS "$HORIZON_URL/accounts/$ADDRESS/payments?order=asc&limit=200&cursor=$HORIZON_CURSOR"

# Confirm one transaction outcome by hash.
curl -sS "$HORIZON_URL/transactions/$TX_HASH"
```

Follow Horizon's `_links.next` where available. Store the last Horizon cursor
with the same reset epoch as your RPC cursor. Horizon `/payments`,
`/operations`, and `/transactions` do **not** list Soroban contract events.

## Archive and reconciliation

For an audit trail that survives provider retention and testnet resets:

1. Persist normalized events as they are observed, including unrecognized
   topics. Do not silently drop unknown event names.
2. Keep a compact record per event:

   ```json
   {
     "network": "testnet",
     "reset_epoch": "<EPOCH_ID>",
     "contract_id": "<CONTRACT_ID>",
     "topic_0": "AAAADwAAAAdjcmVhdGVkAA==",
     "ledger": 1000001,
     "transaction_hash": "<TX_HASH>",
     "event_id": "<EVENT_ID>",
     "event_cursor": "<EVENT_CURSOR>",
     "value_xdr": "AAAA..."
   }
   ```

3. Reconcile contract state with read-only calls such as `get_info` and
   `get_payment_count` (`--send no`); see
   [soroban-cli-quickref.md](../docs/soroban-cli-quickref.md).
4. Reconcile asset movement with Horizon `/payments` and per-asset balances. A
   matching Bridgelet event and a matching Horizon payment are both needed.
5. Re-verify the contract ID, WASM hash, and interface after any reset before
   resuming a scan.
6. Never store secret keys, signing seeds, or reusable signatures in the
   archive; testnet data is public but still must not contain credentials (see
   [data-retention.md](../policies/data-retention.md)).

## Failure modes

| Symptom | Likely meaning | Action |
|---|---|---|
| Empty result for a new ID | No events in the requested window; possibly uninitialized or undeployed | Verify with `get_info` and the registry; do not conclude activity never happened |
| Start ledger older than retention | Window aged out | Record `history_unavailable`; replay from your own archive |
| ID no longer resolves | Reset or redeploy | Start a new epoch; re-read the registry |
| Unknown `topics[0]` | Interface or event-name drift | Preserve the event and check the deployed interface |
| Ledger decreases | Reset | New epoch; do not merge cursors across epochs |
| Cursor stops advancing | End of served window or provider behavior | Stop the scan and record the last committed event id |
| Event exists but balance is unchanged | Metadata event, failed inner call, or classic transfer missing | Check Horizon and state getters; see [event-watchlist.md](../monitoring/event-watchlist.md) |

## Related documents

- [event-topics.md](../registry/event-topics.md): canonical event names and
  payload notes
- [event-watchlist.md](../monitoring/event-watchlist.md): what an absent event
  does and does not mean
- [event-tail-spec.md](../tooling/event-tail-spec.md): proposed tailing tool;
  no implementation is claimed
- [rate-of-change-expectations.md](rate-of-change-expectations.md): why history
  and interfaces can shift
- [troubleshooting-index.md](../docs/troubleshooting-index.md): runbook routing

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-11-18 | Initial getEvents/Horizon pagination, topic-filter, retention, and reconciliation guidance |
| 1.1 | 2026-11-18 | Corrected to Soroban RPC v23 cursors, base64 ScVal topics, exclusive endLedger, and event-id deduplication |
