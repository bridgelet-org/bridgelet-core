# Alert False-Positive Log

## Status

> **Template, empty by design.** No automated alerting from
> `testnet/monitoring/alert-thresholds.md` (or any other monitoring document) is
> implemented in this repository, so no alert has fired and there are **no
> entries yet**. This file exists so that when alerts exist, every false positive
> is recorded and thresholds are revisited instead of being set once and left.

## Why keep it

Thresholds proposed in `testnet/monitoring/alert-thresholds.md`,
`balance-thresholds.md` and `uptime-check-spec.md` are starting guesses. Testnet
behaviour (resets, RPC instability, bursty test traffic) makes noise likely.
A log of why alerts fired incorrectly is the input for tuning.

## What counts as a false positive

An alert that fired **but did not indicate a problem the on-call person needed to
act on**. Record it even when the cause is "expected testnet noise". Record
*false negatives* (a real problem with no alert) in the same table with kind `FN`.

## How to record

Add a row at the **top** of the table within one business day of the alert.
Include enough to retune without re-investigating.

| Date (UTC) | Alert / threshold | Kind (FP/FN) | What fired | Why it was wrong | Evidence (links, hashes) | Action | Threshold change | Owner |
|---|---|---|---|---|---|---|---|---|
| _example, remove when first real row is added_ — 2026-01-01 | expiry-to-sweep ratio | FP | Ratio exceeded limit | Batch of intentionally expired test accounts, no real traffic | link to events | none | raised min-sample from 20 to 50 | @handle |

## Review cadence

- On each new entry: check whether the same alert has produced two or more false
  positives in 30 days. If so, propose a threshold or window change.
- Monthly: summarize counts per alert in the weekly summary
  (`testnet/monitoring/weekly-health-summary-template.md`).
- When a threshold changes, also update the threshold document and link this log
  entry from the change.

## Common causes to look for

- Testnet or Soroban RPC reset/outage (`testnet/runbooks/rpc-outage.md`).
- Low sample sizes making ratios swing.
- Deliberate test activity (synthetic transactions, demo accounts) counted as real traffic.
- Event lag making a count look low.
