# Weekly Testnet Health Summary (Manual Template)

A lightweight interim measure until any of the automated monitoring in
`testnet/monitoring/` is built. One person fills this in per week (suggested:
same weekday), commits a copy under a dated name such as
`testnet/monitoring/summaries/2026-W42.md` if the team wants history, or posts it
in the support channel (`testnet/docs/contact-and-ownership.md`).

> No automation produces these numbers today. Gather them manually from Soroban
> RPC events, `get_*` read calls and the registry, and say which you could not
> obtain rather than guessing.

---

## Summary for week of YYYY-MM-DD (UTC) — filled by: @handle

**Overall:** ☐ Healthy ☐ Degraded ☐ Down ☐ Unknown

### 1. Sweep success rate

| Metric | Value | How measured |
|---|---|---|
| Sweep attempts | N | submitted `execute_sweep` / `claim` transactions |
| Successful sweeps | N | `sweep` events (controller) with matching `swept_mul` |
| Failed attempts | N | rejected/failed transactions; list the top error codes |
| Success rate | N / N = %  | successes ÷ attempts |
| Expired accounts | N | `expired` events |
| Expiry-to-sweep ratio | E / S | compare with `alert-thresholds.md` |

If there is no traffic, write "no activity" and do not report a percentage.

### 2. Incidents

| Date | Summary | Runbook/postmortem | Status |
|---|---|---|---|
| | | `testnet/runbooks/incident-postmortem-template.md` | |

"None this week" is a valid entry.

### 3. Registry freshness

| Item | Value |
|---|---|
| Last verification date (`testnet/registry/last-verified.json`) | |
| Days since last verification | |
| Contract IDs match live state? | ☐ Yes ☐ No ☐ Not checked |
| Deployment drift noticed (changelog entry present?) | |

Follow `testnet/registry/verify-contract-live.md` to refresh.

### 4. Dependencies and funding

| Item | Value |
|---|---|
| Soroban RPC / Horizon / Friendbot status | see `testnet/monitoring/third-party-dependency-status.md` if present |
| Relayer / demo account balances | see `testnet/monitoring/balance-thresholds.md` |

### 5. Alerts and false positives

Counts of alerts fired, and new rows in `testnet/monitoring/false-positive-log.md`.

### 6. Actions for next week

- [ ]
- [ ]

---

## Filling guidance

- Use UTC and the Monday–Sunday week.
- Cite how each number was obtained; "unknown" beats an unverified figure.
- Keep it to one page. Link to details instead of pasting logs.
- Never include secret keys or signed payloads.
