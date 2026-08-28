# Asset allowlist enforcement at the EphemeralAccount layer

Closes #548.

## Current state

`EphemeralAccount::record_payment` (`contracts/ephemeral_account/src/lib.rs`)
accepts any `asset: Address` from the `authorized_controller` with no check
against an approved-asset list — only amount positivity, duplicate-asset, and
a 10-asset cap are enforced. `AssetAllowlist` (`contracts/asset_allowlist`)
already exists as a standalone, admin-gated registry with an O(1)
`is_allowed(asset)` read, but per its own doc comment it is not wired into
any consumer yet.

## Decision

Enforce at the `EphemeralAccount` layer, not only the external registry,
because `record_payment` is the actual point of asset receipt — an optional
registry a caller might not consult does not prevent an ephemeral account
from holding a disallowed asset.

## Proposed design

- Extend `initialize` with an optional `asset_allowlist: Option<Address>`
  (mirrors the existing `reserve_contract: Option<Address>` pattern from
  Issue #405), stored via a new `DataKey::AssetAllowlist`.
- In `record_payment`, before `storage::add_payment`, if an allowlist address
  is configured, cross-contract call `AssetAllowlist::is_allowed(asset)` and
  return a new `Error::AssetNotAllowed` on `false`.
- No allowlist configured preserves current permissive behavior (backward
  compatible with existing deployments).
- Do not duplicate allowlist storage in `EphemeralAccount` — always defer to
  `AssetAllowlist` as source of truth.

## Test coverage required

- `record_payment` succeeds for an asset present in a configured allowlist.
- `record_payment` returns `Error::AssetNotAllowed` for an asset absent from
  a configured allowlist.
- `record_payment` behaves as today (no rejection) when no allowlist is
  configured.
