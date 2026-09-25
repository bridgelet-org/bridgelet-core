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

Only request a top-up when the balance is below the 10 XLM trigger or when the
hard floor requires restoration. Do not retry in a tight loop because the
faucet may be rate-limited by account, IP, or network policy.

### 3. Verify the result

After the request:

1. Read the public account balance again.
2. Require an increased balance or record the faucet response and
   `no_balance_change`.
3. Confirm the identity can still act as the intended fee payer.
4. Update the registry's last-checked and last-refunded fields.
5. Restore toward the 50 XLM working target before a presentation.

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
- [`../docs/network-config.md`](../docs/network-config.md)
- [`../config/soroban-cli-identity-setup.md`](../config/soroban-cli-identity-setup.md)
- [`../docs/getting-started.md`](../docs/getting-started.md)
- [`uptime-check-spec.md`](uptime-check-spec.md)
- [`event-watchlist.md`](event-watchlist.md)
