# Local Mock vs Live Testnet for Integration Tests

There is no local sandbox entry point for these contracts today (no
`stellar` quickstart wiring in this repository). `scripts/test.sh` runs
`cargo test` for the contracts in an **in-process Soroban test environment**,
which covers contract logic but is not a drop-in for an integrator's application
tests. This page helps you decide how to test your integration.

> Opinionated guidance from reading the repository; not benchmarked. Speeds are
> qualitative.

## The options

| Option | What it is |
|---|---|
| **A. Local mock** | Your own fake of the contract behaviour (in your language): state machine, errors, events |
| **B. In-process contract tests** | Rust tests with the Soroban test environment (what this repo does); only for Rust consumers |
| **C. Live testnet** | Real deployed contracts, real RPC, real fees |
| **D. Local Soroban network** | A local standalone network you run yourself (not provided by this repo) |

## Tradeoffs

| | Local mock | Live testnet |
|---|---|---|
| Speed | Milliseconds | Seconds per transaction (ledger close ~5 s, plus polling) |
| Determinism | Fully deterministic | Subject to RPC lag, resets, concurrent users, shared state |
| Cost / quotas | None | Needs funded accounts, Friendbot limits (`testnet/faucet/`) |
| Fidelity | **Only as good as your mock.** Easy to drift from the real contract (error numbers, event shapes, state transitions) | Real behaviour, real auth, real events |
| Availability | Always | Depends on testnet and RPC (`testnet/runbooks/rpc-outage.md`) and on contracts actually being deployed (`testnet/registry/contract-status.md`) |
| Catches | Logic bugs in your code | Integration issues: passphrase, fees, sequence numbers, auth entries, indexing lag, upgrades |
| CI suitability | Excellent | Fragile unless isolated and retried |
| Shared state | None | Other users and resets interfere |

## Recommendation: both, layered

1. **Unit and most integration tests against a mock** (fast, deterministic):
   cover your branching on each contract error
   (`testnet/examples/error-handling.md`), your idempotency logic
   (`testnet/integration/idempotency-key-recommendation.md`), and event parsing.
2. **A small live-testnet smoke suite** (a handful of tests) for: create → pay →
   sweep, a rejected locked-mode sweep, expiry/recovery, and event arrival. Run on
   a schedule or before releases, not on every commit.
3. **Contract-drift guard**: run the mock against recorded real behaviour, e.g.
   keep a few fixtures captured from live runs and assert your mock reproduces
   them, so mock and reality don't diverge silently.

## Keeping a mock honest

- Generate the error table from `contracts/*/src/errors.rs` (or compare against it
  in a test) instead of hand-copying numbers.
- Mirror `AccountStatus` transitions: `Active → PaymentReceived → Swept`, or →
  `Expired`.
- Remember the quirks that are easy to mock wrongly: `EphemeralAccount::sweep()` does
  **not** verify its signature, `SweepController` verifies Ed25519 and uses a
  nonce, `claim()` uses Soroban auth entries, the controller does not move tokens
  (the SDK performs the payout), and `AccountFactory::batch_initialize` drops error
  detail. See `testnet/docs/overview.md` and `testnet/security/authorized-controller-confusion.md`.
- Mock **events** too if your code depends on them; a mock that returns state but
  no events hides integration bugs.
- Mark mock-only coverage in your reports so passing mocks are not mistaken for
  live validation.

## Decision guide

| Your situation | Suggestion |
|---|---|
| Early development, rapid iteration | Mock first |
| Pre-release validation | Live smoke suite |
| Contracts not deployed on testnet | Mock only until `contract-status.md` shows verified IDs |
| Many parallel CI jobs | Mock; live suite serialized in one job |
| Rust consumer | Consider B (the Soroban test environment) for contract-level checks |
