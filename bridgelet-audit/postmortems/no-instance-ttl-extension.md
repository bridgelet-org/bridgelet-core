<!--
Purpose: Postmortem documenting the inconsistent instance-storage TTL extension across the four core contracts.
Owner: Gracora (closes #320).
Status: Documentation-only. No contracts, docs, scripts, or tools changes are introduced by this file.
-->

# Postmortem: Missing Instance-Storage TTL Extension Across Three of Four Contracts

| Field | Value |
| :--- | :--- |
| **Related issue** | [#320](https://github.com/bridgelet-org/bridgelet-core/issues/320) |
| **Owner / reviewer** | `_operator-name_` |
| **Last reviewed** | `_ISO-8601 date_` |

## Background: instance-storage TTL vs per-key TTL

Soroban separates storage into three tiers with different cost and lifetime characteristics:
**instance**, **persistent**, and **temporary**. Each tier has a *TTL* — the number of ledger
closings before the entry is archived (evicted). The TTL must be actively extended by the
contract; Soroban does not auto-renew entries.

**Per-key TTL** (`storage().persistent().extend_ttl(...)`) controls how long an individual
storage key survives. This is already handled correctly in all four contracts: each
`set_*` helper writes to persistent storage and relies on the caller to bump TTL when needed.

**Instance-storage TTL** (`storage().instance().extend_ttl(threshold, extend_to)`) controls how
long the *contract instance itself* remains live. If the instance TTL expires, the contract
becomes inaccessible — all subsequent calls fail with a host error, even if the individual
storage keys still have TTL remaining. This is the concern at issue: instance storage is
where the contract's DataKey enum, init flags, and version metadata live.

## The inconsistency

Each of the four core contracts defines `storage::extend_instance_ttl()` using the same
constants (`threshold = 100`, `extend_to = 518_400`). But adoption is uneven:

| Contract | Has `extend_instance_ttl`? | Called in every public fn? | Notes |
| :--- | :--- | :--- | :--- |
| **`reserve_contract`** | Yes | **Yes** — every public fn calls it as its first line | Consistent and correct. |
| **`ephemeral_account`** | Yes | **Mostly** — called in 13 of 15 public fns. Missing from `expire()` and `recover()` (which are permissionless expiry entry points). | Minor gap. |
| **`sweep_controller`** | Yes | **Mostly** — called in 6 of 9 public fns. Missing from `update_authorized_signer`, `apply_signer_update`, and `get_pending_signer_update`. | Minor gap. |
| **`account_factory`** | **No** | **Never** — no `extend_instance_ttl` helper exists and neither `initialize()` nor `batch_initialize()` calls it. | Complete absence. |

### Why this matters

`AccountFactory` stores two critical instance-level values: the `EphemeralAccountWasmHash`
and the `BatchNonce`. If the factory's instance TTL expires:

1. `batch_initialize` calls fail with a host-level TTL expiry error, halting account
   deployment entirely.
2. The stored WASM hash becomes unreachable, so no new ephemeral accounts can be deployed
   until the factory is re-deployed and re-initialized.
3. The monotonic `BatchNonce` is lost (or rather, inaccessible), which means salt
   uniqueness guarantees for future batches cannot be verified.

For `ephemeral_account` and `sweep_controller`, the gaps are narrower but still matter:
`expire()` and the signer-rotation functions are called infrequently, but they operate on
live accounts. If a caller waits long enough without interacting with the contract, the
instance TTL could expire, trapping funds behind an inaccessible contract.

## Contrast with ReserveContract

`ReserveContract` is the only contract where every public function begins with
`storage::extend_instance_ttl(&env)`. This means the instance is extended on every
interaction, regardless of which function is called. Even if `set_base_reserve` is called
once a year, the instance TTL is bumped on that call and remains valid for another
518,400 ledgers (~29 days).

The other three contracts rely on callers frequently hitting functions that include the TTL
extension. If a contract's callers go quiet for longer than the TTL window, the instance
expires — a failure mode that does not exist in `ReserveContract`.

## Lessons learned

When reviewing individual functions in isolation, instance-storage TTL is easy to miss because
it does not affect the function's own logic. A function that reads and writes persistent
storage keys looks complete without an instance TTL bump. The omission only surfaces when the
caller's invocation frequency drops below the TTL threshold — a condition that is invisible
in unit tests (where the test environment's ledger never advances far enough to trigger TTL
expiry) and only manifests in long-running production deployments.

A project-wide convention — enforced by a lint rule or a checklist item — that *every public
function must call `extend_instance_ttl` as its first line* would have caught all three gaps
before they reached production.

---

## Related Issues

- [#427](https://github.com/bridgelet-org/bridgelet-core/issues/427) — account_factory never extends instance storage TTL either
- [#428](https://github.com/bridgelet-org/bridgelet-core/issues/428) — reserve_contract is the only contract with correct TTL handling, access control, and bounds checking — and it's unused
- [#224](https://github.com/bridgelet-org/bridgelet-core/issues/224) — Remove hardcoded base reserve constant
