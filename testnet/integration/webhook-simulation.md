# Webhook / Event Simulation for Local Consumers

## Overview

Event-consuming apps do not need a live deployment to test their handlers. This document shows how to build local JSON fixtures from the documented event shapes, deliver them to a webhook, and test ordering and idempotency without waiting for on-chain confirmations.

> **Source of truth.** `testnet/registry/event-topics.md` defines every topic symbol and payload struct used here. `testnet/registry/contract-status.md` records all four contracts as undeployed with IDs `TBD`, so these fixtures are derived from documentation, **not observed on-chain events**. No IDs, hashes, or transaction hashes below are real.

---

## Fixture conventions

A real Soroban RPC `getEvents` record carries base64-encoded XDR in `topic` and `value`. For local handler tests, use a decoded shape that is easier to read:

| Field | Meaning |
|---|---|
| `resetEpoch` | Local epoch label; change it when simulating a testnet reset |
| `ledger` | Ledger sequence; the total order for events |
| `txHash` | Transaction hash placeholder |
| `eventIndex` | Index within the transaction |
| `contract` | `EphemeralAccount` or `SweepController` |
| `contractId` | Placeholder for the emitting contract address |
| `topic` | Array of `ScVal` topics; `topic[0]` is the event-name symbol |
| `value` | Decoded `contracttype` payload |

`i128` amounts are written as decimal strings so the JSON does not imply a lossy JS `number`. Convert them to `bigint` in your handler. `u32` / `u64` values that fit in a safe integer are shown as numbers.

The deduplication key used below is `(resetEpoch, ledger, txHash, contractId, eventIndex)`, matching the replay rule in `testnet/tooling/event-tail-spec.md`.

---

## JSON fixtures

Save as `fixtures.json`. Every address is a placeholder; replace nothing with a guessed ID.

```json
[
  {
    "resetEpoch": "local-1",
    "ledger": 1000,
    "txHash": "<TX_HASH_1>",
    "eventIndex": 0,
    "contract": "EphemeralAccount",
    "contractId": "<EPHEMERAL_ACCOUNT_ID>",
    "topic": ["created"],
    "value": {
      "creator": "<CREATOR_ADDRESS>",
      "expiry_ledger": 1100
    }
  },
  {
    "resetEpoch": "local-1",
    "ledger": 1001,
    "txHash": "<TX_HASH_2>",
    "eventIndex": 0,
    "contract": "EphemeralAccount",
    "contractId": "<EPHEMERAL_ACCOUNT_ID>",
    "topic": ["payment"],
    "value": {
      "amount": "1000000",
      "asset": "<TOKEN_CONTRACT_ID>"
    }
  },
  {
    "resetEpoch": "local-1",
    "ledger": 1002,
    "txHash": "<TX_HASH_3>",
    "eventIndex": 0,
    "contract": "EphemeralAccount",
    "contractId": "<EPHEMERAL_ACCOUNT_ID>",
    "topic": ["swept_mul"],
    "value": {
      "destination": "<DESTINATION_ADDRESS>",
      "payments": [
        {
          "asset": "<TOKEN_CONTRACT_ID>",
          "amount": "1000000",
          "timestamp": 1001
        }
      ]
    }
  },
  {
    "resetEpoch": "local-1",
    "ledger": 1002,
    "txHash": "<TX_HASH_3>",
    "eventIndex": 1,
    "contract": "SweepController",
    "contractId": "<SWEEP_CONTROLLER_ID>",
    "topic": ["sweep"],
    "value": {
      "ephemeral_account": "<EPHEMERAL_ACCOUNT_ID>",
      "destination": "<DESTINATION_ADDRESS>",
      "amount": "1000000"
    }
  },
  {
    "resetEpoch": "local-1",
    "ledger": 1003,
    "txHash": "<TX_HASH_4>",
    "eventIndex": 0,
    "contract": "SweepController",
    "contractId": "<SWEEP_CONTROLLER_ID>",
    "topic": ["dest_upd"],
    "value": {
      "old_destination": null,
      "new_destination": "<DESTINATION_ADDRESS>"
    }
  }
]
```

`old_destination: null` is the `None` case of `Option<Address>` — flexible mode before the first update. Do not replace it with an empty string. Additional documented symbols you may add are `multi_pay`, `expired`, `reserve`, and `dest_auth`; no fixtures exist for `AccountFactory`, which currently emits no events directly.

---

## Mock webhook delivery

This uses only Node.js built-ins and a local port. It starts an endpoint, POSTs each fixture to it, prints what the consumer processed, and skips duplicate delivery keys. No credentials, no network calls outside localhost.

```javascript
const http = require('http');
const fixtures = require('./fixtures.json');

const processed = new Set();
const received = [];

function keyOf(event) {
  return [
    event.resetEpoch,
    event.ledger,
    event.txHash,
    event.contractId,
    event.eventIndex,
  ].join('|');
}

const server = http.createServer((req, res) => {
  let body = '';
  req.on('data', (chunk) => {
    body += chunk;
  });
  req.on('end', () => {
    if (req.url !== '/webhook' || req.method !== 'POST') {
      res.writeHead(404).end();
      return;
    }
    const event = JSON.parse(body);
    const key = keyOf(event);
    if (!processed.has(key)) {
      processed.add(key);
      received.push(event);
    }
    res.writeHead(200, { 'content-type': 'application/json' });
    res.end(JSON.stringify({ accepted: true, duplicate: !received.includes(event) }));
  });
});

server.listen(8080, '127.0.0.1', async () => {
  for (const event of fixtures) {
    await fetch('http://127.0.0.1:8080/webhook', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(event),
    });
  }
  console.log(`processed ${received.length} unique events`);
  server.close();
});
```

Point your own handler at `http://127.0.0.1:8080/webhook` instead of the stub endpoint to exercise real consumer code. Keep the fixture loader separate from production configuration so mock events can never be mistaken for network events.

---

## Tests worth running

| Test | Delivery to simulate | Expected consumer behaviour |
|---|---|---|
| In-order stream | Fixtures in ledger order | State advances through `created` → `payment` → `swept_mul` / `sweep` |
| Duplicate delivery | Send the same fixture twice | Process once; the dedup key suppresses the second |
| Out-of-order delivery | Send ledger 1002 before 1001 | Buffer or reject by ledger order; do not treat a later ledger as a reset |
| Same event, different index | Duplicate the payload with `eventIndex: 1` | Process both: they are distinct events |
| Missing event | Drop `payment` from the stream | Fail a completeness assertion if the test expects it; never infer it from `swept_mul` |
| Reorg-like reset | Emit a `local-2` epoch whose ledger is lower than `local-1` | Quarantine old-epoch state, require re-verification, do not merge epochs |
| Idempotent sweep handler | Deliver `sweep` twice | Side effects happen once; a `SweepCompleted` event is not proof of value movement |
| Unknown field | Add a field to `value` | Surface it; do not drop it silently as WASM interface drift |

The reset case mirrors `testnet/policies/reset-policy.md`: a lower ledger is the signal, and a new epoch means previously stored IDs and nonces have no meaning. `testnet/examples/idempotency-considerations.md` explains why a duplicate sweep is a state question, not a retry.

---

## Validating against real payloads later

Nothing can be validated on-chain while the contracts are undeployed. Once `testnet/registry/contract-status.md` records a verified deployment:

1. **Verify the ID** with `testnet/registry/verify-contract-live.md`. Do not proceed on an ID copied from an older artifact.
2. **Capture records** with a bounded ledger range, for example:

   ```bash
   stellar events --id "$EPHEMERAL_ACCOUNT_ID" \
     --start-ledger "$START_LEDGER" \
     --network testnet \
     --output json

   stellar events --id "$SWEEP_CONTROLLER_ID" \
     --start-ledger "$START_LEDGER" \
     --network testnet \
     --output json
   ```

   `stellar events` is a top-level command in CLI 23.4.1; `stellar contract events` does not exist. `--id`, `--start-ledger`, `--cursor`, `--topic`, `--type`, and `--output` are valid options; `--contract-id` and a JSON `--filter` are not. If you filter by topic, `--topic` takes base64-encoded `ScVal` strings rather than plain symbols.

3. **Decode `topic` and `value`** with the same SDK version used to build the transaction, then compare field names, types, and `topic[0]` symbols with `testnet/registry/event-topics.md` and your fixtures.
4. **Update fixtures when the ABI changes.** Record the difference rather than making the decoder silently accept it; `testnet/registry/upgrade-history.md` is where an in-place upgrade belongs.
5. **Keep the facts separate.** A matching event, a successful transaction, and a changed destination balance are three independent checks — see `testnet/examples/README.md`.

---

## Related Documentation

- `testnet/registry/event-topics.md` — source of truth for topic symbols and payload structs
- `testnet/registry/contract-status.md` — canonical deployment status; nothing is live today
- `testnet/tooling/event-tail-spec.md` — ledger cursors, dedup tuple, and reset handling
- `testnet/integration/sdk-checklist.md` — wrapper requirements including event simulation
- `testnet/examples/README.md` — events, transaction results, and balances as separate facts
- `testnet/examples/idempotency-considerations.md` — duplicate sweeps and safe retries
- `testnet/policies/reset-policy.md` — reset detection and what does not survive
- `testnet/registry/verify-contract-live.md` — pre-flight ID verification
- `testnet/monitoring/event-watchlist.md` — event/absence policy for indexers

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-11-18 | Initial local event fixtures, mock webhook delivery, and idempotency test guidance |
