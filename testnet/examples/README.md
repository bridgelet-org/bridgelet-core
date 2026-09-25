# Testnet Integration Examples

This directory is the home for small, operational examples that exercise the
current Bridgelet contracts on Stellar Testnet. The examples are intentionally
more concrete than [`docs/api-reference.md`](../../docs/api-reference.md),
which documents the abstract interface and types.

These snippets are integration guides, not a production SDK. Testnet is reset
periodically, deployment records can become stale, and every contract ID must
be verified before it is used.

## Before running an example

- Use the exact network settings in
  [`testnet/docs/network-config.md`](../docs/network-config.md).
- Verify the contract IDs against a live read-only query. The canonical status
  registry currently conflicts with older deployment artifacts, so a copied ID
  is not proof of availability.
- Use a funded testnet `G...` identity for fees. Never put a Stellar secret
  key or Ed25519 signing seed in this repository, a URL, or a log.
- Prefer a fresh `EphemeralAccount` for a create-pay-sweep walkthrough. A
  reusable deployment ID and newly deployed child account are different things.
- Treat successful submission, a contract event, and a balance change as
  separate facts. A complete sweep workflow verifies all three.

Common placeholders:

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

| Example | Use it to exercise | Network effect |
|---|---|---|
| [Read account state](#read-account-state) | Decode status, expiry, and recorded payments | Read-only |
| [Check sweep readiness](#check-sweep-readiness) | Cross-check `can_sweep` and controller nonce | Read-only |
| [Create and initialize](#create-and-initialize) | Deploy a fresh ephemeral account | State-changing |
| [Pay and record](#pay-and-record) | Transfer native XLM, then record it in contract state | State-changing |
| [Authorize and sweep](#authorize-and-sweep) | Submit a signed `SweepController::execute_sweep` | State-changing |
| [Inspect events and balances](#inspect-events-and-balances) | Correlate events with the final terminal state | Read-only |

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

- [`synthetic-transaction-plan.md`](../monitoring/synthetic-transaction-plan.md)
  — proposal for a periodic end-to-end canary
- [`explorer-links.md`](../monitoring/explorer-links.md) — links for recorded
  contract IDs
- [`testnet/docs/getting-started.md`](../docs/getting-started.md) — broader
  setup and troubleshooting
- [`docs/SIGNATURE_FORMAT.md`](../../docs/SIGNATURE_FORMAT.md) — signature
  format details
