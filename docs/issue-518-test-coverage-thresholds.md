# Test Coverage Measurement & Thresholds

Tracking: #518

## Current state

`docs/testing.md` mentions `cargo-tarpaulin` as an option, but no coverage
tool is actually installed or run in `.github/workflows/test.yml`. The
workflow only runs `cargo test --verbose`, `cargo fmt -- --check`, and
`cargo clippy -- -D warnings` per crate under `contracts/*`. There is no
coverage report artifact and no failure mode tied to coverage regressing.

## Tooling choice

Use `cargo-llvm-cov` rather than `cargo-tarpaulin`:
- Works reliably for `no_std` Soroban contract crates targeting
  `wasm32v1-none`, where tarpaulin's ptrace-based instrumentation is
  unreliable.
- Ships an `llvm-cov` subcommand with `--fail-under-lines <PCT>`, which maps
  directly to CI threshold enforcement without extra scripting.

Per-crate invocation (run from each `contracts/<name>` directory, matching
the existing loop structure in `test.yml`):

```bash
cargo llvm-cov --workspace --lcov --output-path lcov.info
cargo llvm-cov report --fail-under-lines 60
```

## Threshold

An initial threshold must be measured, not assumed. Until a baseline run is
recorded per crate, treat 60% line coverage as the floor for crates with an
existing `src/test.rs`, and 0% (report-only, non-blocking) for crates
without tests yet. Raise the floor in 5-10 point increments per quarter as
gaps close, following the same trajectory bridgelet-sdk used to reach its
80% threshold.

## CI wiring

Add a `coverage` job to `.github/workflows/test.yml` that loops over the
same 16 contract directories as the `test` job, runs the two commands
above, and uploads `lcov.info` per crate as a build artifact so regressions
are visible in the PR checks list.
