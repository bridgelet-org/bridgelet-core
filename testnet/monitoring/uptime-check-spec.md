# Testnet Uptime-Check Specification

## Status

> **Specification only.** This document defines a future read-only health
> check. The repository does not contain a scheduler, monitor, alert
> integration, or uptime history.

A future check should prove that configured contracts resolve and return
internally consistent state. It must not be confused with a synthetic
create-pay-sweep transaction or an event-absence policy.

## Questions answered by the check

Each cycle should answer:

1. Does the configured contract ID resolve on Stellar Testnet?
2. Does the deployed contract expose the expected read interface?
3. Does the returned state decode and satisfy basic invariants?
4. Is a known account fixture in the lifecycle state recorded for it?
5. Is the Soroban ledger advancing and the collector current?

The check does not prove that a token transfer, signature, relayer, payment
watcher, or sweep authorization is correct.

## Source and deployment preconditions

The proposed interface is based on the checked-in contract source. Before a
monitor is enabled:

- reconcile `testnet/registry/contract-status.md` with deployment artifacts;
- verify every contract ID through a read-only call;
- record the deployed WASM/code identity and interface;
- populate at least one known initialized `EphemeralAccount` fixture in
  [`../registry/known-test-accounts.md`](../registry/known-test-accounts.md);
- define which contracts are required versus optional; and
- identify the current Stellar testnet reset epoch.

A stored ID is configuration, not proof of liveness. A method-not-found or ABI
decode error is an interface/version signal until the deployed identity is
checked.

## Required configuration

| Setting | Meaning |
|---|---|
| `NETWORK` | `testnet` |
| `NETWORK_PASSPHRASE` | `Test SDF Network ; September 2015` |
| `RPC_URL` | Primary Soroban RPC endpoint |
| `FALLBACK_RPC_URL` | Optional, independently reviewed fallback |
| `EA_CONTRACT_ID` | Verified controller-linked account or fixture contract |
| `SWEEP_CONTROLLER_ID` | Verified `SweepController` contract |
| `READER_IDENTITY` | Funded identity used only for simulations/reads |
| `EXPECTED_*_WASM_HASH` | Optional code identity for drift detection |
| `POLL_INTERVAL_SECONDS` | Core frequency |
| `REQUEST_TIMEOUT_SECONDS` | Per-probe deadline |

Do not store secret keys or auth payloads in monitor configuration. A read-only
simulation should not require a transaction submission.

## Contract profiles

### Core profile

- `EphemeralAccount`
- `SweepController`

Enable the core profile only after both IDs and a known initialized account
fixture are verified.

### Optional profile

- `ReserveContract`
- `AccountFactory`

A missing optional contract is `not_configured`, not automatically a public
network outage. The current registry does not verify either optional contract
as deployed.

## Read-only transport

Use Soroban RPC `simulateTransaction` or an SDK equivalent that submits no
transaction. Do not call any state-changing method as an uptime probe,
including:

- `initialize`, `record_payment`, `sweep`, `sweep_claim`, `expire`,
  `recover`, `reclaim_reserve`, or `upgrade` on `EphemeralAccount`;
- `initialize`, `execute_sweep`, `claim`, or
  `update_authorized_destination` on `SweepController`; or
- `initialize` and `set_base_reserve` on `ReserveContract`.

CLI examples below are operator references. A worker must use a confirmed
dry-run or simulation path so a version change cannot accidentally submit a
write.

## Probe matrix

| Contract | Method | Arguments | Expected shape | Core frequency |
|---|---|---|---|---|
| `EphemeralAccount` | `get_info` | none | `AccountInfo` or `NotInitialized` | Every cycle |
| `EphemeralAccount` | `get_status` | none | `0..=3` | Every cycle |
| `EphemeralAccount` | `is_expired` | none | Boolean | Every cycle |
| `EphemeralAccount` | `get_payment_count` | none | `0..=10` | Every cycle |
| `SweepController` | `get_nonce` | none | `u64` | Every cycle |
| `SweepController` | `can_sweep` | account address | Boolean | Every cycle when fixture exists |
| `ReserveContract` | `has_base_reserve` | none | Boolean | Optional, every 30 minutes |
| `ReserveContract` | `get_base_reserve` | none | `Option<i128>` in stroops | Optional, every 30 minutes |
| `ReserveContract` | `get_admin` | none | `Option<Address>` | Optional, every 30 minutes |

The current `known-test-accounts.md` contains placeholders. Until a real
initialized fixture is registered, an `EphemeralAccount` result must be
reported as `fixture_missing`, not silently treated as a healthy active
account.

## Response contracts

### `EphemeralAccount::get_info`

A successful initialized response has this logical shape:

```json
{
  "creator": "<ADDRESS>",
  "status": 0,
  "expiry_ledger": 0,
  "recovery_address": "<ADDRESS>",
  "payment_received": false,
  "payment_count": 0,
  "payments": [],
  "swept_to": null
}
```

Required invariants:

- `status` is `0`, `1`, `2`, or `3`;
- `payment_count` is between 0 and 10;
- `payment_count` equals the number of returned payment records;
- `payment_received` equals `payment_count > 0`;
- payment assets are unique and amounts are positive;
- amounts remain in the asset's base units; and
- `swept_to` is absent or a valid address.

`get_info` must fail with `NotInitialized` for an uninitialized account. That
error is not an initialized-account health pass.

### `get_status`, `is_expired`, and payment count

Current status values are:

| Value | State |
|---:|---|
| `0` | `Active` |
| `1` | `PaymentReceived` |
| `2` | `Swept` |
| `3` | `Expired` |

`is_expired` is true when the current ledger is greater than or equal to
`expiry_ledger`. Record the ledger used for the comparison; do not convert the
decision to wall-clock time.

`get_status` returns `Active` for an uninitialized contract, and
`get_payment_count` returns zero. Cross-check both with `get_info` before
reporting a green fixture.

### `SweepController::get_nonce`

The response is an unsigned 64-bit integer. Normalize it as a decimal string so
JavaScript consumers do not lose precision.

- Zero is a valid newly initialized value.
- A missing storage key also defaults to zero, so a zero response alone does
  not prove initialization.
- For the same contract and reset epoch, the value must not decrease.

### `SweepController::can_sweep`

A Boolean response is valid in either state. Current source returns `true` only
when the account has a recorded payment, status is `PaymentReceived`, and the
account is not expired. Cross-check a `true` result with the account's
`get_info` and `is_expired` values.

A `false` result is not automatically an outage: a new, already swept, expired,
or uninitialized fixture can legitimately return false.

## Optional ReserveContract checks

For an enabled and initialized optional contract:

- `get_base_reserve` returns `Some(stroops)` or `None`;
- `has_base_reserve` agrees with the presence of a stored value;
- `get_admin` returns the configured admin when initialized; and
- the value is within the contract's accepted limit.

`None` or `false` is valid before configuration. Report it as `not_ready`, not
as a failed network call.

`ReserveContract` reads currently extend its instance TTL. A repeated read
probe can therefore keep the contract active; that is a side effect of the
current source and is not proof of user traffic.

## Example operator read

```bash
stellar contract invoke \
  --simulate-only \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  get_info

stellar contract invoke \
  --simulate-only \
  --id <SWEEP_CONTROLLER_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  get_nonce

stellar contract invoke \
  --simulate-only \
  --id <SWEEP_CONTROLLER_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  can_sweep \
  --ephemeral_account <EA_CONTRACT_ID>
```

A monitor should call the equivalent RPC simulation and store structured
results rather than parse human-formatted CLI text.

## Timing and retry policy

Initial proposal, subject to a measured baseline:

| Setting | Value |
|---|---:|
| Core cycle | Every 300 seconds, with 0-30 seconds jitter |
| Optional cycle | Every 1,800 seconds |
| Per-attempt timeout | 10 seconds |
| Retries | Two after the initial attempt |
| Backoff | 1 second, then 3 seconds |
| Maximum core-cycle duration | 30 seconds |
| Consecutive-failure threshold | Three cycles before a low-urgency alert |

Do not overlap retries for the same contract. A retry is a transport attempt,
not a new state sample, until one result is accepted and recorded.

Stellar testnet ledgers close roughly every five seconds, but the ledger
sequence returned by the RPC is authoritative. Associate every probe with the
latest ledger observed in that cycle.

## Result schema

```json
{
  "check_id": "<stable-id>",
  "network": "testnet",
  "reset_epoch": "<epoch-id>",
  "contract_id": "<CONTRACT_ID>",
  "method": "get_info",
  "arguments": {},
  "started_at": "<UTC_TIMESTAMP>",
  "completed_at": "<UTC_TIMESTAMP>",
  "duration_ms": 0,
  "rpc_ledger": 0,
  "outcome": "pass",
  "result": {},
  "error": null
}
```

Controlled outcomes:

- `pass`
- `fail`
- `degraded`
- `not_configured`
- `fixture_missing`
- `timeout`
- `rpc_error`
- `contract_not_found`
- `interface_mismatch`
- `decode_error`
- `unexpected_value`
- `nonce_regression`
- `testnet_reset`

Store `u64` and `i128` values as decimal strings. Do not store credentials,
auth signatures, or unrelated transaction payloads in a probe record.

## Pass criteria

A core cycle passes only when:

1. the network and reset epoch match configuration;
2. the ledger is advancing;
3. all required contract calls return and decode;
4. the known initialized account satisfies the `get_info` invariants;
5. `get_status`, `is_expired`, and payment count agree with that state;
6. `get_nonce` does not regress; and
7. a `can_sweep` response, when requested, is internally consistent.

Do not report a pass solely because a safe default such as `Active`, zero, or
false was returned.

## Testnet reset handling

A lower ledger sequence, missing contract, or changed code identity can indicate
a reset or redeployment. On detection:

1. stop using the old baseline;
2. create a new reset epoch;
3. quarantine old contract IDs and cursor history;
4. require deployment-record reconciliation;
5. verify new IDs, interfaces, and fixtures; and
6. resume only after preflight passes.

Never compare a nonce, fixture state, or event rate across reset epochs without
segmenting it.

## Acceptance criteria for a future implementation

The implementation is ready when it can demonstrate that it:

- uses simulation and never submits a state-changing probe;
- records contract ID, method, ledger, latency, decoded result, and error;
- preserves 64-bit and 128-bit numeric precision;
- distinguishes valid defaults from initialization failures;
- handles missing optional contracts and fixtures explicitly;
- detects nonce regression and testnet resets;
- obeys the documented frequencies and bounded timeouts; and
- survives deployment ID and interface changes without a false green state.

Until those conditions are implemented and exercised, the status is:

```text
testnet_uptime_check: not_configured
```

## Related documents

- [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md)
- [`../registry/known-test-accounts.md`](../registry/known-test-accounts.md)
- [`../registry/contract-status.md`](../registry/contract-status.md)
- [`balance-thresholds.md`](balance-thresholds.md)
- [`event-watchlist.md`](event-watchlist.md)
