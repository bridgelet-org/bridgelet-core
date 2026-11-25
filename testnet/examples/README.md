# Testnet Integration Examples

This directory is the home for small, runnable-in-spirit code snippets,
walkthroughs, and integration patterns demonstrating how to interact with
Bridgelet contracts on Stellar Testnet.

While [`docs/api-reference.md`](../../docs/api-reference.md) defines abstract
interface signatures, method parameters, and type definitions, the examples in
this directory show how those pieces fit together in real integration flows:
which preconditions to verify before calling a method, how arguments are
encoded, how to simulate read-only calls, how to serialize signing and nonce
handling, and how to verify on-chain state and balances after submission.

> **Testnet Notice:** The code snippets here are integration patterns and
> operator references, not a packaged production SDK. Stellar Testnet is reset
> periodically, deployment records can change, and contract IDs must always be
> verified live before use.

## Integration Patterns & Languages

The examples in this directory illustrate patterns across multiple toolchains:

- **Stellar CLI (`stellar contract ...`):** Canonical shell commands for
  operators and developers running simulations, manual invocations, and event
  queries.
- **JavaScript / TypeScript SDK (`@stellar/stellar-sdk`):** Client-side
  patterns for account generation, simulation via `simulateTransaction`, XDR
  envelope construction, and Ed25519 hash signing.
- **Direct Soroban RPC (JSON-RPC / HTTP):** Raw request payloads for Postman,
  curl, or custom HTTP clients interacting with `simulateTransaction`,
  `sendTransaction`, and `getEvents`.
- **Rust Contract Client / In-Process Testing:** Rust patterns for backend
  relayers and service components.

## Before running an example

1. **Check network configuration:** Use the exact passphrase and RPC settings in
   [`../docs/network-config.md`](../docs/network-config.md).
2. **Verify live contract IDs:** Confirm deployment IDs via read-only simulation
   or [`../registry/contract-status.md`](../registry/contract-status.md) rather
   than assuming static IDs are live.
3. **Use funded testnet identities:** Use a funded testnet `G...` identity for
   transaction fees. Never commit or log secret keys (`S...`) or Ed25519 signing seeds.
4. **Distinguish simulation from submission:** Always run `--simulate-only` or
   `simulateTransaction` first to check return values and preconditions without
   spending fees or advancing state.
5. **Separate transaction success, events, and balance changes:** A complete
   sweep or claim verification confirms the transaction status, inspects
   contract state (`get_status == Swept`), and validates the recipient's token
   balance.

Common environment variables:

```bash
export NETWORK=testnet
export RPC_URL="https://soroban-testnet.stellar.org"
export READER="<funded-testnet-identity>"
export CREATOR="<ephemeral-account-creator>"
export PAYER="<xlm-payer>"
export RELAYER="<sweep-submitter>"

export EPHEMERAL_ACCOUNT_ID="<verified-ephemeral-account-id>"
export SWEEP_CONTROLLER_ID="<verified-sweep-controller-id>"
export EPHEMERAL_ACCOUNT_WASM_HASH="<verified-wasm-hash>"
export XLM_SAC="CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC"
```

## Example index

### 1. Account Lifecycle & Management

| Example | Pattern demonstrated | Execution profile |
|---|---|---|
| [Read account state](#read-account-state) | Read-only simulation to decode status, expiry, and recorded payments | Read-only |
| [Create and initialize](#create-and-initialize) | Deploy a fresh ephemeral account and invoke `initialize` with controller linkage | State-changing |
| [Pre-flight account status checks](checking-account-status.md) | Safely gating calls using composite checks (`is_expired`, `get_info`, `can_sweep`) | Read-only |
| [Monitoring a single account's lifecycle](monitoring-a-single-account-lifecycle.md) | Reconstructing full account history from deployed events to terminal state | Read-only |
| [Simulating batch account creation](batch-account-creation-mock.md) | Loop individual deployments while `account_factory` is undeployed on testnet | State-changing |
| [Expiry and recovery](expiry-and-recovery.md) | Handling expired accounts and executing `recover` to return un-swept funds | State-changing |

### 2. Payment & Sweep Workflows

| Example | Pattern demonstrated | Execution profile |
|---|---|---|
| [Read account state](#read-account-state) | Decode status, expiry, and recorded payments | Read-only |
| [Check sweep readiness](#check-sweep-readiness) | Cross-check `can_sweep` and controller nonce | Read-only |
| [Create and initialize](#create-and-initialize) | Deploy a fresh ephemeral account | State-changing |
| [Pay and record](#pay-and-record) | Transfer native XLM, then record it in contract state | State-changing |
| [Authorize and sweep](#authorize-and-sweep) | Submit a signed `SweepController::execute_sweep` | State-changing |
| [Inspect events and balances](#inspect-events-and-balances) | Correlate events with the final terminal state | Read-only |
| [Pre-flight account status checks](checking-account-status.md) | Gate a state-changing call on `is_expired`, `get_info` and `can_sweep` | Read-only |
| [Error handling](error-handling.md) | Map each contract error discriminant to handling code | Read-only |
| [Idempotency considerations](idempotency-considerations.md) | Reason about a duplicate sweep and the one safe retry | Read-only |
| [Signed sweep walkthrough](signed-sweep-walkthrough.md) | Primary path: Ed25519-signed `execute_sweep` | State-changing |
| [Gas-free claim walkthrough](gas-free-claim-walkthrough.md) | Recipient-signed, relayer-submitted `claim()` | State-changing |
| [Multi-asset sweep](multi-asset-sweep.md) | Sweep several recorded assets and read `SweepExecutedMulti` | State-changing |
| [Expiry and recovery](expiry-and-recovery.md) | Let an account expire and verify the recovery path | State-changing |
| [Reading contract storage](reading-contract-storage.md) | Inspect persistent storage through RPC | Read-only |
| [Integration test harness notes](integration-test-harness-notes.md) | Test against live testnet versus the in-process test environment | Read-only |
| [Simulating batch account creation](batch-account-creation-mock.md) | Loop individual deployments while `account_factory` is unavailable | State-changing |
| [Create and fund an ephemeral account](create-and-fund-ephemeral-account.md) | `initialize()`, fund with XLM, then `record_payment()` with expected `created`/`payment` events | State-changing |
| [Monitoring a single account's lifecycle](monitoring-a-single-account-lifecycle.md) | Order the event sequence from creation to sweep or expiry | Read-only |
| [Testing locked mode](testing-locked-mode.md) | Initialize a locked controller and confirm other destinations are rejected | State-changing |
| [Update authorized destination](update-authorized-destination-flow.md) | `update_authorized_destination` and its `dest_upd` event | State-changing |

## Keeping this index current

Whenever an example file is added or modified in `testnet/examples/`, keep this
index table synchronized in the same pull request. Verify indexing completeness
with:

```bash
for f in testnet/examples/*.md; do
  b=$(basename "$f"); [ "$b" = README.md ] && continue
  grep -q "($b)" testnet/examples/README.md || echo "not indexed: $b"
done
```

## Read account state

`get_info` is the strongest account-level probe because it proves the instance
is initialized. `get_status` alone returns `Active` even for an uninitialized
instance, so do not use `0` as proof of initialization.

```bash
stellar contract invoke \
  --simulate-only \
  --id "$EPHEMERAL_ACCOUNT_ID" \
  --network "$NETWORK" \
  --source "$READER" \
  -- \
  get_info

stellar contract invoke \
  --simulate-only \
  --id "$EPHEMERAL_ACCOUNT_ID" \
  --network "$NETWORK" \
  --source "$READER" \
  -- \
  is_expired
```

The current `AccountStatus` values are:

| Value | State |
|---:|---|
| `0` | `Active` |
| `1` | `PaymentReceived` |
| `2` | `Swept` |
| `3` | `Expired` |

A recorded `Payment` contains an asset contract address, an integer amount in
that asset's base units, and the ledger timestamp at which it was recorded.
`record_payment` does not verify or transfer the underlying asset.

## Check sweep readiness

```bash
stellar contract invoke \
  --simulate-only \
  --id "$SWEEP_CONTROLLER_ID" \
  --network "$NETWORK" \
  --source "$READER" \
  -- \
  can_sweep \
  --ephemeral_account "$EPHEMERAL_ACCOUNT_ID"

stellar contract invoke \
  --simulate-only \
  --id "$SWEEP_CONTROLLER_ID" \
  --network "$NETWORK" \
  --source "$READER" \
  -- \
  get_nonce
```

A readiness pass means the account has a recorded payment, its status is
`PaymentReceived`, and it has not reached `expiry_ledger`. The nonce is global
to the controller, not per account. Fetch it immediately before signing and
serialize submissions so two workers cannot sign the same value.

## Create and initialize

This abbreviated shell flow shows the integration order. The operator must
confirm the WASM hash, current ledger, and argument encoding before submitting
anything.

```bash
LATEST_LEDGER="$(
  curl -fsS "$RPC_URL" \
    -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"getLatestLedger"}' |
  jq -r '.result.sequence'
)"

NEW_ACCOUNT_ID="$(
  stellar contract deploy \
    --wasm-hash "$EPHEMERAL_ACCOUNT_WASM_HASH" \
    --network "$NETWORK" \
    --source "$CREATOR"
)"

EXPIRY_LEDGER=$((LATEST_LEDGER + 1000))

stellar contract invoke \
  --id "$NEW_ACCOUNT_ID" \
  --network "$NETWORK" \
  --source "$CREATOR" \
  -- \
  initialize \
  --creator "$(stellar keys address "$CREATOR")" \
  --expiry_ledger "$EXPIRY_LEDGER" \
  --recovery_address "$(stellar keys address "$PAYER")" \
  --authorized_controller "$SWEEP_CONTROLLER_ID" \
  --admin "$(stellar keys address "$CREATOR")"
```

Use a new value for `NEW_ACCOUNT_ID` on each run. A deployment transaction and
an `initialize` transaction are separate steps, and an uncertain result must
be queried before it is retried.

## Pay and record

The example transfers `0.1` XLM (`1,000,000` stroops) to the fresh account,
verifies the SEP-41 balance, and then records the same amount and asset.

```bash
export CANARY_AMOUNT_STROOPS=1000000

stellar contract invoke \
  --id "$XLM_SAC" \
  --network "$NETWORK" \
  --source "$PAYER" \
  -- \
  transfer \
  --to "$NEW_ACCOUNT_ID" \
  --amount "$CANARY_AMOUNT_STROOPS"

stellar contract invoke \
  --simulate-only \
  --id "$XLM_SAC" \
  --network "$NETWORK" \
  --source "$READER" \
  -- \
  balance \
  --id "$NEW_ACCOUNT_ID"

stellar contract invoke \
  --id "$NEW_ACCOUNT_ID" \
  --network "$NETWORK" \
  --source "$READER" \
  -- \
  record_payment \
  --amount "$CANARY_AMOUNT_STROOPS" \
  --asset "$XLM_SAC"
```

Do not call `record_payment` before the real transfer has finalized. If the
transfer result is uncertain, preserve its transaction hash and inspect the
token balance instead of retrying either operation blindly.

## Authorize and sweep

The controller verifies an Ed25519 signature over the current controller
nonce, the exact destination, and the controller contract ID. Keep signing
and submission serialized per `SweepController` deployment.

```text
SHA256(
  destination.to_xdr()
  || nonce encoded as an unsigned u64 in 8-byte big-endian order
  || sweep_controller_id.to_xdr()
)
```

After a fresh `get_nonce` result, an SDK or signer can produce the invocation
shown here conceptually:

```text
execute_sweep(
  ephemeral_account = NEW_ACCOUNT_ID,
  destination = CANARY_DESTINATION,
  auth_signature = SIGNATURE_FROM_FRESH_NONCE
)
```

```bash
stellar contract invoke \
  --id "$SWEEP_CONTROLLER_ID" \
  --network "$NETWORK" \
  --source "$RELAYER" \
  -- \
  execute_sweep \
  --ephemeral_account "$NEW_ACCOUNT_ID" \
  --destination "<canary-destination-address>" \
  --auth_signature "<64-byte-signature>"
```

A complete success check requires:

1. the transaction has a successful final status;
2. `get_status` is `2` (`Swept`);
3. `get_info.swept_to` is the requested destination;
4. `can_sweep` is now `false`;
5. the controller nonce advanced by one for `execute_sweep`; and
6. the destination balance changed by the recorded amount for each asset.

## Inspect events and balances

Choose a start ledger before the workflow, then query both the child account
and controller.

```bash
export START_LEDGER="<ledger-before-the-run>"

stellar contract events \
  --contract-id "$NEW_ACCOUNT_ID" \
  --start-ledger "$START_LEDGER" \
  --filter '{"topics":[["created"],["payment"],["multi_pay"],["swept_mul"]]}' \
  --network "$NETWORK"

stellar contract events \
  --contract-id "$SWEEP_CONTROLLER_ID" \
  --start-ledger "$START_LEDGER" \
  --filter '{"topics":[["sweep"]]}' \
  --network "$NETWORK"
```

For a normal one-asset `execute_sweep` flow, correlate `created`, `payment`,
`swept_mul`, and the controller's `sweep` event with the same run. A missing
event is not enough by itself to classify a transaction as failed; retain the
transaction result and decoded contract state.

## Related documents

- [`../../docs/api-reference.md`](../../docs/api-reference.md) — abstract
  contract interface signatures and type definitions
- [`../../docs/SIGNATURE_FORMAT.md`](../../docs/SIGNATURE_FORMAT.md) — signature
  format details and hashing specification
- [`../monitoring/synthetic-transaction-plan.md`](../monitoring/synthetic-transaction-plan.md)
  — proposal for a periodic end-to-end canary
- [`../monitoring/explorer-links.md`](../monitoring/explorer-links.md) — links
  for recorded contract IDs
- [`../docs/getting-started.md`](../docs/getting-started.md) — broader
  setup and troubleshooting
- [`../registry/contract-status.md`](../registry/contract-status.md) — current
  testnet deployment status table
