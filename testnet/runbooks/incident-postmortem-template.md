# Testnet Incident Postmortem Template

Use this lightweight template for a factual Stellar Testnet incident record.
It supports best-effort maintainer work; it does not imply an SLA or active
on-call coverage.

Keep the first report short. Use `unknown` instead of guessing. Record all
times in UTC.

> If the incident may involve unauthorized transfers, signature bypasses, or
> state corruption, do not publish exploit details in a public issue. Use the
> repository's private vulnerability reporting channel and link it here only
> after the report is private.

## Incident record

- **Title:** `{{short, specific title}}`
- **Status:** `investigating | identified | mitigated | recovered | closed`
- **Severity:** `security | critical | degraded | single-user | unknown`
- **Classification:** `endpoint | contract | deployment/config | automation | data integrity | security | unknown`
- **Reporter:** `{{name or team}}`
- **Investigator:** `{{name or unassigned}}`
- **Started:** `{{YYYY-MM-DDTHH:MM:SSZ}}`
- **Detected:** `{{YYYY-MM-DDTHH:MM:SSZ}}`
- **Mitigated:** `{{timestamp or N/A}}`
- **Recovered:** `{{timestamp or N/A}}`
- **Public issue:** `{{URL or none}}`
- **Private security report:** `{{URL or none}}`
- **Safe to summarize publicly:** `yes | no`

## Summary

_What happened, what was affected, and how it was resolved or contained. Keep
this to one short paragraph._

## Impact

- **Affected endpoints:** `{{RPC / Horizon / both / unknown}}`
- **Affected contracts:** `{{contract IDs or unknown}}`
- **Affected workflows:** `{{payment recording / sweep / claim / expiry / deployment / other}}`
- **Integrator impact:** `{{description}}`
- **On-chain impact:** `{{none known / unknown / hashes, ledgers, state, assets}}`
- **Data integrity:** `{{intact / discrepancy observed / unknown}}`
- **Security relevance:** `{{none known / suspected / confirmed / unknown}}`

## Detection

_How the issue became visible._

- **Detector:** `{{user / maintainer / automation / unknown}}`
- **Initial signal:** `{{exact error, timeout, event, or unexpected state}}`
- **First known affected operation:** `{{UTC time and transaction hash}}`

## Timeline

| Time (UTC) | Event or observation | Evidence |
|---|---|---|
| `{{timestamp}}` | `{{what happened}}` | `{{tx hash, log, status page, issue}}` |
| `{{timestamp}}` | `{{action taken}}` | `{{link or note}}` |
| `{{timestamp}}` | `{{recovery verification}}` | `{{link or note}}` |

## Technical evidence

Record public identifiers and redacted material only.

- Network and exact passphrase:
- Soroban RPC and Horizon URLs:
- Client/tool versions:
- Contract IDs:
- Deployed commit and WASM/code hashes, if verified:
- Transaction hashes and finalized ledgers:
- Exact errors and diagnostic-event excerpts:
- Invocation/auth trace or contract-event evidence:
- Account/controller state before and after:
- Relevant per-asset balances before and after:
- Last Horizon/event cursor and controller nonce:
- Automation queue and pause state:
- Evidence gaps:

Do not include secret keys, seed phrases, unredacted `.env` files, private
authorization material, or an unexpired signed payload.

## Findings

### Observed facts

- `{{fact directly supported by evidence}}`
- `{{fact directly supported by evidence}}`

### Inferences

- `{{inference and the evidence supporting it}}`

### Unknowns

- `{{question that remains unresolved}}`

### Root cause

`{{confirmed cause / most likely cause with uncertainty / unknown}}`

## Response and recovery

### Actions taken

- `{{pause, queue, rollback, redeploy, configuration correction, other}}`
- `{{action deliberately not taken and why}}`

### Recovery verification

- [ ] The intended endpoint is reachable and the ledger is advancing.
- [ ] Network passphrase and contract IDs match the intended deployment.
- [ ] In-flight transactions were reconciled by hash.
- [ ] Account/controller state and relevant balances were checked.
- [ ] Dependent automation was resumed deliberately, not by bulk replay.
- [ ] Registry or deployment documentation reflects verified reality.

## What helped and what blocked recovery

- **What helped:**
- **What delayed diagnosis or recovery:**
- **Missing evidence or tooling:**

Use factual, blameless language.

## Follow-up actions

| Action | Owner | Due date | Tracking | Status |
|---|---|---|---|---|
| `{{specific action}}` | `{{owner or unassigned}}` | `{{date or none}}` | `{{issue/PR}}` | `open` |

An explicitly unassigned action is better than inventing an on-call owner.

## Closure checklist

- [ ] Impact and timeline are sufficient for another maintainer to follow.
- [ ] Root cause is confirmed or explicitly left unknown.
- [ ] Evidence and private-report locations are recorded.
- [ ] Follow-up items have owners or are explicitly unassigned.
- [ ] Public documentation does not claim monitoring or response times that
      do not exist.
- [ ] No credentials or sensitive payloads are included.

## Related documents

- [`on-call-testnet.md`](on-call-testnet.md)
- [`rpc-outage.md`](rpc-outage.md)
- [`reentrancy-suspicion.md`](reentrancy-suspicion.md)
- [`../docs/support.md`](../docs/support.md)
