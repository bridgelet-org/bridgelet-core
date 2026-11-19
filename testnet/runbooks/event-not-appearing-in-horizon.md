# Runbook: Transaction Succeeded but the Contract Event Is Not Showing

## Symptom

An explorer (e.g. Stellar Expert) shows your transaction as successful, but the
contract event you expect (`created`, `payment`, `swept_mul`, `sweep`, ...) is
missing from your event query.

## First, check you are querying the right API

**Horizon does not serve Soroban contract events.** Contract events come from
Soroban RPC `getEvents` (or `stellar contract events`). Horizon exposes classic
operations/effects and transaction results. If your tooling reads "events" from a
Horizon endpoint, an empty result is expected. Use Soroban RPC for contract
events and Horizon for classic payment evidence
(`testnet/runbooks/rpc-outage.md` explains the split).

## Checklist

1. **Indexing lag.** Query again after a few ledgers. Compare your query's
   ledger window against `getLatestLedger`; the transaction's ledger must be
   within the window and already ingested.
2. **Retention window.** Soroban RPC keeps only a limited number of recent
   ledgers. If the transaction is older than the retained range, `getEvents`
   returns nothing or errors with "startLedger must be within the ledger range".
   Capture events as they occur for anything you need long-term.
3. **Wrong contract ID.** Filter on the contract that emits the event.
   `swept_mul`, `payment`, `created` come from the **EphemeralAccount**;
   `sweep`, `dest_auth`, `dest_upd` come from the **SweepController**. Querying
   the controller for `payment` returns nothing by design.
4. **Wrong topic filter.** Event names are `symbol_short!` values (e.g. `swept_mul`,
   `multi_pay`); match the exact symbol from `testnet/registry/event-topics.md`
   and encode it as an `ScVal` symbol, not a string.
5. **Wrong event type filter.** Contract events have type `contract`; make sure
   the filter does not restrict to `system` or `diagnostic`.
6. **Transaction did not emit it.** A successful transaction does not imply the
   event. Examples: `record_payment` on a later asset emits `multi_pay` (not
   `payment`); a failed sub-invocation rolls back its events; a
   read-only/simulated call emits nothing on-chain.
7. **Wrong network.** The explorer and your RPC must be the same network
   (`testnet/docs/network-config.md`).
8. **Pagination.** If you used a limit, follow the cursor; events beyond the
   first page are easy to miss.

## Verify independently

```bash
# Fetch the transaction and confirm its ledger and success
curl -s https://soroban-testnet.stellar.org -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getTransaction","params":{"hash":"<TX_HASH>"}}'

# Then query events narrowly around that ledger
stellar contract events --id "$CONTRACT_ID" --network testnet \
  --start-ledger <TX_LEDGER> --count 20 --output pretty
```

Also confirm the **state** changed with a read-only call
(`get_status`, `get_info`). State is authoritative; events are a notification
path.

## If it is still missing

Record the transaction hash, ledger, contract ID, the exact filter you used, and
the RPC response, then follow `testnet/runbooks/rpc-outage.md` and open an issue.
