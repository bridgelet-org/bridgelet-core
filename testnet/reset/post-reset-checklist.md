# Post-Redeploy Documentation Checklist

## Status

> **No Bridgelet contract is currently deployed.** The canonical registry,
> `testnet/registry/contract-status.md`, records all four as undeployed with
> two `TBD` and two `N/A` IDs and Last Verified `Never`. This checklist is
> written for the next redeploy. It does not assert that any ID or hash named
> in this tree is live.

Running `scripts/deploy-testnet.sh` changes every contract ID, destroys all
contract state, and makes a large part of `testnet/` wrong at the same moment.
This is the checklist for fixing that documentation in one change. It is a
sibling of [`README.md`](README.md) (the procedure) and
[`communication-template.md`](communication-template.md) (the message).

## How to use this checklist

Every group below is a table with **Owner**, **Date**, and **Status** columns.
Owner starts as `Unassigned` because no deployment authority is named in this
repository — see [`../policies/reset-policy.md`](../policies/reset-policy.md)
§ Authority. Date is the UTC date the row was actually completed. Status is
one of:

| Status | Meaning |
|---|---|
| `Pending` | Not started. The starting value for every row. |
| `In progress` | Someone is editing it. |
| `Verified` | Committed, and checked against the live deployment. |
| `N/A` | Reviewed; no change needed this time. Record the reason in the notes. |

Do not fill a row from the deploy script's own output. Output is a claim, not
a verification. The gate below comes first.

## Verification gate

Before any new ID, WASM hash, admin address, or "deployed" statement is written
into a document, resolve it with
[`../registry/verify-contract-live.md`](../registry/verify-contract-live.md).
A read-only invocation is enough to prove an ID resolves; `get_info` on an
`EphemeralAccount` proves initialization where `get_status` does not, because
the latter returns `Active` (`0`) for an uninitialized instance.

```bash
export NETWORK=testnet
export RPC_URL="https://soroban-testnet.stellar.org"
export READER="<funded-testnet-identity>"

# Resolve each contract ID before recording it anywhere.
stellar contract invoke --send no \
  --id "<stored-sweep-controller-id>" --network "$NETWORK" \
  --source "$READER" -- get_nonce

stellar contract invoke --send no \
  --id "<stored-ephemeral-account-id>" --network "$NETWORK" \
  --source "$READER" -- get_info
```

Also apply the three pre-flight steps in `reset-policy.md`: re-verify the IDs,
check the staleness timestamp, and check whether the network itself moved. The
latest-ledger check is in
[`../runbooks/rpc-outage.md`](../runbooks/rpc-outage.md) §6. Two caveats that
matter here:

- `testnet/registry/last-verified.json` is a staleness mechanism, **not** proof
  of liveness. All four timestamps are recent while every contract is recorded
  as undeployed.
- The native XLM Stellar Asset Contract ID is derived from the testnet
  passphrase and does **not** change on a redeploy. Do not replace it along
  with the Bridgelet IDs.

## 1. Registry

`testnet/registry/` is the integrator-facing mirror of "what is live". Treat
`contract-status.md` as canonical; where it and older deployment artifacts
disagree, the registry is the one to reconcile first.

| Document | Update required | Verify | Owner | Date | Status |
|---|---|---|---|---|---|
| `registry/contract-status.md` | Flip status/IDs/WASM hashes/Last Verified; clear per-contract blockers | Full `verify-contract-live.md` pass | Unassigned | — | Pending |
| `registry/wasm-hash-reference.md` | Populate Current WASM Hashes, Deploy Date/By, and Commit-to-Hash mapping | Compare recorded hash with on-chain code | Unassigned | — | Pending |
| `registry/upgrade-history.md` | For a fresh redeploy, confirm no in-place upgrade row is needed; reset the "Initial deployment" placeholders. Log any `upgrade()` that accompanied the change | Transaction history for `upgrade` invocations | Unassigned | — | Pending |
| `registry/last-verified.json` | Refresh all four ISO timestamps **after** verification, not at deploy time | Cross-check against `contract-status.md` | Unassigned | — | Pending |
| `registry/admin-addresses.md` | Re-query public creator, controller, admin, signer and destination addresses | `get_info` / `get_admin`; infer the rest from init txs | Unassigned | — | Pending |
| `registry/known-test-accounts.md` | Recreate the long-lived demo accounts under new IDs; old instances are gone | `get_info` on each replacement | Unassigned | — | Pending |
| `registry/event-topics.md` | Update contract-ID references; confirm event names/payloads are unchanged and revise the deployment-status table | Compare against source and observed events | Unassigned | — | Pending |
| `registry/reserve-contract-status.md` | If included in the deploy, flip status and record admin/base reserve; otherwise confirm still not deployed | `get_admin`, `get_base_reserve`, `has_base_reserve` | Unassigned | — | Pending |
| `registry/account-factory-status.md` | If included in the deploy, flip status and record the stored WASM hash; otherwise confirm still not deployed | `batch_initialize_count` dry-run | Unassigned | — | Pending |
| `registry/verify-contract-live.md` | Correct "Expected deployed: Yes" headings that contradict the canonical registry | Re-read each expected-value table | Unassigned | — | Pending |
| `registry/dependency-graph.md` | Update the live call graph if wiring or initialized roles changed | Trace initialize arguments | Unassigned | — | Pending |
| `registry/README.md` | Review the trigger/update table and the sample `last-verified.json`; edit only if the process changed | Read-through | Unassigned | — | Pending |

## 2. Config

`testnet/config/` is integrator-facing, non-secret consumption config. Keep the
no-secrets rule: no `S...` key or Ed25519 seed belongs in any of these files.

| Document | Update required | Verify | Owner | Date | Status |
|---|---|---|---|---|---|
| `config/README.md` | Confirm the "where contract IDs come from" paths still describe the deploy output; update names if the artifact layout changed | Compare with `deployments/testnet.json` and `deployment-artifacts/contract-ids.txt` | Unassigned | — | Pending |
| `config/soroban-cli-identity-setup.md` | Replace any concrete IDs/WASM hashes in the examples with verified values; note that `initialize` argument counts may have changed | Dry-run each command against the new IDs | Unassigned | — | Pending |
| `config/nonce-tracking.md` | Confirm the nonce restart at `0` is stated; replace concrete controller IDs in examples | `get_nonce` on the new controller | Unassigned | — | Pending |
| `config/expiry-ledger-testing.md` | Review for embedded IDs/hashes and current-ledger arithmetic | Read-through + fresh ledger query | Unassigned | — | Pending |
| `config/gas-free-claim-testing.md` | Review for embedded IDs/hashes and the current `claim()` interface | Simulate once | Unassigned | — | Pending |
| `config/Refund-cancellation.md` | Review for embedded IDs/hashes and role assumptions | Read-through | Unassigned | — | Pending |

## 3. Docs

These contain the highest concentration of copied IDs: `network-config.md`,
`getting-started.md`, and the dated `changelog.md` entry.

| Document | Update required | Verify | Owner | Date | Status |
|---|---|---|---|---|---|
| `docs/changelog.md` | Add the new dated entry at the top with all five required fields plus the deployed commit; never rewrite a historical entry | Compare with the deployed commit SHA | Unassigned | — | Pending |
| `docs/network-config.md` | Update the `.env` snippet, "Deployed contract IDs" table, WASM hash row, and the stale `null`-placeholders note | Every ID resolved per the gate | Unassigned | — | Pending |
| `docs/getting-started.md` | Update the shared instance ID, `EPHEMERAL_WASM_HASH` in both CLI and JS snippets, and the SweepController ID | Run the read-only parts of both walkthroughs | Unassigned | — | Pending |
| `docs/faq.md` | Reconcile "which contracts can I call", ID locations, and reset/stability answers with the new deployment | Read-through against the registry | Unassigned | — | Pending |
| `docs/support.md` | Confirm the triage steps still point at the right files; usually no ID change | Read-through | Unassigned | — | Pending |
| `docs/overview.md` | Update the deployed-by/initialized-by table, trust-level table, and the `741aec2` historical claim if the script's scope changed | Compare with `scripts/deploy-testnet.sh` scope | Unassigned | — | Pending |
| `docs/soroban-cli-quickref.md` | Replace any concrete contract IDs in snippets; confirm method names | Simulate one command | Unassigned | — | Pending |
| `docs/contact-and-ownership.md` | Update contacts only if the deploy owner/role changed; do not invent an owner | Confirm with the contacts themselves | Unassigned | — | Pending |

`docs/api-reference.md` is explicitly out of scope for this documentation set
and is not edited by this checklist.

## 4. Examples

Most examples use named placeholders and need only a read-through. The two
files with concrete IDs are `examples/README.md` (the native XLM SAC, which is
stable, and the WASM-hash placeholder) and `examples/multi-asset-sweep.md`.

| Document | Update required | Verify | Owner | Date | Status |
|---|---|---|---|---|---|
| `examples/README.md` | Reconfirm the placeholder preamble and `EPHEMERAL_ACCOUNT_WASM_HASH`; leave the native XLM SAC ID alone | Read-through + hash check | Unassigned | — | Pending |
| `examples/multi-asset-sweep.md` | Review the concrete asset address and any Bridgelet IDs | Simulate a read-only call | Unassigned | — | Pending |
| `examples/signed-sweep-walkthrough.md` | Review placeholders, nonce sequencing, and destination assumptions | Read-through | Unassigned | — | Pending |
| `examples/integration-test-harness-notes.md` | Confirm "deploy your own instance per run" still holds; update any stored IDs | Read-through | Unassigned | — | Pending |
| `examples/idempotency-considerations.md` | Confirm nonce-reset guidance after redeploy | Read-through | Unassigned | — | Pending |
| `examples/gas-free-claim-walkthrough.md` | Review IDs and relayer/recipient roles | Read-through | Unassigned | — | Pending |
| `examples/expiry-and-recovery.md` | Review IDs and expiry arithmetic | Read-through | Unassigned | — | Pending |
| `examples/error-handling.md` | Confirm discriminant table against source, not the changelog | Compare with `contracts/*/src/errors.rs` | Unassigned | — | Pending |
| `examples/checking-account-status.md` | Review IDs and status values | Read-through | Unassigned | — | Pending |
| `examples/batch-account-creation-mock.md` | Review factory availability and WASM hash assumptions | Read-through | Unassigned | — | Pending |
| `examples/reading-contract-storage.md` | Review IDs and storage keys | Read-through | Unassigned | — | Pending |
| `examples/postman-collection.json` | Replace concrete contract IDs in saved requests | Re-run a saved read-only request | Unassigned | — | Pending |

## 5. Runbooks

Runbooks usually need a scope check rather than an ID rewrite. Update them if
a procedure's expected state (nonce, status, destination) changed.

| Document | Update required | Verify | Owner | Date | Status |
|---|---|---|---|---|---|
| `runbooks/README.md` | Confirm the common-prerequisites pointer to the registry is still correct | Read-through | Unassigned | — | Pending |
| `runbooks/nonce-desync.md` | Confirm guidance covers a nonce that restarted at `0` after redeploy | Compare with `get_nonce` | Unassigned | — | Pending |
| `runbooks/failed-sweep-signature.md` | Reconfirm nonce and signer diagnostics against the new controller | Simulate `get_nonce` | Unassigned | — | Pending |
| `runbooks/stuck-ephemeral-account.md` | Reconfirm expected terminal states | Read-through | Unassigned | — | Pending |
| `runbooks/account-factory-batch-failure.md` | Confirm factory availability and index-salt caveat | Read-through | Unassigned | — | Pending |
| `runbooks/multi-payment-edge-cases.md` | Review IDs and slot limits | Read-through | Unassigned | — | Pending |
| `runbooks/upgrade-verification.md` | Confirm the redeploy-vs-upgrade distinction is explicit | Read-through | Unassigned | — | Pending |
| `runbooks/rpc-outage.md` | Confirm the reset-detection section still matches observed signals | Read-through | Unassigned | — | Pending |
| `runbooks/on-call-testnet.md` | Update escalation contacts only if ownership changed | Confirm with contacts | Unassigned | — | Pending |
| `runbooks/incident-postmortem-template.md` | No deploy-specific data; review only | Read-through | Unassigned | — | Pending |
| `runbooks/reentrancy-suspicion.md` | Confirm "do not redeploy as an investigation step" still holds | Read-through | Unassigned | — | Pending |
| `runbooks/expired-account-recovery-check.md` | Review IDs and recovery-address assumptions | Read-through | Unassigned | — | Pending |

## 6. Monitoring

`testnet/monitoring/` contains plans, not running monitoring. Do not describe
any of it as active after a redeploy.

| Document | Update required | Verify | Owner | Date | Status |
|---|---|---|---|---|---|
| `monitoring/README.md` | Reconfirm the deployment-record warning and the "not configured" status | Compare with the registry | Unassigned | — | Pending |
| `monitoring/explorer-links.md` | Replace all four old contract IDs and their explorer URLs | Open each new link | Unassigned | — | Pending |
| `monitoring/event-watchlist.md` | Update contract-ID filters; confirm topic names against `event-topics.md` | Compare with source | Unassigned | — | Pending |
| `monitoring/dashboards-plan.md` | Review for embedded IDs; keep the proposal label | Read-through | Unassigned | — | Pending |
| `monitoring/alert-thresholds.md` | Review for embedded IDs; keep the proposal label | Read-through | Unassigned | — | Pending |
| `monitoring/balance-thresholds.md` | Review for embedded IDs and reserve assumptions | Read-through | Unassigned | — | Pending |
| `monitoring/uptime-check-spec.md` | Update example IDs; keep the spec-only status line | Read-through | Unassigned | — | Pending |
| `monitoring/synthetic-transaction-plan.md` | Update the `XLM_SAC_ID` note if needed (it is passphrase-derived) and any Bridgelet IDs | Read-through | Unassigned | — | Pending |

## 7. Tooling

`testnet/tooling/` holds specifications, not runnable tools. Update example
output, never the status line.

| Document | Update required | Verify | Owner | Date | Status |
|---|---|---|---|---|---|
| `tooling/README.md` | Review the status block and index; no ID changes expected | Read-through | Unassigned | — | Pending |
| `tooling/nonce-inspector-spec.md` | Replace the concrete controller ID in sample output | Re-read sample against a fresh `get_nonce` | Unassigned | — | Pending |
| `tooling/event-tail-spec.md` | Review sample contract IDs and native XLM SAC value | Read-through | Unassigned | — | Pending |
| `tooling/contract-id-lookup-spec.md` | Confirm "report where the ID came from and how fresh" still fits the new artifacts | Read-through | Unassigned | — | Pending |
| `tooling/faucet-helper-spec.md` | Confirm initialize argument count and role assumptions | Read-through | Unassigned | — | Pending |
| `tooling/sweep-signer-integration-notes.md` | Confirm signer/destination assumptions; no secret values in examples | Read-through | Unassigned | — | Pending |
| `tooling/decision-log.md` | Review only; do not turn an undecided question into a decision | Read-through | Unassigned | — | Pending |

## 8. Security

Security documents are observations and reasoned forecasts. Do not convert a
forecast into an observation because a deploy happened.

| Document | Update required | Verify | Owner | Date | Status |
|---|---|---|---|---|---|
| `security/README.md` | Add a dated baseline observation only if one was actually taken; otherwise keep the current-state block | Read-through + saved observation | Unassigned | — | Pending |
| `security/dependency-audit-notes.md` | Refresh the version chain and the deployed commit/WASM hash row if the SDK or CLI changed | Rebuild the deployed commit's `Cargo.lock` | Unassigned | — | Pending |
| `security/admin-key-hygiene.md` | Review key-custody guidance; note that a redeploy does not rotate the deployer key | Read-through | Unassigned | — | Pending |
| `security/known-testnet-abuse-patterns.md` | Reconcile P2–P8 examples with the new controller/accounts; keep the reasoning, update IDs | Compare with the abuse-pattern source claims | Unassigned | — | Pending |
| `security/unverified-signature-path-warning.md` | Confirm the `sweep()` stub status in the deployed WASM | Compare deployed hash with source | Unassigned | — | Pending |
| `security/reentrancy-observation-log.md` | No deploy-specific edit; keep the append-only format | Read-through | Unassigned | — | Pending |
| `security/disclosure-process.md` | Review routes only | Read-through | Unassigned | — | Pending |

## 9. Reset and communication

| Document | Update required | Verify | Owner | Date | Status |
|---|---|---|---|---|---|
| `reset/README.md` | Record the run date and operator in the change, not in the procedure itself | Compare with the changelog entry | Unassigned | — | Pending |
| `reset/communication-template.md` | Fix the stale `config/testnet_contracts.json` path before use; send the notice through the changelog channel | Confirm the path exists before publishing | Unassigned | — | Pending |
| `policies/reset-policy.md` | Review the worked-example drift section; update only if the disagreement is actually resolved | Compare registry with artifacts | Unassigned | — | Pending |
| `policies/versioning-scheme.md` | Apply the proposed tag if one was assigned; keep the "proposed, not enforced" status | Compare with the three tagged artifacts | Unassigned | — | Pending |
| `policies/breaking-change-notice.md` | Confirm the notice owed was given; add a retrospective entry if an exemption was used | Compare notice date with deploy date | Unassigned | — | Pending |
| `policies/data-retention.md` | Review only; no deploy-specific IDs | Read-through | Unassigned | — | Pending |
| `policies/contribution-guide.md` | Review only; no deploy-specific IDs | Read-through | Unassigned | — | Pending |

## Outside this tree

These are changed by the deploy flow or by a separate owner. Listed so nobody
mistakes them for documentation edits:

| Artefact | Who changes it |
|---|---|
| `deployments/testnet.json` | Rewritten by `scripts/deploy-testnet.sh` |
| `deployment-artifacts/contract-ids.txt` | Rewritten by `scripts/deploy-testnet.sh` |
| Root `.env.example` | Deployment-time credential template; never hold secrets in `testnet/` |
| `docs/api-reference.md` | Out of scope for this documentation set |
| `scripts/`, `tools/` | Out of scope; raise a separate issue for a real bug |

## Definition of done

- [ ] Every row above has an Owner, a UTC Date, and a Status other than
      `Pending` or `In progress`.
- [ ] Every `Verified` row was checked against a live read-only invocation,
      not against the deploy script's output.
- [ ] `contract-status.md` and `last-verified.json` agree with each other.
- [ ] `network-config.md`, `getting-started.md`, and `changelog.md` carry the
      same IDs and the same deployed commit.
- [ ] No document claims liveness that was not observed, and no secret or
      signing seed appears in any edited file.
- [ ] The announcement was published before the redeploy, or the exemption
      and retrospective entry are recorded.

## Related Documentation

- [`README.md`](README.md) — the manual fresh-deployment procedure
- [`communication-template.md`](communication-template.md) — the announcement
  template, including its stale-path caveat
- [`../policies/reset-policy.md`](../policies/reset-policy.md) — what is
  destroyed, who decides, and the three verification steps
- [`../registry/contract-status.md`](../registry/contract-status.md) — the
  canonical status record
- [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md)
  — the procedure behind the verification gate
- [`../policies/breaking-change-notice.md`](../policies/breaking-change-notice.md)
  — what notice was owed before the change
- [`../policies/versioning-scheme.md`](../policies/versioning-scheme.md) — the
  proposed deployment tag

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-11-18 | Initial post-redeploy documentation checklist with owner/date/status tracking |
