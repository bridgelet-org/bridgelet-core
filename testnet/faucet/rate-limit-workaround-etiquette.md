# Friendbot Rate Limits: Accepted Workarounds and Etiquette

`testnet/faucet/README.md` documents Friendbot funding and its 429 / 504
behaviours. This page records the **expected pattern** for getting funds when
Friendbot throttles you, so it doesn't become tribal knowledge. Friendbot is a
shared public service run by others; be a good citizen.

> Status: no Friendbot limits are measured here; none of the figures below are
> guaranteed. Follow current guidance from the Stellar documentation.

## First, do the normal things

1. Back off and retry later with exponential delay (respect any `Retry-After`).
2. Fund **once** per account; reuse accounts instead of creating fresh funded ones.
3. Check you actually need a new account; a balance top-up for an existing one
   is often unnecessary.
4. Check status of the upstream service (`testnet/monitoring/third-party-dependency-status.md`).

## Accepted workaround: fund through an intermediary

Instead of calling Friendbot for every account:

1. Maintain **one funded intermediary account** (a testnet-only key; see
   `testnet/security/testnet-vs-mainnet-key-reuse-warning.md`), funded through Friendbot
   once or occasionally.
2. Create or fund new test accounts from the intermediary with a normal Stellar
   `create_account` / payment operation, keeping its balance above the minimum
   reserve (`testnet/runbooks/insufficient-reserve-errors.md`).
3. Top the intermediary up from Friendbot when it runs low, not before each use.

Record intermediary fundings in `testnet/faucet/funding-audit-log.md`.

## Do not

- Rotate IPs, proxies or VPNs to evade Friendbot limits. The earlier README
  mentions waiting 60 seconds; evading throttling by proxy is abusive to a shared
  service and is **not** an accepted practice here.
- Script loops that fund many accounts in a burst.
- Drain accounts to re-fund them (create-and-merge churn), or drain a shared demo
  account (`testnet/policies/demo-account-etiquette.md`).
- Share your intermediary's secret key; share the public address if others should
  be able to request funds.

## Sizing the intermediary

Estimate funds as: `(number of accounts) × (account minimum + fees + working
balance)`. Friendbot's grant has been 10,000 test XLM per account; confirm the
current amount when you fund. A single grant usually covers many test accounts.

## If you need more than that

Ask in the support channel (`testnet/docs/contact-and-ownership.md`) rather than
inventing a larger workaround. If a new pattern proves useful, add it here so it is
documented.
