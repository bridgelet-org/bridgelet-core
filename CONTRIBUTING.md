# Contributing to Bridgelet Core

Thank you for contributing to Bridgelet Core. This guide explains how to set up the repository locally, run tests, and submit high-quality pull requests.

## Project Setup

### Requirements
- Rust toolchain (stable)
- `wasm32-unknown-unknown` target
- `cargo` and `rustup`
- `soroban-cli` version `22.0.0`

### Install Dependencies

```bash
rustup toolchain install stable
rustup target add wasm32-unknown-unknown
cargo install --locked soroban-cli --version 22.0.0
```

### Initial Setup

```bash
git clone https://github.com/bridgelet-org/bridgelet-core.git
cd bridgelet-core
```

## Repository Structure

- `contracts/ephemeral_account/` – Ephemeral account contract logic
- `contracts/sweep_controller/` – Sweep controller contract logic
- `contracts/shared/` – Shared contract types
- `contracts/reserve_contract/` – Reserve management contract
- `docs/` – Architecture, API, security, testing, and storage docs

## Running Tests

### Unit Tests

Run all unit tests for the EphemeralAccount contract:

```bash
cd contracts/ephemeral_account
cargo test
```

Run all unit tests for the SweepController contract:

```bash
cd contracts/sweep_controller
cargo test
```

### Integration Tests

Integration tests for the sweep controller are located under `contracts/sweep_controller/tests/integration.rs`.

```bash
cd contracts/sweep_controller
cargo test --test integration
```

### All Workspace Tests

To run tests across the entire workspace:

```bash
cargo test
```

### Formatting and Linting

Apply formatting:

```bash
cargo fmt
```

Check formatting:

```bash
cargo fmt -- --check
```

Run lints:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

## Branching and PRs

### Branch Naming
Use feature branches with clear names, for example:

- `docs/issue-94-97-add-docs`
- `fix/sweep-controller-auth`
- `feat/storage-layout-docs`

### Pull Request Guidelines
- Open one PR per logical change set.
- Reference the related GitHub issues in the PR description.
- Include a short summary of what changed and why.
- Add testing steps and note any workarounds.

### Commit Messages
- Keep commit messages clear and focused.
- Use present tense, e.g. `Add error catalogue docs`.
- Reference issue numbers when appropriate.

## Documentation

Documentation should be updated whenever:
- contracts change behavior
- new storage keys are added
- new error codes are introduced
- test or deployment workflows change

## Issue and Review Workflow

- Assign yourself to issues you are working on.
- Keep the branch focused on the assigned issue(s).
- Rebase or merge from `main` before final review.
- Ensure CI passes before requesting review.
