# Testnet Uptime-Check Specification

## Status

> **Specification only.** This document defines a future read-only health
> check. The repository does not contain a scheduler, monitor, alert
> integration, or uptime history.
>
> Actual monitoring tooling (e.g. scheduled runners, background daemons, or
> external alerting services) is explicitly out of scope for this repository
> and will live under external automation, CI/CD runners, or a dedicated
> monitoring service. This specification defines which contract methods to
> call, expected response shapes, timing thresholds, and cadence so that
> implementation can proceed later against an agreed specification rather than
> ad hoc.

A future check should prove that configured contracts resolve and return
internally consistent state. It must not be confused with a synthetic
create-pay-sweep transaction or an event-absence policy.

## Questions answered by the check

Each cycle should answer:

1. Is the Soroban RPC endpoint reachable, healthy, and advancing ledgers?
2. Does each configured contract ID resolve on Stellar Testnet?
3. Does each deployed contract expose the expected read-only interface?
4. Does the returned state decode and satisfy basic invariants?
5. Is a known account fixture in the lifecycle state recorded for it?
6. Does each probe response arrive within acceptable latency bounds?
7. Is the controller nonce advancing monotonically without regressions?

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

| Setting | Meaning | Default / Target |
|---|---|---|
| `NETWORK` | `testnet` | `testnet` |
| `NETWORK_PASSPHRASE` | `Test SDF Network ; September 2015` | Required constant |
| `RPC_URL` | Primary Soroban RPC endpoint | `https://soroban-testnet.stellar.org` |
| `FALLBACK_RPC_URL` | Optional, independently reviewed fallback | Optional |
| `EA_CONTRACT_ID` | Verified controller-linked account or fixture contract | Strkey (`C...`) |
| `SWEEP_CONTROLLER_ID` | Verified `SweepController` contract | Strkey (`C...`) |
| `RESERVE_CONTRACT_ID` | Verified `ReserveContract` (optional profile) | Strkey (`C...`) |
| `ACCOUNT_FACTORY_ID` | Verified `AccountFactory` (optional profile) | Strkey (`C...`) |
| `READER_IDENTITY` | Funded identity used only for simulations/reads | Dedicated public key |
| `EXPECTED_*_WASM_HASH` | Optional code identity for drift detection | 32-byte hex hash |
| `POLL_INTERVAL_SECONDS` | Core frequency | `300` (5 minutes) |
| `REQUEST_TIMEOUT_SECONDS` | Per-probe deadline | `10` seconds |
| `LATENCY_DEGRADED_MS` | Latency warning threshold | `2000` ms |

Do not store secret keys or auth payloads in monitor configuration. A read-only
simulation should not require a transaction submission or signing secret.

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
as deployed on testnet.

## Read-only transport

Use Soroban RPC `simulateTransaction` or an SDK equivalent that submits no
transaction. Do not call any state-changing method as an uptime probe,
including:

- `initialize`, `record_payment`, `sweep`, `sweep_claim`, `expire`,
  `recover`, `reclaim_reserve`, or `upgrade` on `EphemeralAccount`;
- `initialize`, `execute_sweep`, `claim`, or
  `update_authorized_destination` on `SweepController`;
- `initialize` and `set_base_reserve` on `ReserveContract`; or
- `initialize`, `batch_initialize`, or `batch_initialize_paginated` on
  `AccountFactory`.

CLI examples below are operator references. A worker must use a confirmed
dry-run or simulation path so a version change cannot accidentally submit a
write.

## Probe matrix

### 1. Preflight: Endpoint & ledger liveness

| Target | Method / Endpoint | Arguments | Expected shape | Cadence |
|---|---|---|---|---|
| Soroban RPC | `getHealth` | none | `{"status": "healthy"}` | Every cycle |
| Soroban RPC | `getLatestLedger` | none | `{"sequence": u32, ...}` | Every cycle |

### 2. Core profile: Core contract methods

| Contract | Method | Arguments | Expected shape | Cadence |
|---|---|---|---|---|
| `EphemeralAccount` | `get_info` | none | `AccountInfo` or `NotInitialized` | Every cycle |
| `EphemeralAccount` | `get_status` | none | `0..=3` (Active/PaymentReceived/Swept/Expired) | Every cycle |
| `EphemeralAccount` | `is_expired` | none | Boolean | Every cycle |
| `EphemeralAccount` | `get_payment_count` | none | `0..=10` | Every cycle |
| `SweepController` | `get_nonce` | none | `u64` | Every cycle |
| `SweepController` | `can_sweep` | account address | Boolean | Every cycle when fixture exists |

### 3. Optional profile: Ancillary contracts (when deployed)

| Contract | Method | Arguments | Expected shape | Cadence |
|---|---|---|---|---|
| `ReserveContract` | `has_base_reserve` | none | Boolean | Optional, every 30 minutes |
| `ReserveContract` | `get_base_reserve` | none | `Option<i128>` in stroops | Optional, every 30 minutes |
| `ReserveContract` | `get_admin` | none | `Option<Address>` | Optional, every 30 minutes |
| `AccountFactory` | `batch_initialize_count` | `requests: []` (empty vector) | `u32` (returns `0`) | Optional, every 30 minutes |

The current `known-test-accounts.md` contains placeholders. Until a real
initialized fixture is registered, an `EphemeralAccount` result must be
reported as `fixture_missing`, not silently treated as a healthy active
account.

## Response contracts and expected shapes

### Preflight responses

#### `getHealth`
Expected JSON-RPC response:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "status": "healthy"
  }
}
```
Any status other than `healthy`, HTTP 5xx, or network failure triggers the
transport retry policy.

#### `getLatestLedger`
Expected JSON-RPC response:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "id": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "protocolVersion": 22,
    "sequence": 1234567
  }
}
```
Invariants:
- `sequence` must be strictly positive (`> 0`);
- Across consecutive cycles, `sequence` must monotonically advance (unless
  within the same 5-second ledger window); and
- If `sequence` decreases unexpectedly, trigger `testnet_reset` detection.

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

### `get_status`, `is_expired`, and `get_payment_count`

Current status values are:

| Value | State |
|---:|---|
| `0` | `Active` |
| `1` | `PaymentReceived` |
| `2` | `Swept` |
| `3` | `Expired` |

- `is_expired` is true when the current ledger is greater than or equal to
  `expiry_ledger`. Record the ledger used for the comparison; do not convert
  the decision to wall-clock time.
- `get_status` returns `Active` for an uninitialized contract, and
  `get_payment_count` returns zero. Cross-check both with `get_info` before
  reporting a green fixture.

### `SweepController::get_nonce`

The response is an unsigned 64-bit integer (`u64`). Normalize it as a decimal
string so JavaScript consumers do not lose precision.

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

### Optional `ReserveContract` checks

For an enabled and initialized optional contract:

- `get_base_reserve` returns `Some(stroops)` or `None`;
- `has_base_reserve` agrees with the presence of a stored value;
- `get_admin` returns the configured admin when initialized; and
- the value is within the contract's accepted limit (`0..=10_000_000_000` stroops).

`None` or `false` is valid before configuration. Report it as `not_ready`, not
as a failed network call.

`ReserveContract` reads currently extend its instance TTL. A repeated read
probe can therefore keep the contract active; that is a side effect of the
current source and is not proof of user traffic.

### Optional `AccountFactory` checks

When `AccountFactory` is configured:

- Probe method: `batch_initialize_count(requests: [])`
- Arguments: Empty vector of `AccountInitRequest`
- Expected return: `0` (u32)

Note: `AccountFactory` does not currently expose a public read getter for its
stored `ephemeral_account_wasm_hash`. Simulation of `batch_initialize_count`
with an empty vector verifies that the factory contract ID resolves, is
callable, and executes without requiring caller authentication.

## Example operator read

```bash
# 1. Preflight RPC health
curl -s -X POST https://soroban-testnet.stellar.org \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'

# 2. EphemeralAccount read-only simulation
stellar contract invoke \
  --simulate-only \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  get_info

# 3. SweepController nonce simulation
stellar contract invoke \
  --simulate-only \
  --id <SWEEP_CONTROLLER_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  get_nonce

# 4. SweepController can_sweep simulation
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

## Timing, latency thresholds, and retry policy

### Probe cadence

| Check tier | Schedule | Jitter | Max execution window |
|---|---|---|---|
| Preflight check (RPC Health & Ledger) | Every 300 seconds (5 min) | 0–30 seconds | 5 seconds |
| Core profile (EphemeralAccount + Controller) | Every 300 seconds (5 min) | 0–30 seconds | 25 seconds |
| Optional profile (Reserve + Factory) | Every 1,800 seconds (30 min) | 0–60 seconds | 30 seconds |

### Expected response timing & latency thresholds

A healthy check requires not only a correct shape, but responses within
defined latency bounds:

| Metric | Target / Threshold | Action / Classification |
|---|---|---|
| Nominal latency (P50) | `< 500 ms` | Normal healthy operation |
| Upper nominal latency (P95) | `< 2,000 ms` | Acceptable under normal testnet load |
| Degraded latency threshold | `2,000 ms – 5,000 ms` | Record as `degraded` outcome; warn if persistent |
| Per-probe timeout deadline | `10,000 ms` (10 s) | Abort attempt; record as `timeout` |
| Full cycle maximum duration | `30,000 ms` (30 s) | Abort cycle; record as `cycle_timeout` |

### Retry and backoff policy

- **Transport retries:** Up to 2 retries following an initial transport timeout
  or connection error (3 attempts total).
- **Backoff schedule:** 1 second after 1st failure, 3 seconds after 2nd
  failure.
- **No retry on contract logic errors:** If a call returns a contract trap
  (`Error(Contract, #N)`), method not found, or ABI decode error, do NOT
  retry; record outcome immediately as `contract_error` or
  `interface_mismatch`.
- **Preflight short-circuit:** If `getHealth` or `getLatestLedger` fails all
  retries, abort the cycle immediately with `rpc_unavailable` without
  attempting contract simulations.
- **Consecutive failure threshold:** 3 consecutive cycle failures before
  dispatching a low-urgency alert.

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

1. the preflight checks (`getHealth` and `getLatestLedger`) succeed;
2. the network and reset epoch match configuration;
3. the ledger is advancing;
4. all required contract calls return, decode, and finish within latency
   thresholds;
5. the known initialized account satisfies the `get_info` invariants;
6. `get_status`, `is_expired`, and payment count agree with that state;
7. `get_nonce` does not regress; and
8. a `can_sweep` response, when requested, is internally consistent.

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
- preflights Soroban RPC health and ledger advancement before contract calls;
- records contract ID, method, ledger, latency, decoded result, and error;
- preserves 64-bit and 128-bit numeric precision;
- distinguishes valid defaults from initialization failures;
- handles missing optional contracts and fixtures explicitly;
- detects nonce regression, degraded latency, and testnet resets;
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
- [`alert-thresholds.md`](alert-thresholds.md)
- [`third-party-dependency-status.md`](third-party-dependency-status.md)
