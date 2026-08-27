# Admin-Role Transfer & Revocation Test Plan

Tracking: #519

## Contracts with rotatable admin state

Only `access_controller` (`contracts/access_controller/src/lib.rs`) exposes
a rotation-capable model today, via `grant_role` / `revoke_role` gated by
`require_admin_or_super_admin`. `reserve_contract` and `ephemeral_account`
each have a `storage::set_admin` helper but no public entrypoint calls it
outside `initialize` — admin is effectively fixed at deploy time in both.
This plan targets `access_controller` directly and flags the other two as a
gap (see `docs/issue-521-admin-role-runbook.md`).

## Required tests (new: `contracts/access_controller/src/test.rs` cases)

1. **Old holder loses access immediately** — `grant_role(caller, "admin",
   old_admin)`, then `revoke_role(caller, "admin", old_admin)`. Assert
   `has_role("admin", old_admin) == false` and that a subsequent
   `grant_role` call authorized only by `old_admin` panics with
   `Error::Unauthorized`.
2. **New admin has no-gap access** — `grant_role(super_admin, "admin",
   new_admin)`, then immediately call `revoke_role(new_admin, "admin",
   some_other_account)` authorized by `new_admin` alone. Assert it succeeds
   in the same test without any intervening state change.
3. **Unauthorized self-assignment is rejected** — an account with no role
   calls `grant_role(attacker, "admin", attacker)` authorized only by
   itself. Assert `Error::Unauthorized`, using `env.mock_auths` with a
   single mocked auth for `attacker` (not `mock_all_auths`) so the
   authorization check is actually exercised rather than bypassed.
4. **Super admin status cannot be revoked** — call `revoke_role(caller,
   "super_admin", super_admin)` and assert `Error::CannotRevokeSuperAdmin`,
   covering the existing guard at lib.rs:66-68.
5. **Interference during transfer** — while `super_admin` is mid-transfer
   (grant to `new_admin` issued, revoke of `old_admin` not yet submitted),
   an unrelated caller attempts `revoke_role` targeting `new_admin`.
   Assert it fails unless the caller is `super_admin` or holds `"admin"`.

## Auth mocking note

Use `env.mock_auths(&[...])` with explicit `MockAuth` entries per test
rather than `env.mock_all_auths()`, since the latter defeats the purpose of
authorization tests by approving every `require_auth()` call unconditionally.
