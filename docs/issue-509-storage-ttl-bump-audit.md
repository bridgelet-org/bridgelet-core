# Issue #509: Storage TTL / Bump Policy Audit

Scope: `contracts/*/src/storage.rs` across all 16 CI-covered contracts.

## What was found

Every contract's `storage.rs` reviewed uses `env.storage().instance()`
exclusively — none of the 16 contracts under `contracts/*` use
`env.storage().persistent()` or `env.storage().temporary()`. This matches
`docs/storage-ttl-strategy.md`'s design rationale ("all bridgelet-core
storage entries use instance storage").

Both `sweep_controller/src/storage.rs` and `reserve_contract/src/storage.rs`
define an explicit bump policy: `extend_instance_ttl` calls
`env.storage().instance().extend_ttl(100, 518_400)` (threshold 100 ledgers,
extend to ~30 days at 5s/ledger). It is called at the top of every
state-changing `SweepController` function (`initialize`, `execute_sweep`,
`claim`, `update_authorized_destination`) and every state-changing
`ReserveContract` function (`initialize`, `set_base_reserve`).

## Findings (correcting a stale assumption)

1. **`docs/storage-ttl-strategy.md` currently says** "TTL is managed by the
   Soroban runtime and does not require manual extension" for instance
   storage. Inaccurate for this codebase: `extend_instance_ttl` is a
   deliberate, explicit bump call, not automatic runtime behavior. Instance
   storage still archives if never bumped — this doc should be corrected as
   a follow-up (out of scope for this audit's file additions).
2. **Read-only/view functions never bump TTL.** `can_sweep`, `get_nonce`,
   `get_reserve_info`, `fee_estimate`, `get_pending_signer_update` (sweep
   controller) and `get_base_reserve`/`has_base_reserve`/`get_admin` (reserve
   contract) never call `extend_instance_ttl`. An instance that is only ever
   *queried* (e.g. an in-progress sweep polled by `can_sweep` without any
   `execute_sweep` landing) will not have its TTL extended by those queries
   — matching the "in-progress sweep record" risk named in the issue body.
3. **All entries in one contract instance share one TTL** — no per-entry
   differentiation is possible or needed; one `extend_instance_ttl` call
   refreshes the whole instance uniformly.

## Recommendation

No entries found in active premature-archival risk today (every
state-mutating path already bumps TTL), but finding (2) should be tracked:
if a long-lived instance is expected to survive on read-only traffic alone,
add a TTL bump to the relevant view function, or document that view-only
usage does not extend contract lifetime.
