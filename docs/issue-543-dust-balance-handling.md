# Dust-Balance Residue Handling After Sweep (#543)

## Location
`contracts/sweep_controller/src/lib.rs` (`sweep_account`, `claim`) and
`contracts/ephemeral_account/src/lib.rs` (`reclaim_reserve_to`,
`execute_sweep_core`).

## Problem
`sweep_account`/`claim` transfer the full recorded `Payment.amount` per asset via
`transfers::execute_transfers` — this is the primary balance sweep, separate from
the base-reserve tracking already implemented in `reclaim_reserve_to`
(`get_reserve_remaining`/`get_reserve_available`/`is_reserve_reclaimed`). After a
primary sweep, small residual amounts can remain: fee deduction on the source side,
rounding on multi-asset splits, or portions of the base reserve not yet available
per `reclaim_reserve_to`'s partial-reclaim logic (`reclaim_amount = min(available,
remaining)`).

## Decision: dust is recovered via the existing reserve-reclaim path, not swept anew
1. **Primary asset dust** (SEP-41 token remainders below what a second `transfer()`
   call is worth): the contract already transfers the *entire* recorded
   `Payment.amount` in one call per asset — there is no partial-transfer path today,
   so no primary-asset dust is created by the sweep logic itself. Any dust in this
   category originates from the sender under-funding relative to a quoted amount and
   is covered by #542 (`docs/issue-542-partial-payment-detection.md`), not by sweep
   logic.
2. **Base-reserve dust**: `reclaim_reserve_to` already implements incremental
   reclaim (`reserve_available` may be less than `reserve_remaining` on a given
   call) and is explicitly re-callable — `reclaim_reserve` is documented as "safe to
   call repeatedly: once fully reclaimed, subsequent calls transfer 0." This is the
   confirmed policy for reserve dust: **left and recovered via a follow-up
   `reclaim_reserve()` call**, not abandoned, and not force-swept inside
   `sweep_account`.
3. **Distinguishable events**: `reclaim_reserve_to` already emits `ReserveReclaimed`
   (`fully_reclaimed: bool`, `remaining_reserve: i128`) separately from
   `SweepCompleted`/`SweepExecutedMulti` — this is the existing mechanism satisfying
   the "dust event distinguishable from primary sweep event" requirement. No new
   event type is introduced; `fully_reclaimed: false` on a `ReserveReclaimed` event
   is the signal for off-chain accounting that dust remains.
4. Test coverage to add in `contracts/sweep_controller/src/test.rs` /
   `contracts/ephemeral_account/src/test.rs`: a sequence asserting a primary sweep
   leaves `is_reserve_reclaimed() == false` when `reserve_available <
   reserve_remaining`, followed by a second `reclaim_reserve()` call that drains the
   remainder and flips `fully_reclaimed` to `true`.
