# Worked Example: Create and Fund an Ephemeral Account

## Overview

This is the create-then-pay walkthrough: create an `EphemeralAccount` instance,
call `initialize()` once, move real XLM into it, and call `record_payment()` so
the contract's bookkeeping matches the balance. It is the starting point for the
[signed sweep](signed-sweep-walkthrough.md) and
[gas-free claim](gas-free-claim-walkthrough.md) walkthroughs.

The four facts are deliberately kept separate:

| Fact | How you establish it |
|---|---|
| The contract instance exists | `stellar contract deploy` returns an ID |
| The instance is initialized | `initialize()` succeeds; `get_info()` stops returning `NotInitialized` |
| XLM actually moved | SEP-41 `transfer` plus a `balance` read on the native asset contract |
| The payment is recorded | `record_payment()` succeeds; `get_status()` is `1`, `payment_count` is `1` |

> `record_payment()` **does not transfer tokens**. It only writes `{asset,
> amount, timestamp}` into contract storage. Skipping the real transfer leaves
> an account whose later sweep fails with `TransferFailed` in
> `transfers::execute_transfers()`.

## Status of this example

The command shapes below come from `contracts/ephemeral_account/src/lib.rs`,
`contracts/ephemeral_account/src/events.rs` and the CLI conventions in
`testnet/docs/soroban-cli-quickref.md` (checked against `stellar` 23.4.1). They
have not been captured from a live run.

The contract ID is **read from the registry at run time**, never copied into
this file. `testnet/registry/contract-status.md` has two tables that disagree:
the older deployment-status table says `TBD` / `Never`, while the
"Quick reference (one-line status)" table at the bottom records the supported
`ephemeral_account` deployment. Use the quick-reference row and re-verify it
with `testnet/registry/verify-contract-live.md` before depending on it. IDs on
testnet can change on redeploy or reset; a stale ID is the most common cause of
a failed first call.

## Prerequisites

- `stellar` CLI 23.4.1 or later, `curl`, and `jq`.
- Two funded testnet identities: one to deploy and sign `initialize()` as
  `creator`, and one to pay for the XLM transfer. A single identity can play
  every role for a smoke test.
- `testnet/registry/contract-status.md` and `deployments/testnet.json` in the
  current checkout.

```bash
export NETWORK=testnet
export RPC_URL="https://soroban-testnet.stellar.org"
export CREATOR="<funded-testnet-identity>"      # signs initialize()
export PAYER="<funded-testnet-identity>"        # funds the new instance
export READER="<funded-testnet-identity>"       # read-only calls and record_payment()
```

Never put a secret key or signing seed in a shell history, URL, log, or this
repository. See `testnet/config/soroban-cli-identity-setup.md`.

---

## Step 1: Resolve the live IDs from the registry

Read the `ephemeral_account` (and `sweep_controller`) rows of the quick-reference
table rather than pasting an ID from an older document:

```bash
REGISTRY=testnet/registry/contract-status.md

EPHEMERAL_ACCOUNT_ID="$(
  sed -n 's/^| ephemeral_account | Y[^|]*| `\([A-Z2-7]\{56\}\)`.*/\1/p' \
    "$REGISTRY" | head -n 1
)"
SWEEP_CONTROLLER_ID="$(
  sed -n 's/^| sweep_controller | Y[^|]*| `\([A-Z2-7]\{56\}\)`.*/\1/p' \
    "$REGISTRY" | head -n 1
)"

: "${EPHEMERAL_ACCOUNT_ID:?No live ephemeral_account ID in $REGISTRY - re-verify the registry}"
: "${SWEEP_CONTROLLER_ID:?No live sweep_controller ID in $REGISTRY - re-verify the registry}"
echo "ephemeral_account (registry): $EPHEMERAL_ACCOUNT_ID"
echo "sweep_controller  (registry): $SWEEP_CONTROLLER_ID"
```

The `sed` extracts the first backticked `C...` ID on a `Y` row. If the table
format ever changes, copy the IDs from that table by hand instead — the point
is that the IDs come from the current registry, not from this page.

Prove the ID is live before sending anything (`--send no` simulates; it does not
sign, submit, or cost a fee):

```bash
stellar contract invoke \
  --id "$EPHEMERAL_ACCOUNT_ID" \
  --network "$NETWORK" \
  --source "$READER" \
  --send no \
  -- \
  get_info
```

- `Error(Contract, #2)` (`NotInitialized`) means the ID is a fresh instance and
  you can initialize it directly: set `EA_ID="$EPHEMERAL_ACCOUNT_ID"` and skip
  to Step 3.
- `Ok(AccountInfo)` means this instance is already initialized — including the
  shared deployment, which is used by other integrators. `initialize()` on it
  returns `AlreadyInitialized` (`#1`). **Do not initialize someone else's
  account.** Deploy your own instance in Step 2.
- A "not found" error means the ID is stale; re-run
  `testnet/registry/verify-contract-live.md`.

> Older pages in this tree use `--simulate-only` for read-only calls. On
> `stellar` 23.4.1 the flag is `--send no`; see
> `testnet/docs/soroban-cli-quickref.md`.

---

## Step 2: Deploy a fresh instance (normal case)

One `EphemeralAccount` contract instance represents one ephemeral account, and
`initialize()` may be called only once per instance. Deploy a new instance from
the WASM hash of the verified deployment. Read the hash from
`deployments/testnet.json` (`config.ephemeralAccountWasmHash`) rather than
hardcoding it here:

```bash
EPHEMERAL_WASM_HASH="$(jq -r '.config.ephemeralAccountWasmHash' deployments/testnet.json)"
: "${EPHEMERAL_WASM_HASH:?Missing ephemeralAccountWasmHash in deployments/testnet.json}"

EA_ID="$(stellar contract deploy \
  --wasm-hash "$EPHEMERAL_WASM_HASH" \
  --network "$NETWORK" \
  --source "$CREATOR")"

echo "Fresh EphemeralAccount: $EA_ID"
```

`deployments/testnet.json` is owned by `scripts/deploy-testnet.sh`; re-check it
after a redeploy. A local build can be deployed with `--wasm
target/wasm32v1-none/release/ephemeral_account.wasm` instead (see
`testnet/examples/batch-account-creation-mock.md`), but then you are testing
your build, not the live deployment.

Record the ledger before the state-changing calls so the event query in the
last step has a correct lower bound:

```bash
START_LEDGER="$(curl -fsS "$RPC_URL" \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getLatestLedger"}' \
  | jq -r '.result.sequence')"
```

---

## Step 3: Pick a future `expiry_ledger`

`expiry_ledger` is a **ledger sequence**, not a Unix timestamp, and the account
is expired at `current_ledger >= expiry_ledger` — no grace ledger. `initialize()`
rejects a value that is not strictly in the future.

```bash
LATEST_LEDGER="$(curl -fsS "$RPC_URL" \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getLatestLedger"}' \
  | jq -r '.result.sequence')"

EXPIRY_LEDGER=$((LATEST_LEDGER + 720))   # ~1 hour at ~5 s/ledger
echo "latest=$LATEST_LEDGER  expiry=$EXPIRY_LEDGER"
```

720 ledgers is roughly an hour; pick a larger margin for a longer test. See
`testnet/config/expiry-ledger-testing.md` for the ledger-to-time table and the
reset/ledger-rate caveats.

---

## Step 4: Call `initialize()` once

Five arguments, in this order: `creator`, `expiry_ledger`, `recovery_address`,
`authorized_controller`, `admin`. The deployed source is:

```rust
initialize(creator: Address, expiry_ledger: u32, recovery_address: Address,
           authorized_controller: Address, admin: Address) -> Result<(), Error>
```

```bash
CREATOR_ADDRESS="$(stellar keys address "$CREATOR")"

stellar contract invoke \
  --id "$EA_ID" \
  --network "$NETWORK" \
  --source "$CREATOR" \
  -- \
  initialize \
  --creator "$CREATOR_ADDRESS" \
  --expiry_ledger "$EXPIRY_LEDGER" \
  --recovery_address "$CREATOR_ADDRESS" \
  --authorized_controller "$SWEEP_CONTROLLER_ID" \
  --admin "$CREATOR_ADDRESS"
```

- `creator.require_auth()` runs, so `--source` must be the identity whose
  address is passed as `--creator`.
- `expiry_ledger`, `recovery_address`, `admin` and `authorized_controller` are
  immutable after this call. Use the real `SweepController` ID, not your own
  key, if the account will ever be swept.
- Success publishes the `created` event (`AccountCreated { creator,
  expiry_ledger }`) — see [Expected event output](#expected-event-output).
- `initialize()` on the same instance a second time returns `Error(Contract,
  #1)` (`AlreadyInitialized`). If the result is uncertain, query `get_info()`
  instead of retrying blindly.

---

## Step 5: Fund the account with XLM (real value)

`record_payment()` only records bookkeeping, so send actual XLM first. Amounts
are in stroops: 1 XLM = 10,000,000 stroops, so 10 XLM = 100,000,000 stroops.

```bash
XLM_SAC="$(stellar contract id asset --asset native --network "$NETWORK")"
AMOUNT_STROOPS=100000000

stellar contract invoke \
  --id "$XLM_SAC" \
  --network "$NETWORK" \
  --source "$PAYER" \
  -- \
  transfer \
  --to "$EA_ID" \
  --amount "$AMOUNT_STROOPS"

# Verify the SEP-41 balance before recording anything.
stellar contract invoke \
  --id "$XLM_SAC" \
  --network "$NETWORK" \
  --source "$READER" \
  --send no \
  -- \
  balance \
  --id "$EA_ID"
# → 100000000
```

The `transfer` emits a SEP-41 `transfer` event on the **native asset contract**,
not on `EphemeralAccount`. Keep that transaction hash; if the result is
uncertain, inspect the balance rather than resubmitting the transfer. See
`testnet/examples/README.md` ("Pay and record") and `testnet/faucet/README.md`
for funding etiquette.

---

## Step 6: Call `record_payment()`

```bash
stellar contract invoke \
  --id "$EA_ID" \
  --network "$NETWORK" \
  --source "$READER" \
  -- \
  record_payment \
  --amount "$AMOUNT_STROOPS" \
  --asset "$XLM_SAC"
```

`record_payment(amount: i128, asset: Address)` requires no authorization: any
funded identity may call it. The amount must be positive, the asset must be a
SEP-41 contract address (native XLM is addressed through its SAC), and each
asset may be recorded only once per instance (max 10 assets). The first call on
a fresh account emits `payment` and moves status `Active (0)` →
`PaymentReceived (1)`; later assets emit `multi_pay` and do not change status.
See `testnet/examples/multi-asset-sweep.md`.

---

## Step 7: Verify the resulting state

All three reads are simulated and free:

```bash
stellar contract invoke --id "$EA_ID" --network "$NETWORK" --source "$READER" \
  --send no -- get_info
# → { creator: "<CREATOR_ADDRESS>", status: 1, expiry_ledger: <EXPIRY_LEDGER>,
#     recovery_address: "<CREATOR_ADDRESS>", payment_received: true,
#     payment_count: 1,
#     payments: [ { asset: "<XLM_SAC>", amount: 100000000, timestamp: <u64> } ],
#     swept_to: null }

stellar contract invoke --id "$EA_ID" --network "$NETWORK" --source "$READER" \
  --send no -- get_status
# → 1 (PaymentReceived)

stellar contract invoke --id "$EA_ID" --network "$NETWORK" --source "$READER" \
  --send no -- get_payment_count
# → 1
```

`timestamp` is the ledger timestamp at which `record_payment()` was called, not
the transfer time. `get_info()` is the initialization proof: on an uninitialized
instance it returns `Error(Contract, #2)`, while `get_status()` would misleadingly
return `0` (`Active`). See `testnet/examples/checking-account-status.md`.

---

## Expected event output

Query both events from the fresh instance starting at the ledger captured in
Step 2. On `stellar` 23.4.1 the event command is top-level `stellar events`
with `--id`; older pages in this tree show `stellar contract events` with a
JSON `--filter`, which is not valid for that CLI version
(`testnet/integration/webhook-simulation.md`):

```bash
stellar events \
  --id "$EA_ID" \
  --start-ledger "$START_LEDGER" \
  --network "$NETWORK" \
  --output json
```

Two contract events are expected from this flow, in this order (fields shown
are the decoded payloads; ledger, hash and index values depend on the run):

```json
[
  {
    "contractId": "<EA_ID>",
    "topic": ["created"],
    "value": {
      "creator": "<CREATOR_ADDRESS>",
      "expiry_ledger": 4912345
    }
  },
  {
    "contractId": "<EA_ID>",
    "topic": ["payment"],
    "value": {
      "amount": "100000000",
      "asset": "<XLM_SAC>"
    }
  }
]
```

| Event | `topic[0]` | Payload struct | Emitted by |
|---|---|---|---|
| Account created | `created` | `AccountCreated { creator: Address, expiry_ledger: u32 }` | `initialize()` |
| First payment | `payment` | `PaymentReceived { amount: i128, asset: Address }` | first `record_payment()` |
| Later payment | `multi_pay` | `MultiPaymentReceived { asset: Address, amount: i128 }` | subsequent `record_payment()` |

The topic symbols are published with `symbol_short!`, so they are the exact
plain strings `created` and `payment` in `topic[0]`. Amounts may be rendered as
decimal strings in JSON; `i128` is not a JSON number. A second asset would add a
`multi_pay` record; a second `record_payment()` of the *same* asset emits
nothing because it fails with `DuplicateAsset`. The exact topic table and
payload types are in `testnet/registry/event-topics.md`; the ordering rules for
a full account lifecycle are in
`testnet/examples/monitoring-a-single-account-lifecycle.md`.

An event, a successful transaction, and a balance change are three independent
facts. Confirm all three before treating the account as ready to sweep.

---

## Full script

Copy-paste version of the flow above. It assumes the identities in the
prerequisites already exist and are funded.

```bash
#!/usr/bin/env bash
set -euo pipefail

export NETWORK=testnet
export RPC_URL="https://soroban-testnet.stellar.org"
CREATOR="${CREATOR:?set to a funded testnet identity}"
PAYER="${PAYER:?set to a funded testnet identity}"
READER="${READER:?set to a funded testnet identity}"

# 1. IDs come from the registry, never from this script.
REGISTRY=testnet/registry/contract-status.md
EPHEMERAL_ACCOUNT_ID="$(
  sed -n 's/^| ephemeral_account | Y[^|]*| `\([A-Z2-7]\{56\}\)`.*/\1/p' \
    "$REGISTRY" | head -n 1
)"
SWEEP_CONTROLLER_ID="$(
  sed -n 's/^| sweep_controller | Y[^|]*| `\([A-Z2-7]\{56\}\)`.*/\1/p' \
    "$REGISTRY" | head -n 1
)"
: "${EPHEMERAL_ACCOUNT_ID:?No live ephemeral_account ID in $REGISTRY}"
: "${SWEEP_CONTROLLER_ID:?No live sweep_controller ID in $REGISTRY}"

START_LEDGER="$(curl -fsS "$RPC_URL" -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getLatestLedger"}' | jq -r '.result.sequence')"

# 2. Deploy a fresh instance from the recorded WASM hash.
EPHEMERAL_WASM_HASH="$(jq -r '.config.ephemeralAccountWasmHash' deployments/testnet.json)"
: "${EPHEMERAL_WASM_HASH:?Missing ephemeralAccountWasmHash in deployments/testnet.json}"
EA_ID="$(stellar contract deploy --wasm-hash "$EPHEMERAL_WASM_HASH" \
  --network "$NETWORK" --source "$CREATOR")"
echo "EphemeralAccount: $EA_ID"

# 3. Expiry ~1 hour out, computed from a fresh ledger read.
LATEST_LEDGER="$(curl -fsS "$RPC_URL" -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getLatestLedger"}' | jq -r '.result.sequence')"
EXPIRY_LEDGER=$((LATEST_LEDGER + 720))

# 4. initialize() - once per instance, creator must sign.
CREATOR_ADDRESS="$(stellar keys address "$CREATOR")"
stellar contract invoke --id "$EA_ID" --network "$NETWORK" --source "$CREATOR" \
  -- initialize \
  --creator "$CREATOR_ADDRESS" \
  --expiry_ledger "$EXPIRY_LEDGER" \
  --recovery_address "$CREATOR_ADDRESS" \
  --authorized_controller "$SWEEP_CONTROLLER_ID" \
  --admin "$CREATOR_ADDRESS"

# 5. Fund with real XLM (10 XLM = 100,000,000 stroops).
XLM_SAC="$(stellar contract id asset --asset native --network "$NETWORK")"
AMOUNT_STROOPS=100000000
stellar contract invoke --id "$XLM_SAC" --network "$NETWORK" --source "$PAYER" \
  -- transfer \
  --to "$EA_ID" \
  --amount "$AMOUNT_STROOPS"

# 6. Record the payment in contract state.
stellar contract invoke --id "$EA_ID" --network "$NETWORK" --source "$READER" \
  -- record_payment --amount "$AMOUNT_STROOPS" --asset "$XLM_SAC"

# 7. Verify state and events.
stellar contract invoke --id "$EA_ID" --network "$NETWORK" --source "$READER" \
  --send no -- get_info
stellar events --id "$EA_ID" --start-ledger "$START_LEDGER" \
  --network "$NETWORK" --output json
```

---

## Failure modes

| Symptom | Cause | Action |
|---|---|---|
| `Error(Contract, #1)` on `initialize()` | Instance already initialized — usually the shared registry ID | Deploy a fresh instance; never re-initialize another integrator's account |
| `Error(Contract, #2)` on `record_payment()` | Instance was never initialized | Run `initialize()` first |
| `Error(Contract, #4)` | `amount <= 0` | Use a positive amount in stroops |
| `Error(Contract, #5)` | `expiry_ledger <= current_ledger` | Re-read the latest ledger and add a margin |
| `Error(Contract, #13)` | The asset already has a recorded payment | One payment per asset per instance; use a different asset or a fresh instance |
| `Error(Contract, #14)` | More than 10 assets | Deploy another instance |
| `Error(Auth, InvalidAction)` on `initialize()` | Transaction was not signed by `creator` | Make `--source` the identity that matches `--creator` |
| `balance` is `0` after `record_payment()` | Bookkeeping only; no real transfer happened | Do the SEP-41 `transfer`; do not re-record the payment |
| "contract not found" | Stale or wrong-network ID | Re-verify via `testnet/registry/verify-contract-live.md`; check `network.toml` |
| `txBadSeq` / account not found | Fee payer is unfunded | Fund the identity with Friendbot; see `testnet/faucet/README.md` |

The full error list is in `contracts/ephemeral_account/src/errors.rs` and
`testnet/examples/error-handling.md`.

---

## Related Documentation

- [`README.md`](README.md) — example index and common placeholders
- [`signed-sweep-walkthrough.md`](signed-sweep-walkthrough.md) — the next step:
  sweep the account this example creates
- [`gas-free-claim-walkthrough.md`](gas-free-claim-walkthrough.md) — the
  recipient-signed alternative to a signed sweep
- [`checking-account-status.md`](checking-account-status.md) — read-only gates
  before a state-changing call
- [`multi-asset-sweep.md`](multi-asset-sweep.md) — multiple `record_payment()`
  calls and the `multi_pay` event
- [`batch-account-creation-mock.md`](batch-account-creation-mock.md) — repeating
  the create/initialize steps for several accounts
- [`../registry/contract-status.md`](../registry/contract-status.md) — source
  of the live contract ID used here
- [`../registry/event-topics.md`](../registry/event-topics.md) — topic symbols
  and payload structs
- [`../docs/getting-started.md`](../docs/getting-started.md) — the shorter
  first-call walkthrough, including a JavaScript SDK variant
- [`../config/expiry-ledger-testing.md`](../config/expiry-ledger-testing.md) —
  choosing `expiry_ledger` for fast test iteration
- [`../docs/soroban-cli-quickref.md`](../docs/soroban-cli-quickref.md) — CLI
  flag and command shapes for `stellar` 23.4.1

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-11-25 | Initial create-then-pay worked example with registry-sourced IDs and expected `created`/`payment` event output |
