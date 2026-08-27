# Automated Resource-Fee Regression Tracking in CI

Closes #531.

## Problem

An earlier-proposed `benchmark-contracts.sh` measures Soroban resource cost
(CPU instructions, read/write bytes) at various member/account counts for
contracts like `multisig_approval`, `reserve_contract`, and
`sweep_controller`, but nothing in `.github/workflows/test.yml` runs it or
compares results across PRs, so a hot-path regression can land silently.

## Proposed integration

- Add a job (or a step gated on `paths: contracts/**`) that runs a fast
  subset of `benchmark-contracts.sh` — one representative account/member
  count per contract, not the full sweep — after the existing "Run tests"
  step, using the same `wasm32v1-none` / `soroban-cli 22.0.0` toolchain
  already installed by `test.yml`.
- Persist results as `contracts/<name>/resource-baseline.json`:

  ```json
  {
    "contract": "multisig_approval",
    "scenario": "5-of-9 approve",
    "cpu_instructions": 1820000,
    "read_bytes": 4096,
    "write_bytes": 512,
    "recorded_at": "2026-08-27",
    "commit": "<sha>"
  }
  ```

- CI computes percent delta between the baseline and the PR's measured
  values. A delta beyond a threshold (proposed: 15%) posts a warning in the
  job summary rather than failing the build, since some increases are
  legitimate.

## Baseline update process

A baseline is updated only in the same PR that causes the legitimate
increase, with the PR description stating the reason (e.g. "added event
emission for audit_log integration"). This keeps baseline changes reviewable
alongside the code that justifies them, mirroring the size-budget override
pattern in #530.
