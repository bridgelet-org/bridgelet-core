# Funding Audit Log for Long-Lived Testnet Accounts

A simple log of when and how much the long-lived demo/test accounts
(`testnet/registry/known-test-accounts.md`) and shared funding accounts (relayer,
intermediary from `testnet/faucet/rate-limit-workaround-etiquette.md`) were
funded. Its purpose is to tell **"ran low from normal test activity"** apart
from **"something is draining this account unexpectedly."**

> **No entries yet.** The long-lived accounts listed in the registry are pending
> deployment, so nothing has been funded or logged. The log starts empty on
> purpose; entries must come from real observed fundings, not estimates.

## How to use it

1. **Record every funding** (Friendbot, intermediary transfer, manual top-up) in
   the "Funding events" table the same day.
2. **Snapshot balances periodically** (for example in the weekly summary,
   `testnet/monitoring/weekly-health-summary-template.md`) in the "Balance
   snapshots" table.
3. **Compare.** Expected balance = last snapshot + fundings − known spend. If the
   actual balance is lower than expected by more than the fees for logged activity,
   investigate.

## Funding events

Add new rows at the top.

| Date (UTC) | Account (public address / label) | Source (Friendbot / intermediary / other) | Amount (XLM) | Tx hash | Reason | Recorded by |
|---|---|---|---|---|---|---|
| _none yet_ | | | | | | |

## Balance snapshots

| Date (UTC) | Account | Balance (XLM) | Source of reading (Horizon URL / tool) | Notes |
|---|---|---|---|---|
| _none yet_ | | | | |

## Reading a balance

```bash
curl -s "https://horizon-testnet.stellar.org/accounts/<G_ADDRESS>" \
  | jq '.balances[] | select(.asset_type=="native") | .balance'
```

## Investigating an unexpected drop

1. List recent operations for the account on Horizon
   (`/accounts/<G>/operations?order=desc&limit=50`) and look for payments out,
   fee spikes, or `account_merge`.
2. Compare with expected fee burn from logged activity (a relayer's balance falls
   with each submitted transaction).
3. Check whether a testnet reset happened (balances return to zero after a wipe;
   see `testnet/policies/reset-policy.md`).
4. If still unexplained, follow `testnet/security/disclosure-process.md` for key
   compromise and `testnet/runbooks/incident-postmortem-template.md` for the write-up.

## Rules

- Public addresses and hashes only. **Never log secret keys.**
- If an account is wiped by a reset, add a final row noting it and start a new
  section for the replacement account.
