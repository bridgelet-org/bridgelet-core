# Testnet Balance Thresholds for Long-Lived Accounts

## Status

> **Proposed operating policy.** No automated balance check or Friendbot
> top-up exists in this repository. The known-account registry currently has
> only placeholders, so these values are starting guardrails rather than
> measurements from existing accounts.

This policy applies to long-lived Stellar Testnet `G...` identities used to
deploy, initialize, record payments, submit sweeps, or demonstrate the
contracts. A `C...` contract address is not the Friendbot funding target; fund
the public identity that pays the transaction.

## Units

- Native XLM uses stroops.
- `1 XLM = 10,000,000` stroops.
- Store and compare balances as integer stroops, not floating-point values.
- SEP-41 payment amounts use each asset's own base units.

The `EphemeralAccount` source constant `1,000,000,000` stroops (100 XLM) is
internal reserve bookkeeping, not the minimum balance for a fee-payer account.
Do not copy it into this policy.

## Starting thresholds

| State | Balance | Stroops | Action |
|---|---:|---:|---|
| Hard floor | 5 XLM | `50,000,000` | Stop scheduled writes and demos; re-fund first |
| Re-fund trigger | 10 XLM | `100,000,000` | Request one Friendbot top-up before the next run |
| Working target | 50 XLM | `500,000,000` | Restore toward this target after a top-up |

Decision logic:

```text
balance < 50,000,000
  -> hard_floor_breach

50,000,000 <= balance < 100,000,000
  -> refund_required

balance >= 100,000,000
  -> usable for routine low-volume work

balance < 500,000,000
  -> schedule restoration toward the working target
```

These are conservative operational values, not Stellar protocol minimums. A
read-only identity may use a lower, explicitly approved threshold. Identities
that run batches, relayers, or demonstrations should use the 50 XLM target.

## Account-specific thresholds for proposed long-lived fixtures

The long-lived demo and test accounts proposed in
[`../registry/known-test-accounts.md`](../registry/known-test-accounts.md)
serve distinct operational roles with vastly different transaction consumption
profiles. Applying a single generic threshold risks either false-positive alerts
on static screenshot fixtures or out-of-gas failures on contract deployers.

The table below documents specific minimum funding thresholds and Friendbot
re-funding triggers for each proposed fixture:

| Proposed identity / role | Registry reference | Gas burn profile | Hard floor | Re-fund trigger | Pre-demo target | Estimated capacity |
|---|---|---|---:|---:|---:|---|
| `testnet-known-accounts` (deployer & admin) | Creation procedure (lines 22-25) | High (WASM deploy, instance creation, contract admin init) | 20 XLM (`200,000,000`) | 50 XLM (`500,000,000`) | 200 XLM (`2,000,000,000`) | ~10 contract deployments or ~50 admin invocations |
| `docs-walkthrough-01` (CLI demo operator) | Nomination form & CLI walkthrough | Medium (interactive invocations: `initialize`, `record_payment`) | 5 XLM (`50,000,000`) | 15 XLM (`150,000,000`) | 50 XLM (`500,000,000`) | ~30-40 interactive CLI invocations with congestion spikes |
| `demo-screenshots-01` (UI / screenshot fixture) | Registry table & nomination form | Minimal (read-only queries: `get_status`, `get_info`; rare payment) | 2 XLM (`20,000,000`) | 5 XLM (`50,000,000`) | 20 XLM (`200,000,000`) | Unlimited read-only queries (free); ~10 state updates |
| `sdk-integration-fixture` (SDK test suite fixture) | Registry table (reserved for SDK) | High burst (rapid automated matrix runs: create, fund, sweep, expire) | 15 XLM (`150,000,000`) | 30 XLM (`300,000,000`) | 100 XLM (`1,000,000,000`) | 2-3 complete integration test suite runs |
| `relayer-demo-runner` (relayer & sweep executor) | Operational relayer / controller | Medium-high (cross-contract calls: `execute_sweep`, `claim`, auth checks) | 10 XLM (`100,000,000`) | 25 XLM (`250,000,000`) | 75 XLM (`750,000,000`) | ~20 sweep operations with multi-token transfers |

### Protocol reserve vs. operational gas

A balance above 0 XLM is **not** sufficient to submit transactions on Stellar.
Presenters must account for Stellar's protocol base reserve:

```text
Minimum Account Reserve = (2 + subentries) * 0.5 XLM
```

For an identity holding two SEP-41 trustlines and one signer, the protocol
locks `2.5 XLM` before any transaction can be processed. If an account holds
`2.6 XLM`, attempting a Soroban invocation requiring `0.15 XLM` in resource
fees fails immediately with `tx_insufficient_balance`.

The **Hard Floor** in this table ensures that an account never approaches the
protocol reserve boundary, preserving room for inclusion fees even when
dormant.

## Pre-presentation readiness and headroom calculation

Demos unexpectedly fail mid-presentation when:

1. An account has enough balance for initial steps (e.g., `initialize`), but
   runs out of gas before completing later steps (e.g., `record_payment` or
   `execute_sweep`).
2. Soroban resource fees spike due to network congestion or larger transaction
   footprints.
3. Friendbot rate limits (HTTP 429) prevent an operator from refueling on-the-fly
   during a live demonstration.

### Headroom formula

Before starting any presentation or recording, compute the required safety
balance:

```text
Required Pre-Demo Balance = Hard Floor + (TxCount * P99_Fee * CongestionMultiplier)
```

Where:
- `Hard Floor`: minimum floor for the account role (e.g., 5 XLM for `docs-walkthrough-01`).
- `TxCount`: number of on-chain write transactions planned during the presentation.
- `P99_Fee`: conservative estimate of Soroban invocation cost (~0.05 XLM).
- `CongestionMultiplier`: safety multiplier for testnet surge pricing (recommended: `3x`).

**Worked example for a 10-step interactive CLI walkthrough:**

```text
Hard Floor = 5.0 XLM
Expected Transactions = 10
Estimated Max Fee per Tx = 0.05 XLM
Congestion Multiplier = 3x

Required Pre-Demo Balance = 5.0 + (10 * 0.05 * 3) = 6.5 XLM
```

To maintain a comfortable operational safety margin, the account must be funded
to at least the **Re-fund trigger** (15 XLM) and ideally to the **Pre-demo
target** (50 XLM) before taking the stage.

### Pre-flight verification checklist

Execute this checklist 15 to 30 minutes before any demo or presentation:

1. **Query current balance**:
   ```bash
   stellar keys address docs-walkthrough-01 | xargs -I {} curl -s "https://horizon-testnet.stellar.org/accounts/{}" | jq -r '.balances[] | select(.asset_type=="native") | .balance'
   ```
2. **Compare against threshold**:
   - If balance < **Re-fund trigger** (15 XLM): re-fund via Friendbot immediately.
   - If balance < **Hard floor** (5 XLM): do NOT proceed with the demo until funding is confirmed on-chain.
3. **Confirm balance increase**:
   - Re-query Horizon to verify that testnet confirmed the top-up transaction.
4. **Prepare backup signer**:
   - Have a secondary pre-funded identity registered in `stellar keys` (e.g., `testnet-backup-payer`) as a failover source flag.

## Mid-presentation out-of-gas failure modes and diagnosis

If a transaction fails during a live demo, check the error code to distinguish
an out-of-gas condition from contract logic bugs:

| Error code / signature | Origin | Meaning during demo | Immediate action |
|---|---|---|---|
| `tx_insufficient_balance` | Stellar Core / Horizon | Account balance minus base reserve cannot cover transaction fee bid | Account is out of gas; re-fund fee payer via Friendbot or switch `--source` |
| `tx_insufficient_fee` | Stellar Core / Horizon | Transaction fee bid is below current testnet base fee | Increase `--fee` bid or re-fund fee payer |
| `HostError: Error(Budget, ExceededLimit)` | Soroban host | CPU instruction or memory budget exceeded by contract call | Resource exhaustion; optimize call parameters or increase fee budget |
| `op_underfunded` | Stellar Core | Operation attempted to transfer or lock tokens exceeding available balance | Check token / XLM balance of caller or contract |
| `tx_bad_seq` | Stellar Core | Sequence number mismatch often caused by a previous unconfirmed transaction | Wait for pending tx to clear or resync account sequence |

### Quick diagnostic command

Run this one-liner to inspect whether an account is starved of gas:

```bash
PUBLIC_KEY=$(stellar keys address docs-walkthrough-01)
curl -s "https://horizon-testnet.stellar.org/accounts/$PUBLIC_KEY" | jq '{balances: .balances, subentry_count: .subentry_count}'
```

## Emergency mid-presentation re-funding runbook

If a demo account unexpectedly drops below the hard floor during a rehearsal or
presentation:

### Step 1: Rapid Friendbot request

Trigger an immediate top-up using curl:

```bash
curl -s -X POST "https://friendbot.stellar.org/?addr=$(stellar keys address docs-walkthrough-01)"
```

Or using the Stellar CLI:

```bash
stellar keys fund docs-walkthrough-01 --network testnet
```

### Step 2: Handle Friendbot rate limiting (HTTP 429)

Friendbot enforces rate limits per IP and destination address. If Friendbot
returns `429 Too Many Requests` or fails to disburse:

- **Do NOT retry in a rapid loop.** Retrying repeatedly will extend the cooldown ban.
- Wait at least 15 seconds before retrying.
- If the presentation cannot wait, immediately switch to the backup fee payer.

### Step 3: Presenter failover pattern (backup fee payer)

Presenters should maintain a secondary funded identity (`demo-backup-payer`) so
a depleted demo account does not stall the presentation:

```bash
# If primary docs-walkthrough-01 is out of gas, invoke using backup source:
stellar contract invoke \
  --id <EPHEMERAL_ACCOUNT_ID> \
  --network testnet \
  --source demo-backup-payer \
  -- \
  record_payment \
  --amount 1000000 \
  --asset <TEST_TOKEN_ID>
```

## When to check

Check the public fee-payer balance:

- immediately before a demonstration or integration run;
- hourly while the identity is actively used;
- daily for a dormant but registered fixture;
- after any insufficient-funds or unexpectedly high-fee report; and
- before a scheduled funding request.

A balance check is read-only and must not submit a transaction.

## Friendbot procedure

Friendbot is an operator-triggered funding source. The monitor may recommend a
re-fund, but it must not loop faucet requests automatically.

### 1. Identify and read the account

Use only the public `G...` address recorded in
[`../registry/known-test-accounts.md`](../registry/known-test-accounts.md) or
another approved deployment configuration. Confirm the network is Stellar
Testnet and record:

- public account address;
- UTC check time;
- balance in stroops; and
- reason for the check.

Use Horizon, Soroban RPC account data, or an approved balance-query command.
Do not place a secret key in the command or logs.

### 2. Request one top-up when required

The documented endpoint is:

```text
https://friendbot.stellar.org/?addr=<PUBLIC_G_ADDRESS>
```

An operator may instead use the installed Stellar CLI's supported funding
command. Confirm the exact command syntax with the CLI help before use; do not
copy a funding command from a different CLI major version.

Only request a top-up when the balance is below the account's re-fund trigger or
when the hard floor requires restoration. Do not retry in a tight loop because
the faucet may be rate-limited by account, IP, or network policy.

### 3. Verify the result

After the request:

1. Read the public account balance again.
2. Require an increased balance or record the faucet response and
   `no_balance_change`.
3. Confirm the identity can still act as the intended fee payer.
4. Update the registry's last-checked and last-refunded fields.
5. Restore toward the working target before a presentation.

A successful HTTP response is not proof that the balance increased.

## Registry fields

Each populated long-lived account row should eventually include:

| Field | Meaning |
|---|---|
| Public `G...` account | identity that pays fees |
| Role | deployer, recorder, relayer, watcher, demo operator, recipient |
| Contract/fixture IDs | account or contracts used by the role |
| Threshold policy | approved XLM/stroop values |
| Last balance check | UTC timestamp and balance |
| Last Friendbot attempt | UTC timestamp and result, without credentials |
| Next review | earliest recalibration date |

A contract ID, recorded payment amount, and fee-payer balance are different
values. Never use `Payment.amount` or `SweepCompleted.amount` as a native XLM
account balance.

## Safety rules

- Never automate Friendbot in a loop.
- Never include a secret key, seed phrase, or auth signature in a URL or log.
- Never use a `C...` contract ID as the Friendbot address.
- Do not silently replace a low-balance identity; update every dependent
  deployment reference.
- Do not spend or transfer a known demo identity's funds as part of checking
  its balance.
- A testnet reset can remove an account. Record `account_not_found` as a
  deployment/baseline failure, not as a reason to reuse old credentials.
- Keep enough balance for the next scheduled activity, not only the pending
  transaction.

## Recalibration

Review the thresholds after at least seven days of representative activity or
30 funded transactions. Recalculate using:

- median and high-percentile transaction/resource fees;
- frequency of each role;
- number of retries and failed submissions;
- testnet resets or redeployments; and
- the cost of one emergency top-up before a demonstration.

Raise thresholds when observed burn invalidates the working target. Do not
lower the hard floor merely because a particular run happened to fit.

## Limitations

The repository has no populated known-account list or measured fee-spend
history. Soroban resource fees, rent, account entries, and network conditions
can vary, so a positive balance does not guarantee that a later complex call
will have enough resources.

The current `EphemeralAccount` internal reserve and standalone
`ReserveContract` configuration are separate protocol values. They must not be
treated as demo fee-payer thresholds without a separate design decision.

## Related documents

- [`../registry/known-test-accounts.md`](../registry/known-test-accounts.md)
- [`../policies/demo-account-etiquette.md`](../policies/demo-account-etiquette.md)
- [`../faucet/README.md`](../faucet/README.md)
- [`../docs/network-config.md`](../docs/network-config.md)
- [`../config/soroban-cli-identity-setup.md`](../config/soroban-cli-identity-setup.md)
- [`../docs/getting-started.md`](../docs/getting-started.md)
- [`uptime-check-spec.md`](uptime-check-spec.md)
- [`event-watchlist.md`](event-watchlist.md)
