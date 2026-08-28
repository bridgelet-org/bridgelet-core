# ADR: Timelock for Reserve/Fee Parameter Changes (Issue #524)

## Status
Decided (evaluation only — no code change in this document).

## Context
`contracts/reserve_contract/src/lib.rs::set_base_reserve` takes effect
*immediately* upon a successful admin-authorized call — the new value is
written to storage and is live for the very next `resolve_base_reserve`
cross-contract read from any `EphemeralAccount::initialize` call. There is
no queue/delay step. Fee-related parameters (e.g. in a `fee_splitter`-style
contract elsewhere in this repo) follow the same immediate-write pattern.

## Problem
Even a *correctly authorized* change to `base_reserve` gives integrators
building against `get_base_reserve()`/`require_base_reserve()` zero warning
before the value moves, which can break assumptions baked into off-chain
reserve-accounting logic mid-flight.

## Decision
**A timelock is warranted for `set_base_reserve` specifically**, and for any
analogous fee-rate-setting call in `fee_splitter`, because:
- Both are read by other contracts/integrators as a stable-ish reference
  value, not a per-transaction input.
- Both change rarely, so a mandatory delay has near-zero operational cost.

A timelock is **not** proposed here for read-only or per-call parameters
(e.g. per-payment `amount` in `record_payment`) — those are not admin-set
platform parameters and a delay would be meaningless.

## Rough Design (queue/execute pattern)
1. `queue_base_reserve(amount) -> Result<u32, Error>` — admin-authorized,
   validates `amount` exactly as `set_base_reserve` does today, stores
   `(amount, unlock_ledger = current_ledger + DELAY_LEDGERS)` under a
   proposal id, emits `BaseReserveQueued`. No mutation to the live value yet.
2. `execute_base_reserve(id) -> Result<(), Error>` — permissionless (mirrors
   `expire()`'s permissionless-cleanup pattern), succeeds only once
   `env.ledger().sequence() >= unlock_ledger`, then performs the same write
   + `BaseReserveUpdated` emission `set_base_reserve` does today.
3. `DELAY_LEDGERS` should be a named constant reviewed alongside
   `MAX_RESERVE_STROOPS`, not hardcoded inline.

## Cross-Reference
A general-purpose `TimelockController` pattern has already been proposed as
a standalone contract in this repo's broader proposed-contract set.
**`reserve_contract` should consume that shared primitive once it exists**
rather than reimplementing proposal-id/unlock-ledger storage locally. This
document's sketch above is the interface `reserve_contract` would need.

## Consequence
Tracked as a follow-up implementation issue, gated on the `TimelockController`
contract's interface being finalized.
