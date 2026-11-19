# Testnet Troubleshooting Index

Start here when a Bridgelet integration on Stellar testnet fails. This page
routes a symptom to a specific runbook; it does not replace the runbooks and it
does not promise that someone is available to help. Testnet support is
best-effort, with no SLA (see [support](support.md) and the
[on-call expectations](../runbooks/on-call-testnet.md)).

> **Deployment caveat.** The canonical
> [contract status registry](../registry/contract-status.md) currently records
> two TBD and two N/A IDs (EphemeralAccount and SweepController: `TBD`;
> ReserveContract and AccountFactory: `N/A`), all Last Verified `Never`. Older
> deployment artifacts and some docs publish concrete IDs, but they conflict
> with the registry. This index therefore contains no IDs. Verify any ID
> on-chain before using it as evidence in a diagnosis.

## Scope

| In scope | Out of scope |
|---|---|
| Calls against a Bridgelet contract on Stellar testnet | Source-only bugs not yet deployed; use the normal repo issue process |
| Shared-deployment interference, resets, and drift | A guarantee of uptime, response time, or rollback |
| Read-only diagnosis and evidence collection | Publishing secrets or reusable authorization material |
| Best-effort escalation through `[testnet]` issues | Private vulnerability reports filed as public issues |

If the symptom is a possible unauthorized transfer, signature bypass, or state
corruption, use private vulnerability reporting rather than a public issue.

## Quick triage

Work through these in order. Most "contract bug" reports are resolved in steps
1–3.

| Step | Check | Source |
|---|---|---|
| 1 | Confirm the exact passphrase `Test SDF Network ; September 2015`, the RPC URL, and that your client is pointed at testnet, not standalone or mainnet. | [network-config.md](network-config.md) |
| 2 | Confirm what is actually deployed. Treat every stored ID as a claim; run the read-only checks before concluding the contract is broken. | [contract-status.md](../registry/contract-status.md), [verify-contract-live.md](../registry/verify-contract-live.md) |
| 3 | Check for a reset or redeploy: a lower head ledger, IDs that no longer resolve, or a new changelog entry. | [reset-policy.md](../policies/reset-policy.md), [rpc-outage.md §6](../runbooks/rpc-outage.md#6-detect-a-testnet-reset-during-recovery), [changelog.md](changelog.md) |
| 4 | Match the exact error, method, and lifecycle state to the symptom table below. `get_status` returning `Active (0)` on an uninitialized instance proves nothing; use `get_info`. | [getting-started.md#troubleshooting](getting-started.md#troubleshooting), [error-handling.md](../examples/error-handling.md) |
| 5 | Preserve evidence before retrying: contract ID, transaction hash, finalized ledger, exact error, controller nonce where relevant, and UTC time. Never record secret keys or signatures. | [support.md](support.md), [on-call-testnet.md](../runbooks/on-call-testnet.md) |

A timeout is not proof of failure. Reconcile an in-flight transaction by hash
before resubmitting it.

## Symptom to runbook

| Symptom | First checks | Runbook |
|---|---|---|
| `execute_sweep` rejects a signature; `SignatureVerificationFailed`, wrong destination, wrong key, or a locked-destination violation | Fresh `get_nonce`, destination, controller ID, signer key, deployed interface | [failed-sweep-signature.md](../runbooks/failed-sweep-signature.md) |
| Signature fails again after a successful sweep; local nonce differs from `get_nonce`; multiple workers sign from one cache | Compare cached nonce with `get_nonce`; reconcile by transaction hash | [nonce-desync.md](../runbooks/nonce-desync.md) |
| Account seems frozen: payment recorded but sweep or claim fails, expiry looks wrong, or reserve reclaim is stuck | `get_info`, `get_status`, `is_expired`, current ledger, reserve getters, controller `get_nonce` | [stuck-ephemeral-account.md](../runbooks/stuck-ephemeral-account.md) |
| Second `record_payment` for the same asset; `DuplicateAsset`; payment count or overwrite questions | First vs. later payment for that asset; `get_info`, `get_info_paginated` | [multi-payment-edge-cases.md](../runbooks/multi-payment-edge-cases.md) |
| Account expired or recovered but the event amount, reserve bookkeeping, or actual per-asset balances do not reconcile | Pre/post state, `expired` and `reserve` events, per-asset SEP-41 balances | [expired-account-recovery-check.md](../runbooks/expired-account-recovery-check.md) |
| `AccountFactory::batch_initialize` returns partial failures with `error: None` | Per-account status, expiry ledger, WASM hash, salt collisions, batch size | [account-factory-batch-failure.md](../runbooks/account-factory-batch-failure.md) |
| Soroban RPC or Horizon is unavailable, stalls, or stops advancing | `getLatestLedger`, Horizon payment probe, official status page, last durable cursor | [rpc-outage.md](../runbooks/rpc-outage.md) |
| Repeated entry into an account, controller, or token in one transaction; possible reentrancy | Full transaction trace and diagnostic events, deployed code identity, balances | [reentrancy-suspicion.md](../runbooks/reentrancy-suspicion.md) |
| An `EphemeralAccount::upgrade` happened or must be verified; interface or code hash may have changed | Pre/post state, old and candidate WASM hashes, final transaction result | [upgrade-verification.md](../runbooks/upgrade-verification.md) |

### Process runbooks

| Need | Runbook |
|---|---|
| Understand what support and on-call coverage actually exist before escalating | [on-call-testnet.md](../runbooks/on-call-testnet.md) |
| Record a factual incident, timeline, and follow-up actions | [incident-postmortem-template.md](../runbooks/incident-postmortem-template.md) |
| Navigate to a runbook from the runbook directory itself | [runbooks/README.md](../runbooks/README.md) |

## Cross-links

| Topic | Reference |
|---|---|
| Common error codes for first `initialize` and `record_payment` calls | [getting-started.md#troubleshooting](getting-started.md#troubleshooting) |
| Reporting format and rules about secrets | [support.md](support.md) |
| What changed on testnet and when | [changelog.md](changelog.md) |
| Why behavior and IDs can change with little notice | [rate-of-change-expectations.md](../integration/rate-of-change-expectations.md) |
| No cadence, no notice commitment, and the detection procedure | [reset-policy.md](../policies/reset-policy.md) |
| Event names, payloads, and indexer filters | [event-topics.md](../registry/event-topics.md) |
| When an absent event means anything at all | [event-watchlist.md](../monitoring/event-watchlist.md) |
| Paging through event history for audit trails | [pagination-and-history.md](../integration/pagination-and-history.md) |
| Command syntax and real function names | [soroban-cli-quickref.md](soroban-cli-quickref.md) |

## Escalation

After the relevant runbook has been followed:

1. Open a `[testnet]` issue with the `testnet` label, using the template in
   [support.md](support.md). Include the exact error, contract ID, method,
   transaction hash, approximate ledger, and client version.
2. Use **Report a vulnerability** on the repository's Security tab instead of a
   public issue for suspected security defects, even if reproduced only on
   testnet.
3. State what is unknown. Do not fill gaps with guessed IDs, hashes, or nonces,
   and never paste secret keys, signing seeds, or reusable signatures.

No response time is guaranteed. Priority is triaged by impact, not by an SLA.

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-11-18 | Initial symptom-based index across the current testnet runbooks |
| 1.1 | 2026-11-18 | Corrected canonical registry ID wording |
