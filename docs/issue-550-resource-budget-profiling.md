# Resource budget profiling for high-member-count operations

Closes #550.

## Current state

Two contracts have operations whose cost scales with collection size rather
than being O(1):

- `EphemeralAccount::record_payment` (`contracts/ephemeral_account/src/lib.rs`)
  already self-limits at 10 assets (`Error::TooManyPayments`), and
  `sweep`/`sweep_multi` in `SweepController` (`contracts/sweep_controller/src/lib.rs`,
  lines ~158 and ~242) iterate `info.payments` twice per sweep — once to
  compute totals, once to execute transfers.
- `BatchSweepQueue` (`contracts/batch_sweep_queue/src/lib.rs`) iterates a
  caller-supplied `limit` (line ~118) and runs an O(n²) insertion sort over
  `sorted_ids` when deduplicating the pending list (lines ~153-173) — this is
  the more concerning case since queue size is not currently capped the way
  `EphemeralAccount` payments are.

No measured Soroban instruction counts exist for either path at realistic
scale; the existing 10-asset cap on `EphemeralAccount` looks like an
engineering estimate, not a profiled ceiling.

## Proposed design

- Add a `cargo test`-invokable benchmark harness (or reuse
  `soroban_sdk::testutils::Env::cost_estimate` if available in the pinned SDK
  version) that runs `SweepController::sweep_multi` and
  `BatchSweepQueue`'s dequeue path at N = 5, 10, 25, 50, 100 items and
  records CPU instructions and memory bytes consumed at each N.
- Fit/observe where the curve approaches Soroban's per-transaction
  instruction limit and record that N as the documented ceiling.
- Cross-reference results against the previously-proposed resource-fee
  benchmarking suite (referenced in the issue) once that suite lands, rather
  than maintaining a second, divergent set of numbers.

## Output

- A ceiling documented per operation, e.g. "sweeping more than N assets in
  one `sweep_multi` call is not supported" and "`BatchSweepQueue` dequeue is
  not supported past N pending entries due to O(n²) dedup cost" — recorded in
  this docs/ directory once profiling data exists, and referenced from the
  relevant contract's doc comments.
