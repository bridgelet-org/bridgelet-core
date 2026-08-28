# Admin / Privileged-Parameter Access Control Audit (#513)

## Scope reviewed
`contracts/reserve_contract/src/lib.rs`, cross-checked against the equivalent admin-mutation
patterns in `contracts/sweep_controller/src/lib.rs`.

## Admin-mutable parameters and current authorization model

| Contract | Function | Auth model | Timelock / multi-sig? |
|---|---|---|---|
| reserve_contract | `set_base_reserve` (lib.rs:93) | single `admin.require_auth()`, admin fixed at `initialize` | **None** |
| reserve_contract | `initialize` | one-time, sets admin, no rotation function exists | N/A (one-shot) |
| sweep_controller | `update_authorized_destination` (lib.rs:345) | single `creator.require_auth()`, blocked once a sweep has occurred | **None** |
| sweep_controller | `update_authorized_signer` (lib.rs:417) | single `creator.require_auth()` | **Yes** — 48h (`SIGNER_TIMELOCK_LEDGERS`) delay before taking effect |

## Risk assessment
- `reserve_contract::set_base_reserve` is bounded by `MAX_RESERVE_STROOPS` (10,000 XLM,
  lib.rs:20) which limits blast radius of a fat-fingered value, but a **compromised admin key**
  can still change the live base-reserve figure instantly and unilaterally — every ephemeral
  account's reserve-tracking math depends on this value, so an instant malicious change could
  misstate how much of a swept balance is "reserve" vs. user funds.
- `reserve_contract` has no admin-rotation function at all: if the admin key is lost or
  compromised, there is no path to replace it short of redeploying the contract.
- `sweep_controller::update_authorized_destination` has no timelock, unlike its sibling
  `update_authorized_signer`, which does — an inconsistency within the same contract.

## Recommendation
1. `reserve_contract::set_base_reserve` is the highest-priority candidate for a timelock: the
   repo already has a working timelock pattern to copy (`sweep_controller`'s
   `SIGNER_TIMELOCK_LEDGERS`), and a `contracts/timelock_controller` contract exists in this repo
   that could be composed in instead of duplicating the pattern.
2. Add an admin-rotation function to `reserve_contract` (with its own timelock) so key loss isn't
   an unrecoverable, redeploy-only event.
3. Apply the same timelock already used for `update_authorized_signer` to
   `update_authorized_destination` for consistency, or document why the two are intentionally
   different.
4. File follow-up issues for items 1-3 individually so each can be reviewed and merged
   independently.
