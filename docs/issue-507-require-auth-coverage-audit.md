# Issue #507: `require_auth` Coverage Audit

Scope: `contracts/ephemeral_account/src/lib.rs`,
`contracts/sweep_controller/src/lib.rs`, `contracts/reserve_contract/src/lib.rs`.

## EphemeralAccount (`contracts/ephemeral_account/src/lib.rs`)

| Function | Expected caller | Auth check |
| --- | --- | --- |
| `initialize` | creator | `creator.require_auth()` (line 61) |
| `record_payment` | authorized controller | `controller.require_auth()` (line 114) |
| `sweep` | sweep controller (via Ed25519 sig, not `require_auth`) | signature-based, see below |
| `sweep_claim` | controller | `controller.require_auth()` (line 219) |
| `recover` | designated recovery caller | `caller.require_auth()` (line 444) |
| `upgrade` | admin | `admin.require_auth()` (line 466) |
| `is_expired`, `is_initialized`, `get_status`, `get_reserve_remaining`, `get_reserve_available`, `is_reserve_reclaimed`, `get_last_reserve_event`, `get_reserve_reclaim_event_count`, `get_info`, `simulate_sweep` | any (read-only) | none needed — no state mutation |
| `expire`, `reclaim_reserve` | any (permissionless by design) | none — these are intentionally callable by anyone once the expiry condition is on-chain-verifiable |

**Note on `sweep`**: authorized indirectly — `SweepController` calls
`env.authorize_as_current_contract()` with a `sweep`-scoped auth entry before
invoking it, instead of the ephemeral account calling `require_auth()`
directly. Intentional architecture (see `authorize_ephemeral_sweep` in
`sweep_controller/lib.rs`), not a missing check.
## SweepController (`contracts/sweep_controller/src/lib.rs`)

| Function | Expected caller | Auth check |
| --- | --- | --- |
| `initialize` | creator | `creator.require_auth()` |
| `execute_sweep` | signer (off-chain Ed25519 sig) | `AuthContext::verify()` -> `ed25519_verify` (not `require_auth`) |
| `claim` | recipient | `recipient.require_auth()` |
| `update_authorized_destination` | creator | `creator.require_auth()` |
| `update_authorized_signer` | creator | `creator.require_auth()` |
| `apply_signer_update` | any (time-lock gated, not caller-gated) | none — intentional, gated by `effective_ledger` |
| `migrate`, `get_version`, `can_sweep`, `get_nonce`, `get_reserve_info`, `fee_estimate`, `get_pending_signer_update` | any (read-only/idempotent) | none needed |

## ReserveContract (`contracts/reserve_contract/src/lib.rs`)

| Function | Expected caller | Auth check |
| --- | --- | --- |
| `initialize` | admin | `admin.require_auth()` (line 63) |
| `set_base_reserve` | admin | `admin.require_auth()` (line 100) |
| `get_base_reserve`, `require_base_reserve`, `has_base_reserve`, `get_admin` | any (read-only) | none needed |

## Findings

No state-changing function in any of the three contracts is missing a
`require_auth()`/equivalent check. `execute_sweep`/`sweep()` intentionally
substitute signature verification / pre-attached auth entries for direct
`require_auth()` — documented architecture, not an oversight. No follow-up
issue required.
