# WASM Binary Size Budget CI Check

Closes #530.

## Problem

`.github/workflows/test.yml` builds all 24 contracts under `contracts/*` with
`cargo build --target wasm32v1-none --release` but never inspects the
resulting `.wasm` size. A seemingly small source change (e.g. an added
`require_auth` path or a new event) can silently grow deployed bytecode with
no signal in the PR.

## Proposed check

Add a step after "Build all contracts" in `test.yml` (implementation not
included in this doc — see acceptance criteria) that, per contract:

1. Locates `contracts/<name>/target/wasm32v1-none/release/<name>.wasm`.
2. Compares its byte size against a budget defined in a new
   `contracts/<name>/wasm-budget.toml`:

   ```toml
   # bytes; fails CI if exceeded
   max_size_bytes = 24576
   rationale = "baseline release build + 20% headroom, set 2026-08-27"
   ```

3. Fails the job (non-zero exit) if any contract's WASM exceeds its budget.
4. An intentional override requires bumping `max_size_bytes` in the same PR,
   which keeps the rationale co-located with the change that justified it.

## Historical trend

Each CI run should append `{contract, sha, size_bytes, timestamp}` to a
build artifact (e.g. `wasm-sizes.jsonl`, uploaded via
`actions/upload-artifact@v4` alongside the existing `wasm-contracts`
artifact) so size creep across many small PRs is visible, not just a
single-PR pass/fail.

## Why this stays a doc for now

Any real change to `.github/workflows/test.yml` is a CI behavior change and
is deliberately out of scope for this PR; this document specifies the check
so it can be implemented and reviewed separately.
