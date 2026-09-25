# Runbook: Investigate Suspected Reentrancy on Testnet

## Scope

Use this evidence-first checklist when a live transaction appears to enter an
`EphemeralAccount`, `SweepController`, or recorded SEP-41 asset more often than
expected.

This procedure complements [`../../docs/reentrancy-analysis.md`](../../docs/reentrancy-analysis.md).
It does not repeat the design analysis and does not classify an incident by
event order alone.

A duplicate relayer submission, a replay after `Swept`, a direct controller
call, or a later `record_payment` transaction is not, by itself, reentrancy.

## Immediate safety actions

If there is an unexpected destination, duplicate transfer, or unauthorized
state change:

1. Pause dependent sweep, claim, payment-recording, expiry/recovery, and
   relayer automation.
2. Preserve transaction hashes, ledgers, contract IDs, current controller
   nonce, worker queues, and exact errors.
3. Do not blindly retry the suspect transaction or later queued work.
4. Do not upgrade or redeploy as an investigation step. Any emergency mutation
   needs separate authorization and a recorded rationale.
5. Use private vulnerability reporting rather than a public issue while
   unauthorized effects are possible.
6. Preserve logs and raw diagnostics before rotating credentials or queues.

Never collect secret keys, seed phrases, or reusable auth payloads in this
runbook.

## Current-source baseline

These observations apply to the checked-in source, not automatically to the
WASM running on testnet:

| Area | Current behavior | Investigation implication |
|---|---|---|
| `EphemeralAccount::sweep` | sets `Swept` and `swept_to` before event/reserve work and returns | a second committed account sweep should fail the terminal-state guard |
| `SweepController::execute_sweep` | verifies authorization, invokes the account, then invokes token transfers | nested calls and repeated effects must be correlated in the full transaction trace |
| controller nonce | advances within the signed sweep transaction | a reverted transaction must not persist the increment |
| `record_payment` | records positive distinct-asset metadata; current code has no terminal-status check | a later payment event can be a separate direct call, not intra-transaction reentry |
| `claim` | uses an account-side claim path and emits a controller `sweep` event in current source | correlate invocation and balances; an event alone does not identify one path |

The design analysis claims Soroban execution/storage protections and a
state-write-first guard. Treat that document as rationale. The deployed code,
interface, and transaction trace are the evidence for a specific incident.

## 1. Identify the exact target

Record:

- network and exact passphrase;
- Soroban RPC and Horizon URLs;
- `EphemeralAccount` contract ID;
- `SweepController` contract ID;
- every recorded SEP-41 asset contract ID;
- entry point: `execute_sweep`, `claim`, direct account sweep, or unknown;
- destination, controller nonce, and payment count;
- UTC time, ledger, and transaction hash; and
- all client/worker submissions associated with the same intent.

Do not infer the live contract from a stale registry record alone.

## 2. Bind the observation to a deployed build

Collect, where possible:

- deployed commit/release;
- contract code/WASM hash;
- contract interface and error-code scheme;
- upgrade/redeployment history; and
- initialization/upgrade transaction evidence.

If the deployed identity is unknown, record it as unknown. Do not assert that
current source behavior was active on testnet.

## 3. Capture the complete transaction result

Preserve diagnostic events, not only the client exception:

```bash
curl --silent --show-error \
  -H 'Content-Type: application/json' \
  --data "{
    \"jsonrpc\": \"2.0\",
    \"id\": 1,
    \"method\": \"getTransaction\",
    \"params\": {\"hash\": \"$TX_HASH\"}
  }" \
  "$STELLAR_SOROBAN_RPC_URL"
```

Retain:

- final success/failure status and failure result;
- inclusion ledger and close time;
- every host-function/contract operation;
- Soroban metadata and diagnostic events;
- authorization frames and sub-invocations;
- contract IDs and function names in invocation order;
- return values and trap locations; and
- token calls made by the same transaction.

A strong reentrancy indicator is the target contract being entered again while
the original target invocation frame is still active, with an additional
committed effect. Similar behavior in a later transaction is sequencing or
replay, not reentrancy.

## 4. Capture events for the affected ledger

Query the account and controller in the same narrow range:

```bash
curl --silent --show-error \
  -H 'Content-Type: application/json' \
  --data @- \
  "$STELLAR_SOROBAN_RPC_URL" <<JSON
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "getEvents",
  "params": {
    "startLedger": $TX_LEDGER,
    "endLedger": $TX_LEDGER,
    "filters": [{
      "type": "contract",
      "contractIds": ["$EPHEMERAL_ACCOUNT_ID", "$SWEEP_CONTROLLER_ID"],
      "topics": []
    }]
  }
}
JSON
```

Run a separate narrow query for each relevant token contract. Preserve raw
event payloads, operation index, transaction hash, and contract ID.

Diagnostic events and invocation metadata are required to reconstruct a failed
or nested call. Event order is supporting evidence, not the full call tree.

## 5. Capture state and balances

Use read-only simulations for the account and controller:

```bash
stellar contract invoke \
  --simulate-only \
  --id "$EPHEMERAL_ACCOUNT_ID" \
  --network testnet \
  --source "$READER_IDENTITY" \
  -- \
  get_info

stellar contract invoke \
  --simulate-only \
  --id "$EPHEMERAL_ACCOUNT_ID" \
  --network testnet \
  --source "$READER_IDENTITY" \
  -- \
  get_status

stellar contract invoke \
  --simulate-only \
  --id "$SWEEP_CONTROLLER_ID" \
  --network testnet \
  --source "$READER_IDENTITY" \
  -- \
  get_nonce
```

Also record:

- payment list captured in the event or state;
- source and destination SEP-41 balance for each asset;
- token transfer events and transaction operations;
- previous known controller nonce; and
- any other transaction in the same ledger that could explain the effect.

The recorded payment list is metadata, not proof of actual balance.

## Expected committed effects

For one successful current-source `execute_sweep`:

- the account becomes `Swept`;
- the controller nonce advances once from its prior value;
- one account `swept_mul` event is committed;
- one controller `sweep` event is committed; and
- one transfer occurs for each recorded payment asset.

For a reverted transaction, none of that transaction's state, nonce, events, or
transfers should remain committed.

If a failed transaction is followed by changed on-chain state, first investigate
another transaction, a testnet reset, stale observations, or a different
deployed build. Do not label non-atomic behavior without transaction evidence.

## Indicators that require more evidence

| Observation | Initial interpretation |
|---|---|
| multiple client submissions, one successful hash/effect | duplicate submission or relayer race |
| sequential `AlreadySwept` diagnostics, no second committed sweep | replay protection |
| `payment`/`multi_pay` after `swept_mul` | later direct call or lifecycle issue; inspect the separate transaction |
| more transfers than recorded assets | duplicate transfer, recorded-data issue, loop, or callback; inspect invocation tree |
| same target entered recursively in one active invocation tree with extra committed effects | strong reentrancy indicator |
| different destination | authorization/configuration issue until reentrancy is proven |
| multiple `reserve` events | reclaim is repeatable; not sufficient evidence of reentry |

Use [`../registry/event-topics.md`](../registry/event-topics.md) for names, but
treat the deployed interface and raw payload as authoritative.

## 6. Controlled reproduction

Only reproduce after preserving live evidence and identifying the deployed
build.

Requirements:

- use an isolated throwaway account and assets;
- obtain approval for any new callback-capable test contract;
- record the exact commit, interface, expected invocation tree, and maximum
  loss bound;
- never use the shared deployment or real-value accounts;
- do not modify a live suspect contract to force a reproduction; and
- use `inconclusive` when safe reproduction is not possible.

Calling a function twice sequentially is not a reentrancy reproduction.

## 7. Classify and record

Use one conclusion:

- **Supported reentrancy:** one committed transaction demonstrably re-enters the
  target while active and causes an additional unauthorized committed effect.
- **Not reentrancy:** evidence identifies replay, duplicate submission, normal
  sequencing, a direct later call, or an authorization/configuration error.
- **Inconclusive:** invocation, deployment identity, or state evidence is
  missing.

Record confidence and remaining unknowns. Complete
[`incident-postmortem-template.md`](incident-postmortem-template.md) and route
suspected vulnerabilities through private security reporting.

## Related documents

- [`../../docs/reentrancy-analysis.md`](../../docs/reentrancy-analysis.md)
- [`../../docs/security.md`](../../docs/security.md)
- [`../registry/event-topics.md`](../registry/event-topics.md)
- [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md)
- [`on-call-testnet.md`](on-call-testnet.md)
- [`rpc-outage.md`](rpc-outage.md)
