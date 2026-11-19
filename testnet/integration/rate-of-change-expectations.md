# Rate of Change Expectations on Testnet

Bridgelet's testnet is an **Active Development / MVP / not audited**
environment. It exists to exercise integration paths, not to provide a stable
audited deployment. Contract behavior, IDs, WASM hashes, and event payloads can
change more frequently and with less notice than a stable audited deployment.
Do not design an integration that assumes a testnet instance or interface will
remain unchanged.

> As of this writing, the canonical
> [contract status registry](../registry/contract-status.md) records two TBD and
> two N/A IDs (EphemeralAccount and SweepController: `TBD`;
> ReserveContract and AccountFactory: `N/A`), all Last Verified `Never`. Older
> artifacts in the repo publish concrete IDs; they conflict with the registry.
> Nothing in this document asserts that any contract is live.

## What is and is not promised

| Area | Expectation |
|---|---|
| Uptime and availability | Best effort only. No SLA, no on-call shift, and no guaranteed response time; see [support](../docs/support.md) and [on-call-testnet.md](../runbooks/on-call-testnet.md). |
| Change cadence | No fixed schedule, no release train commitment, and no promise of a notice window. |
| Notice before changes | None is guaranteed. Emergency fixes, rebuilds, and redeploys may be recorded only after the fact. |
| Backward compatibility | Not guaranteed between testnet instances, even when a version label looks small. |
| State and IDs | A testnet reset or redeploy can make stored IDs stop resolving and erase contract state. |
| Audit status | Not audited. Contracts can have known limitations and unreviewed changes. |
| Event history | Served by RPC/Horizon within their retention windows; not an archive. Plan your own persistence. |
| Test data | Public and effectively permanent. Never use real customer data, credentials, or reusable signing material. |

The [version policy](../policies/versioning-scheme.md), changelog, and
upgrade-history files describe intent and observed history; they are not a
contract with integrators.

## Why changes happen

1. **Active development.** Source in this checkout can be ahead of anything
   ever deployed. For example, current `EphemeralAccount::initialize` takes five
   arguments, while [changelog.md](../docs/changelog.md) describes a future
   sixth `authorized_signer: BytesN<32>` argument. Treat the deployed interface,
   not either document, as authoritative.
2. **Resets and redeploys.** [reset-policy.md](../policies/reset-policy.md)
   distinguishes a shared Stellar testnet reset from a Bridgelet redeploy.
   Neither has a fixed cadence or guaranteed advance notice; both can change
   head ledgers and invalidate stored IDs.
3. **Unsupported contracts becoming used.** `ReserveContract` and
   `AccountFactory` are currently undeployed and outside the supported deploy
   tooling. They may remain absent, be deployed later, or be deployed with a
   different interface.
4. **Incident response and upgrades.** A fix can change an error code, event
   topic, or storage layout. Upgrade effects are not always observable as an
   event; see [upgrade-verification.md](../runbooks/upgrade-verification.md).
5. **Breaking-change notices are proposed, not enforcement.**
   [breaking-change-notice.md](../policies/breaking-change-notice.md) defines
   obligations the project *should* meet, but it explicitly notes the current
   source/registry conflict and provides no automatic guarantee for clients.

## Where to look for change signals

Check these before a release, before a batch job, and when a read starts
failing:

| Signal | Source |
|---|---|
| Current deployment claim | [contract-status.md](../registry/contract-status.md), [last-verified.json](../registry/last-verified.json) |
| Manual liveness check | [verify-contract-live.md](../registry/verify-contract-live.md) |
| Observed code changes | [upgrade-history.md](../registry/upgrade-history.md), [changelog.md](../docs/changelog.md) |
| Reset detection and response | [reset-policy.md](../policies/reset-policy.md), [rpc-outage.md §6](../runbooks/rpc-outage.md#6-detect-a-testnet-reset-during-recovery) |
| Event-name and payload drift | [event-topics.md](../registry/event-topics.md), [event-watchlist.md](../monitoring/event-watchlist.md) |
| Source-level interface drift | [soroban-cli-quickref.md](../docs/soroban-cli-quickref.md) and `stellar contract info interface` |

A release tag such as `testnet-v1.x` identifies a source snapshot; it does not
guarantee that the tagged code is what is currently deployed.

## Integrator guidance

### Pin and verify

- Record the exact passphrase `Test SDF Network ; September 2015`, RPC URL,
  network name, and a timestamped snapshot of the registry in every run log.
- Resolve contract IDs from the registry at runtime. Treat IDs copied from
  `network-config.md`, `getting-started.md`, or deployment artifacts as claims
  until a read-only call succeeds.
- Record the WASM hash and generated interface for each verified instance.
  Re-check them when a read fails or an error code changes; use
  `stellar contract info interface --contract-id <ID> --network testnet`.

### Design for change

- Put contract calls behind a thin adapter so an argument or error-code change
  does not spread through the application.
- Use read-only preflight calls (`--send no`) before state-changing calls; see
  [soroban-cli-quickref.md](../docs/soroban-cli-quickref.md).
- Feature-detect functions rather than assuming a deploy contains every method
  listed in source.
- Handle unknown topics and unknown error codes explicitly. Add them to your
  log and stop the affected path instead of guessing a migration.
- Store a reset epoch with every persisted record. If the head ledger drops or
  a stored ID stops resolving, start a new epoch and keep the old data for
  reconciliation; see [pagination-and-history.md](pagination-and-history.md).
- Pin dependency revisions and regenerate bindings deliberately. Do not assume
  a client library will absorb a contract-interface change.

### Expect these specific drifts

| Drift | Example | Response |
|---|---|---|
| Initialization signature change | Five-argument source vs. described six-argument `initialize` | Verify deployed interface; do not send a guessed argument list |
| Event naming mismatch | Source `init`/`reserve` vs. registry `initialized`/`base_reserve_updated` for ReserveContract | Filter on the deployed ABI; record the observed symbol |
| Event emitted but no transfer | `expired` or `reserve` metadata without a SEP-41 transfer | Reconcile per-asset balances; see [expired-account-recovery-check.md](../runbooks/expired-account-recovery-check.md) |
| Absent event | Upgrade emits no event; indexer may be proposed but not running | Use transaction and code-hash evidence; see [upgrade-verification.md](../runbooks/upgrade-verification.md) |
| ID no longer resolves | Reset or redeploy | Re-read the registry, start a new epoch, and replay from your own archive |

### Do not depend on

- A named maintainer being on call or a response deadline.
- Any stored ID, WASM hash, or interface remaining valid for a release cycle.
- Contract state surviving a reset.
- RPC or Horizon retaining all events for the lifetime of the testnet.
- Changelog entries arriving before a change.
- Testnet semantics being identical to a future audited deployment.

## Incident and escalation posture

If a change breaks your integration, follow
[troubleshooting-index.md](../docs/troubleshooting-index.md) and the matched
runbook. Escalation is best-effort: file a `[testnet]` issue with the exact
error, observed interface, transaction hash, ledger, and client version. Do not
publish secrets or reusable signatures. For suspected security defects, use the
repository's private vulnerability report flow instead.

## Related documents

- [README](../../README.md): project maturity statement
- [network configuration](../docs/network-config.md): endpoints and passphrase
- [troubleshooting index](../docs/troubleshooting-index.md): runbook routing
- [data retention policy](../policies/data-retention.md): public/permanent data
  rules
- [pagination and history](pagination-and-history.md): audit-trail design

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-11-18 | Initial Active Development / MVP expectations and integrator guidance |
| 1.1 | 2026-11-18 | Corrected registry ID wording and repository README link |
