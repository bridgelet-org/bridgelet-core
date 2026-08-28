# ADR: Multi-Signature for ReserveContract Parameter Changes (Issue #523)

## Status
Decided (evaluation only — no code change in this document).

## Context
`contracts/reserve_contract/src/lib.rs` gates `set_base_reserve` behind a
single stored `admin: Address` set once in `initialize` and checked via
`admin.require_auth()`. The base reserve is a platform-wide financial
parameter consumed by `ephemeral_account::resolve_base_reserve` on every
account initialization.

## Threat Assessment (single-key model)
- **Key compromise**: one leaked/stolen admin key lets an attacker call
  `set_base_reserve` up to `MAX_RESERVE_STROOPS` (100,000,000,000 stroops),
  immediately corrupting reserve accounting for every newly-initialized
  `EphemeralAccount` that reads it.
- **Insider risk**: a single signer can unilaterally change platform economics
  with no second reviewer, no delay, and no on-chain record of dissent.
- **Availability risk**: loss of the single admin key permanently freezes
  `set_base_reserve` (no recovery path exists in the current contract).

These are realistic, not theoretical — `set_base_reserve` is the *only*
gate protecting this parameter, and it is called rarely enough that the
UX cost of a stronger scheme is low relative to the blast radius of a
mistake.

## Recommendation
**Move to a threshold/multi-sig model** rather than keep single-admin.
Rationale: the parameter is financially consequential and platform-wide,
call frequency is low, and a compromised single key has no containment.

### Rough Implementation Plan
1. Replace `storage::DataKey` admin entry with a `Vec<Address>` of signers
   plus a `threshold: u32`.
2. `set_base_reserve` becomes a two-step propose/approve flow: `propose_reserve_change(amount)`
   stores a pending value keyed by a monotonic proposal id; `approve_reserve_change(id)`
   requires `require_auth()` from a distinct signer each call; once
   `approvals >= threshold`, the value is committed and a
   `BaseReserveUpdated` event emitted (unchanged event shape).
3. Reuse whichever `AdminGovernance`/multisig primitive contract is adopted
   elsewhere in this repo's proposed contract set, rather than
   reimplementing signer-set storage and quorum counting locally — if that
   contract lands first, `reserve_contract` should call into it instead of
   step 1–2 above.

## Consequence
This is a design decision only; implementation should be tracked as a
follow-up issue once (or if) a shared multisig primitive's interface is
finalized.
