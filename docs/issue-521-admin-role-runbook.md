# Admin Role Model & Rotation Runbook

Tracking: #521

## Per-contract admin model (as implemented today)

| Contract | Admin storage | Set at | Rotatable via public entrypoint? |
|---|---|---|---|
| `access_controller` | `super_admin` + role map (`storage::set_role`) | `initialize(super_admin)` | Yes — `grant_role`/`revoke_role`, gated by `require_admin_or_super_admin` (lib.rs:13-23) |
| `reserve_contract` | single `admin` (`storage::set_admin`) | `initialize(admin)` | **No** — `set_admin` exists in `storage.rs:60` but is only called from `initialize`; no public rotate function |
| `ephemeral_account` | single `admin` (`storage.rs:231`, `request.rs:222`) | internal, tied to account creation | **No** — helpers exist but are not exposed via `lib.rs` |
| Other 13 contracts | not yet audited individually | — | Assume fixed at init until confirmed otherwise |

`access_controller` is the only contract with a genuine rotation path. For
`reserve_contract` and `ephemeral_account`, "rotation" today means
redeploying the contract with a new `admin` address — there is no way to
reassign the existing instance's admin. This gap should be tracked as
follow-up work (new `transfer_admin` entrypoints), not silently assumed
solved by this doc.

## Rotation runbook: `access_controller`

1. Confirm current holders: `get_super_admin()` and `has_role("admin", X)`
   for each candidate account.
2. New admin: `grant_role(super_admin, "admin", new_admin)`, authorized by
   the current `super_admin` key.
3. Verify immediately: `has_role("admin", new_admin) == true`.
4. Old admin: `revoke_role(super_admin, "admin", old_admin)`.
5. Verify: `has_role("admin", old_admin) == false`, and confirm a call
   authorized only by `old_admin` now fails with `Unauthorized`.
6. `super_admin` itself cannot be rotated by this contract (no
   `transfer_super_admin` function exists) — rotating it requires
   redeployment. Note this explicitly to operators before relying on
   `access_controller` as a long-lived root of trust.

Steps 2-5 correspond directly to the test cases in
`docs/issue-519-admin-role-transfer-tests.md`; that suite is the
non-production validation referenced by this runbook and should be run
against a fresh `Env` before any real rotation.

## Cross-reference

bridgelet-sdk holds keys that call into these contracts' admin-gated
functions; its operational docs should link back to this file's table
before performing any rotation so the SDK-side key material stays in sync
with which contracts actually support rotation.
