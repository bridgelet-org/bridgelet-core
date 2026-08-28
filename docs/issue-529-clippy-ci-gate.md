# Clippy CI Gate

Closes #529. Documents the clippy enforcement that already exists in `.github/workflows/test.yml` and the local check that mirrors it, since no `CONTRIBUTING.md` exists in this repo yet (see `README.md`: "There is no `CONTRIBUTING.md` in this repository at present.").

## What CI actually runs

The `Run clippy` step in `.github/workflows/test.yml` runs, per crate: `cargo clippy -- -D warnings`. `-D warnings` promotes every clippy warning to a hard error, so a PR with any lint issue fails the `Test Contracts` job. This already satisfies "clippy warnings promoted to errors" — the gap this doc closes is that it wasn't written down anywhere.

## Coverage: which crates are actually gated

The step `cd`s into each crate individually — not a workspace-wide `--workspace` call. It covers exactly 16 of the 24 crates under `contracts/`: `ephemeral_account, sweep_controller, reserve_contract, timelock_controller, multisig_approval, fee_splitter, nonce_registry, allowlist_registry, audit_log, version_registry, asset_allowlist, access_controller, compliance_oracle, claimable_balance_registry, expiry_scheduler, notification_registry`.

**Not covered** (no `cd contracts/<name>` block in `test.yml` — tests, fmt, clippy, and build all silently skip them): `account_factory`, `batch_sweep_queue`, `escrow_vault`, `fee_sponsor`, `metrics_aggregator`, `pause_guardian`, `rate_limiter`, `recovery_registry`. A PR touching only these crates gets no CI signal.

## Running it locally before opening a PR

From repo root, for any crate under `contracts/`:

```sh
cd contracts/<crate_name>
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test --verbose
cargo build --target wasm32v1-none --release
```

Same sequence and order `test.yml` runs (format, clippy, test, build), so a clean local run for a crate predicts a clean CI run for it.

## Recommendation

Extend the four workflow steps' `cd` lists to include the 8 uncovered crates above, so every crate under `contracts/` gets the same fmt/clippy/test/build gate.
