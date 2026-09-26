# Integration Test Harness Notes: Testing Against Live Testnet

## Overview

`scripts/test.sh` runs `cargo test` for all four contracts against an **in-process Soroban test environment** — no network, no deployed contracts, no fees. That is the right tool for contract logic and the wrong tool for anything that depends on a real deployment.

This document is for teams writing **their own** integration suite against the live testnet deployment: what latency to budget, how to manage test accounts, and — the hard part — how to stop your tests from interfering with each other or with other integrators on a shared deployment.

> **Scope.** This is guidance for a *consuming* project's test suite. Nothing here changes `scripts/test.sh`, and nothing here adds a sandbox entrypoint; the repo has no local Soroban sandbox, which is itself a documented gap (see [Why Not a Sandbox](#why-not-a-sandbox)).

---

## What Integration Tests Actually Catch

Worth being explicit about the split, because it determines what belongs in which suite.

| Concern | `cargo test` (in-process) | Live-testnet integration |
|---|---|---|
| State machine and error codes | ✅ | redundant |
| Ed25519 verification logic | ✅ | redundant |
| Reentrancy ordering | ✅ | redundant |
| Nonce desync from concurrent submitters | ❌ | ✅ **only here** |
| Deploy script / WASM hash correctness | ❌ | ✅ |
| Contract actually exists at the expected ID | ❌ | ✅ |
| Wasm-vs-spec interface drift | ❌ | ✅ |
| RPC availability, latency, rate limits | ❌ | ✅ |
| SEP-41 token behaviour against a real token | partially | ✅ |
| Balance conservation with real transfers | ❌ | ✅ |

If your test asserts an error code, it belongs in `cargo test`. If it asserts that the shared testnet deployment is in a state you can use, it belongs here.

---

## Latency: Budget It Properly

Testnet is not a local sandbox. Rough expectations against `soroban-testnet.stellar.org`:

| Operation | Typical wall time |
|---|---|
| Ledger close | ~5 s |
| Single contract invoke (submitted → confirmed) | 5–15 s, occasionally 30 s+ |
| `simulateTransaction` RPC round trip | 200 ms – 2 s |
| `getEvents` over a wide ledger range | 1 – 10 s (scales with range) |
| Friendbot funding | 1 – 5 s, rate-limited (see below) |

### Consequences for suite design

1. **Budget 15 s per state-changing step, not 2 s.** A suite that assumes fast testnet will flake on a bad day and train you to ignore flakes.
2. **Sequence the suite, do not parallelise state transitions.** A test that creates an account and the next test that funds it cannot run concurrently. Serial execution is a feature here.
3. **Read-only calls may be batched or parallelised.** `get_info`, `is_expired`, `get_nonce` are safe to fan out.
4. **Prefer `simulateTransaction` over a submitted transaction for pre-flight checks.** `simulate_sweep()` exists precisely for this — it reports what a sweep would move and any blocking error without changing state, at a fraction of the latency.
5. **Never use a fixed sleep as a synchronisation primitive.** Poll for the condition with a timeout.
6. **Expect testnet to be slow at ledger boundaries.** Ingestion pauses briefly around close; a submission that lands then may take a full extra cycle.

```typescript
// Budget the wait on the condition, not on a guess.
async function waitFor(
  predicate: () => Promise<boolean>,
  { timeoutMs = 30_000, intervalMs = 2_000, label = 'condition' } = {},
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (await predicate()) return;
    await sleep(intervalMs);
  }
  throw new Error(`timed out after ${timeoutMs}ms waiting for ${label}`);
}

// Never: await sleep(5000)
// Always: await waitFor(() => isExpired(account), { label: 'expiry' });
```

---

## Test Account Funding and Cleanup Strategy

### Funding is the bottleneck, not contract calls

Friendbot is rate-limited per address. A suite that creates a fresh Stellar account per test will spend most of its wall time being throttled, not testing. The practical guidance in `testnet/config/friendbot.md` (funding patterns, and the limits that force them) is the reference here.

Three strategies, in increasing order of preference:

#### 1. Reuse a small pool of funded accounts (recommended)

Create **N** accounts once per suite run — say 3–5 — and reuse them across tests. Each holds enough XLM to act as a fee payer and hold token balances.

```
+ setup:    create 4 funded accounts       (~4 Friendbot calls, one time)
+ per test: reuse from the pool            (0 Friendbot calls)
```

#### 2. Reuse across runs by persisting keys

Write pool accounts to a gitignored file (or CI secret store) and reuse them on subsequent runs. Eliminates Friendbot from the hot path entirely.

**Never commit testnet secret keys.** Testnet XLM is worthless, but a committed key trains the wrong habit, gets picked up by secret scanners, and — once a key *is* reused on mainnet — becomes a real incident.

#### 3. Fresh account per test (only for isolation-critical tests)

Accept the latency. Restrict this to tests that genuinely need a virgin account (expiry, one-shot recovery), and cap the count.

### `EphemeralAccount` instances are per-test regardless

The funded *Stellar* account is the reusable resource. The `EphemeralAccount` **contract instance** is not — it is single-use by design:

- One payment per asset; a second `record_payment` for the same asset returns `DuplicateAsset` (13).
- One sweep, ever: after `Swept`, further sweeps fail with `AlreadySwept` (7).
- `expiry_ledger` and `recovery_address` are fixed at `initialize()` and cannot be changed.
- `record_payment()` has no status or expiry guard, so you *can* pollute an expired account — but then it can never be swept, so do not.

**Every test that needs a sweepable account needs a freshly deployed instance.** This is the main cost driver in a live-testnet suite. If you deploy many, `AccountFactory::batch_initialize` is the intended path — with the caveat in [Caveats](#caveats-worth-knowing) below.

### Cleanup

There is no close, no revoke, no "delete". Options:

| Approach | When |
|---|---|
| Let accounts expire | Accounts created with a short `expiry_ledger` self-retire the state machine. They still hold balances and still cost reserve. |
| `expire()` at teardown | Flips status to `Expired`, one-shot. Bookkeeping only — **does not return tokens** (see below). |
| Leave them | Acceptable on testnet. Set a far-future expiry so nothing expires mid-debug. |
| Sweep to a burn destination | Only meaningful for accounts that are actually sweepable. |

**Cleanup does not reclaim anything.** `expire()` and `recover()` set state and emit `AccountExpired`; `ephemeral_account` contains no `TokenClient::transfer()` call, so no tokens move to the `recovery_address`. Reserve reclaim is a counter, not a transfer. "Cleaning up" is tidying your own test state, not returning funds. Plan for that — if your assertions depend on a swept-away balance, that will not happen on the expiry path.

---

## Not Interfering With Shared Testnet State

This is the part that makes live-testnet integration testing a community problem rather than a purely local one. **The testnet deployment is shared.** Other integrators, this repo's own documentation examples, and other people's CI are all hitting the same contracts at the same time.

### The single global nonce is the main hazard

`SweepController` keeps **one** nonce for **all** accounts it controls. Every successful `execute_sweep()` or `claim()` increments it. So:

```
Your test reads nonce = 12
Someone else's sweep lands
You sign over nonce 12
You submit → traps in ed25519_verify
```

This is not a rare race — on a busy shared controller it is routine. See `testnet/config/nonce-tracking.md` for the mechanics and `testnet/security/known-testnet-abuse-patterns.md` for the abuse framing.

**Rules that follow from it:**

1. **Read `get_nonce()` immediately before signing**, never cache it across a test step.
2. **Serialise signing per controller** in your suite. One in-flight sweep at a time.
3. **Treat an `ed25519_verify` trap as retryable**, not as a hard failure. Re-read the nonce and retry.
4. **Bound the retries.** A tight retry loop against a shared controller is itself the abuse pattern. Cap at 2–3 attempts with backoff, then fail with a message pointing at nonce contention rather than looping.
5. **Log the nonce you used** on every sweep, so a failure is diagnosable afterwards.

### Who you must not break

| Shared resource | How to avoid breaking it |
|---|---|
| `SweepController` nonce | Serialise; re-read before each signature |
| Locked `authorized_destination` | If the shared controller is in locked mode, sweep **only** to that address. Check before assuming flexible mode. |
| The `authorized_signer` key | You do not have it. If sweeps fail with a trap and your digest is right, the signer key may have rotated — not your bug. |
| `EphemeralAccount::upgrade()` admin | Never call `upgrade()` against the shared deployment. Not "carefully", not "just once". |
| `SweepController::update_authorized_destination()` | Creator-gated and changes routing for **every** account on that controller. Treat as forbidden. |
| Long-lived demo accounts | `testnet/registry/known-test-accounts.md` accounts are never to be swept or expired. |
| Test tokens' balances | Other integrators use the same SEP-41 tokens. Do not assume a token's supply or any account's balance. |

The broad version: **your test suite may create its own accounts, but it may not change shared configuration.** Everything that mutates controller-level or contract-level config is off-limits against the shared deployment.

### Naming everything

Because you cannot see other integrators' transactions in a useful way, make yours identifiable:

```typescript
const SUITE = 'myorg-sdk-it';           // your team
const RUN   = process.env.CI_RUN_ID ?? String(Date.now());
const label = (what: string) => `${SUITE}/${RUN}/${what}`;  // → recovery_address
```

Recovery addresses and `authorized_controller` values are not settable after `initialize()`, so the label has to be correct on the first try. Put the run id in the recovery address, and you can identify your accounts later from on-chain state alone.

### Detect cross-contamination

```typescript
// If the controller is in locked mode, every sweep must target one address.
// Assert against the address you expect rather than assuming flexibility.
const destination = await resolveAuthorizedDestination(controllerId);
```

There is **no getter** for `authorized_destination` or `authorized_signer` on `SweepController` — you infer them from `execute_sweep` behaviour (`UnauthorizedDestination`) and from `deployments/testnet.json`. If your suite assumes flexible mode and it is actually locked, every test fails with `UnauthorizedDestination` and it will look like a bug in your code. Check this first when a whole suite fails identically.

### Suite-level etiquette

- Prefer a **dedicated** `EphemeralAccount` per test; never share a sweepable account across tests.
- Read-only tests can run anywhere, any time.
- Avoid submitting sweeps in a tight loop. A handful per run is polite; hundreds is a nuisance and can starve other integrators.
- Prefer a locally-run `cargo test` for anything you *can* test there. Reserve live testnet for what genuinely needs a real deployment — it is a shared, rate-limited, occasionally unreliable resource.

---

## Assertions Worth Making

A live-testnet suite earns its keep on assertions an in-process test cannot make:

```typescript
// 1. The deployment exists and responds
await expect(contract.get_status()).resolves.toBeDefined();

// 2. The WASM you expect is what's deployed
//    (guards against a silent upgrade — see testnet/registry/upgrade-history.md)
expect(await liveWasmHash(controllerId)).toBe(EXPECTED_WASM_HASH);

// 3. State machine holds on a real deployment
const acct = await freshEphemeralAccount({ expiryLedgers: 60 });
await expect(acct.sweep(...)).rejects.toThrow(/NoPaymentReceived/);

// 4. One payment per asset, on-chain
await acct.record_payment(100n, USDC);
await expect(acct.record_payment(50n, USDC)).rejects.toThrow(/DuplicateAsset/);

// 5. Expiry is ledger-based and inclusive
expect(await acct.is_expired()).toBe(false);
await waitFor(() => acct.is_expired(), { label: 'expiry' });
await expect(acct.sweep(...)).rejects.toThrow(/AccountExpired/);

// 6. Funds actually moved (the assertion in-process tests CANNOT make)
const before = await token.balanceOf(destination);
await sweep(...);
expect(await token.balanceOf(destination)).toBe(before + EXPECTED);
```

Assertion 6 is the highest-value one in the list. `SweepController` is the only path that calls SEP-41 `transfer()`; if balances do not move, nothing moved.

---

## Sizing the Suite

A pragmatic live-testnet suite is **small and targeted** — everything that cannot be proven in-process, and nothing else.

| Suite layer | Tests | Runtime target |
|---|---|---|
| Contract logic (in-process, `cargo test`) | everything else | seconds |
| Live smoke: deployment reachable, WASM hash matches | 1 test | ~15 s |
| Happy-path sweep, real transfers | 1–2 tests | ~60 s |
| Expiry path | 1 test | ~5 min (ledger-bound) |
| Concurrency / nonce contention | 1 test | ~30 s |

If your live suite is taking more than a few minutes, most of what it is doing belongs in `cargo test`. Exception: expiry tests are inherently ledger-bound — a ~5-minute expiry is about 60 ledgers, and there is no way to skip real ledgers against a live network.

---

## Handling Flakiness

| Symptom | Likely cause | Response |
|---|---|---|
| `ed25519_verify` trap, correct digest | Nonce contention on the shared controller | Re-read nonce, retry (bounded) |
| Whole suite fails with `UnauthorizedDestination` | Controller is in locked mode | Assert against the locked address |
| `AccountExpired` before you expected | Ledger arithmetic; `is_expired()` is `>=` | Widen the margin; recompute from a fresh ledger read |
| Timeout waiting for confirmation | Testnet lag or ingestion pause | Raise the timeout; do not shrink it |
| Friendbot throttled | Too many fresh accounts per run | Switch to a reused account pool |
| `TransferFailed` | One asset lacks balance; the whole sweep reverts | Check every recorded asset's balance |
| Works locally, fails on testnet | Missing `require_auth` signer | Confirm which identity actually signed |

Distinguish *flaky* from *wrong* before adding a retry. A retry that makes a genuinely broken test pass is worse than a failure.

---

## Caveats Worth Knowing

- **`AccountFactory` swallows per-account error detail.** `batch_initialize` returns `success: false, error: None`, so a partial failure tells you *which* address failed but not why. Diagnose each failed address by hand — `testnet/runbooks/account-factory-batch-failure.md`.
- **`AccountFactory` is not built by `scripts/build.sh`**, is not in CI, and is not deployed by `scripts/deploy-testnet.sh`. Confirm it is actually live before building a suite around it — `testnet/registry/account-factory-status.md`.
- **`scripts/deploy-testnet.sh` is currently broken.** It references an unassigned `RESERVE_CONTRACT_ID` under `set -euo pipefail` and aborts. If your suite expects a fresh testnet deployment to exist, it does not.
- **No local sandbox exists.** `soroban sandbox` / `stellar sandbox` is referenced in `testnet/config/expiry-ledger-testing.md` as an option, but the repo provides no entrypoint for it. Until one lands, ledger-bound tests genuinely take real time.
- **`SweepController` has no getters** for `creator`, `authorized_signer`, or `authorized_destination`. Infer them, or read `deployments/testnet.json` — and treat that file as a record, not a guarantee. Verify on-chain where you can.
- **Contract IDs change on redeploy.** Pin to what `deployments/testnet.json` says today and re-verify; a redeploy under new IDs will silently break a hardcoded suite.

---

## Why Not a Sandbox

The obvious objection to all of this latency is: why not run a local Soroban sandbox with instant ledgers?

Because there isn't one. The repo has no sandbox entrypoint, no local deployment script, and CI that is entirely commented out (`.github/workflows/test.yml` is inert; `deploy-testnet.yml` validates the build but has its actual deploy steps commented out). So live testnet is currently the only environment that reflects a real deployment.

That is a real gap, and closing it would remove most of what is painful in this document. Until then, the mitigation is the one above: keep the live suite small, keep the rest in `cargo test`, and be a good citizen of a shared deployment.

---

## Related Documentation

- `testnet/config/nonce-tracking.md` — the shared nonce, and why it is the main contention point
- `testnet/config/expiry-ledger-testing.md` — computing short expiries for ledger-bound tests
- `testnet/config/soroban-cli-identity-setup.md` — the testnet identities referenced here
- `testnet/config/friendbot.md` — funding patterns and rate limits
- `testnet/security/known-testnet-abuse-patterns.md` — the same hazards, from the griefing angle
- `testnet/registry/known-test-accounts.md` — long-lived accounts your suite must not touch
- `testnet/registry/upgrade-history.md` — correlating behaviour changes to upgrades
- `testnet/registry/contract-status.md` — deployment status and contract IDs
- `testnet/examples/` — worked examples of the flows under test
- `docs/testing.md` — the in-process test suite these notes complement

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-26 | Initial integration test harness guidance for live testnet |
