# Testnet Incident Postmortem Template

Use this lightweight template for a factual Stellar Testnet incident record.
It supports best-effort maintainer work; it does not imply an SLA or active
on-call coverage.

Keep the first report short. Use `unknown` instead of guessing. Record all
times in UTC. When an incident is resolved, use this template to record what
happened, analyze root causes, assign follow-ups, and feed lessons back into
[`testnet/docs/known-limitations.md`](../docs/known-limitations.md) and
[`testnet/registry/`](../registry/README.md).

> If the incident may involve unauthorized transfers, signature bypasses, or
> state corruption, do not publish exploit details in a public issue. Use the
> repository's private vulnerability reporting channel and link it here only
> after the report is private.

## Incident record

- **Title:** `{{short, specific title}}`
- **Status:** `investigating | identified | mitigated | recovered | closed`
- **Severity:** `security | critical | degraded | single-user | unknown`
- **Classification:** `endpoint | contract | deployment/config | automation | data integrity | security | unknown`
- **Runbook(s) referenced:** `{{testnet/runbooks/<runbook-name>.md | none}}`
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

## Detection and triage

_How the issue became visible and which diagnostic path was chosen._

- **Detector:** `{{user / maintainer / automation / unknown}}`
- **Initial signal:** `{{exact error, timeout, event, or unexpected state}}`
- **First known affected operation:** `{{UTC time and transaction hash}}`
- **Initial runbook invoked:** `{{testnet/runbooks/<runbook-name>.md | ad-hoc triage}}`

## Timeline

| Time (UTC) | Event or observation | Evidence |
|---|---|---|
| `{{timestamp}}` | `{{what happened}}` | `{{tx hash, log, status page, issue}}` |
| `{{timestamp}}` | `{{action taken / runbook procedure executed}}` | `{{link or note}}` |
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

## Lessons learned and feedback loop

Use this section to feed incident findings back into project documentation, registries, and procedures.

### Feedback for `testnet/docs/known-limitations.md`

Does this incident reveal an MVP contract stub, unverified path, reserve nuance, or operational gap that integrators should know about?

- [ ] **No update needed** (`{{reason: e.g., routine transient RPC outage or client-side operator error}}`)
- [ ] **Update required**
  - **Identified limitation:** `{{describe the quirk, stub, or operational behavior}}`
  - **Target section:** `{{Stubs and partial implementations | Operational limitations}}`
  - **Draft text / table entry:** `{{concise entry explaining what happens and why it matters}}`
  - **Tracking PR/Issue:** `{{URL or #issue}}`

### Feedback for `testnet/registry/`

Did this incident reveal stale registry data, require a redeployment or upgrade, alter contract IDs, or change admin addresses?

- [ ] **No update needed** (`{{reason: on-chain deployment and registry entries remain identical}}`)
- [ ] **Update required**
  - **Files to update:** `{{testnet/registry/contract-status.md | last-verified.json | account-factory-status.md | reserve-contract-status.md | admin-addresses.md | wasm-hash-reference.md | other}}`
  - **Required adjustments:** `{{new contract IDs, WASM hashes, status flags, caveats, or verified timestamps}}`
  - **Verification procedure:** Re-run [`verify-contract-live.md`](../registry/verify-contract-live.md) and update [`last-verified.json`](../registry/last-verified.json).
  - **Tracking PR/Issue:** `{{URL or #issue}}`

### Feedback for `testnet/runbooks/`

Did the runbook used provide clear, correct steps? Were diagnostics missing or misleading?

- **Runbook used:** `{{path to runbook or none}}`
- **Runbook effectiveness:** `effective | missing steps | incorrect commands | not applicable`
- **Needed improvements:** `{{specific additions, e.g. new error codes to diagnose, improved CLI syntax}}`
- **New runbook needed:** `{{yes / no; specify scenario if new runbook should be created}}`
- **Tracking PR/Issue:** `{{URL or #issue}}`

## What helped and what blocked recovery

- **What helped:** `{{tools, documentation, runbooks, logs that expedited resolution}}`
- **What delayed diagnosis or recovery:** `{{unclear errors, missing logs, misleading docs}}`
- **Missing evidence or tooling:** `{{capabilities or CLI helpers needed for next time}}`

Use factual, blameless language.

## Follow-up actions

| Action | Category | Owner | Due date | Tracking | Status |
|---|---|---|---|---|---|
| `{{specific action}}` | `limitations / registry / runbook / bugfix / tooling` | `{{owner or unassigned}}` | `{{date or none}}` | `{{issue/PR}}` | `open` |

An explicitly unassigned action is better than inventing an on-call owner.

## Closure checklist

- [ ] Impact and timeline are sufficient for another maintainer to follow.
- [ ] Root cause is confirmed or explicitly left unknown.
- [ ] Evidence and private-report locations are recorded.
- [ ] Feedback for [`testnet/docs/known-limitations.md`](../docs/known-limitations.md) evaluated and tracked (or explicitly marked not needed).
- [ ] Feedback for [`testnet/registry/`](../registry/README.md) evaluated and tracked (or explicitly marked not needed).
- [ ] Runbook effectiveness reviewed and runbook update tracked if needed.
- [ ] Follow-up items have owners or are explicitly unassigned.
- [ ] Public documentation does not claim monitoring or response times that do not exist.
- [ ] No credentials or sensitive payloads are included.

## Related documents

- [`on-call-testnet.md`](on-call-testnet.md)
- [`rpc-outage.md`](rpc-outage.md)
- [`reentrancy-suspicion.md`](reentrancy-suspicion.md)
- [`../docs/known-limitations.md`](../docs/known-limitations.md)
- [`../docs/support.md`](../docs/support.md)
- [`../registry/contract-status.md`](../registry/contract-status.md)
- [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md)
