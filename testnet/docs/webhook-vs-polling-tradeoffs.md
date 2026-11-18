# Polling vs Webhooks/Indexer for Testnet Events

How an integrator can learn that something happened to an ephemeral account:
**poll** Soroban RPC yourself, or run a dedicated **indexer/webhook relay**.
This page helps smaller integrators start with polling and know when to move on.

> Guidance from the repository's event model and standard Soroban RPC behaviour;
> not benchmarked.

## Facts that shape the choice

- Contract events are served by **Soroban RPC** (`getEvents`, `stellar contract
  events`), **not Horizon** (`testnet/runbooks/event-not-appearing-in-horizon.md`).
- Soroban RPC retains only a **limited window** of recent ledgers; older events
  cannot be fetched later.
- Event names/payloads are in `testnet/registry/event-topics.md`; there are only a
  handful of event types per account lifecycle
  (`testnet/examples/monitoring-a-single-account-lifecycle.md`).
- `get_status` is authoritative state; events are notifications.

## Comparison

| | Polling Soroban RPC | Indexer + webhook relay |
|---|---|---|
| Setup effort | Minimal: a loop and a cursor | A service, storage, deployment, monitoring |
| Latency | Poll interval (≈ ledger close, ~5 s, at best) | Near real-time after ingestion, plus delivery time |
| Reliability | You must persist the cursor and handle restarts | Relay handles retries and dedupe if built well |
| Gaps | Missed events if you stop polling longer than the retention window | Same retention limit unless you persist as you go |
| Scale | Fine for a few accounts; wasteful for many | Scales to many consumers |
| Cost | RPC calls (rate limits apply) | Infrastructure cost |
| Complexity for consumers | Each consumer implements its own logic | One normalized feed |
| Fit | Testing, small integrations, prototypes | Production-like load, multiple consumers, alerting |

## Polling: a sound starting design

1. Persist `lastLedger`/cursor durably.
2. Each interval call `getEvents` with filters for your contract IDs and topics
   (`created`, `payment`, `multi_pay`, `swept_mul`, `expired`, `sweep`, ...),
   starting from `lastLedger`.
3. **Follow pagination** until the page is empty.
4. **De-duplicate** by `(contract, ledger, tx hash, event index)`.
5. Advance the cursor only after processing succeeds.
6. **Reconcile** with read calls (`get_status`, `is_expired`) periodically, and
   always before acting on an event.
7. Backoff on errors; stop and alert on outage
   (`testnet/runbooks/rpc-outage.md`).
8. Keep the interval modest (a few seconds to tens of seconds), respect rate limits.

For a handful of accounts, a few polls per minute is enough. For **expiry**, don't
wait for an event: compare the current ledger with `expiry_ledger`, since
`expired` is emitted only when someone calls `expire()`/`recover()`
(`testnet/monitoring/alert-thresholds.md`).

## When to move to an indexer/webhook relay

- More than a few hundred watched accounts, or several consuming services.
- You need events older than the RPC retention window.
- You need push delivery to external systems or guaranteed delivery with retries.
- You are building dashboards/analytics over history
  (`testnet/monitoring/dashboards-plan.md`).

## Hybrid

Poll for correctness-critical flows (state reads) and add webhooks for
notifications; use events to trigger a state read, not to replace it.

## Pitfalls

- Treating a missing event as proof nothing happened.
- Resetting the cursor to "latest" on restart and losing events.
- Assuming order across contracts; order by ledger and position.
- Forgetting testnet resets, which make cursors point at a ledger range that no
  longer exists (`testnet/policies/reset-policy.md`).
