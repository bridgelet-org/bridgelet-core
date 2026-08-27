# Frozen / AUTH_REQUIRED Asset Handling During Funding (#541)

## Location
`contracts/ephemeral_account/src/lib.rs` (`record_payment`).

## Problem
Stellar assets may set `AUTH_REQUIRED` (issuer must authorize each holder's
trustline) or be frozen by the issuer post-authorization. `record_payment` only
validates `amount > 0`, duplicate-asset, and the 10-asset cap
(`storage::get_total_payments`) — it has no way to inspect Stellar classic asset
flags, since Soroban contracts only see the SEP-41 token interface, not the
underlying trustline's `AUTH_REQUIRED`/`AUTH_REVOCABLE` flags.

## Decision: unsupported for this product's use case, documented explicitly
AUTH_REQUIRED / freezable assets are **not supported** by ephemeral accounts in this
release. Rationale: `record_payment` is only invoked by the `authorized_controller`
after it observes an inbound SEP-41 `transfer` completing on-chain — by definition,
if the transfer reached `record_payment`, the transfer itself already succeeded,
which for an AUTH_REQUIRED asset means the issuer had already authorized the
ephemeral account's trustline at deposit time. The unresolved risk is the freeze
case: an issuer freezes the trustline **after** `record_payment` but **before**
`sweep`/`sweep_claim` executes the `TokenClient::transfer` in
`transfers::execute_transfers`.

## Fail-fast behavior
Because Soroban contracts cannot query classic trustline flags directly, this is
not caught at `record_payment` time. Instead:
- If frozen between funding and sweep, `TokenClient::transfer` fails at sweep time
  the same way a clawback does (see #540 / `docs/issue-540-clawback-asset-handling.md`),
  reverting the sweep transaction atomically per the existing guarantee in
  `transfers.rs`.
- This is intentionally the same failure surface as clawback: both are
  issuer-triggered, post-funding balance losses outside contract control. No new
  contract logic is added to distinguish "frozen" from "clawed back" since neither
  is observable from the SEP-41 interface; `Error::ClawbackDetected` (2019, see
  #540) covers both.
- Integrators are advised (via `docs/security.md`) to restrict supported assets to
  those without `AUTH_REQUIRED`/`AUTH_REVOCABLE` set, verified off-chain before
  directing senders to fund an ephemeral account.
