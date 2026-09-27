# Worked Example: Pre-Flight Account Status Checks

## Overview

Before submitting a state-changing call against `EphemeralAccount` or
`SweepController`, run three read-only probes and gate on them. This walkthrough
uses the three read-only calls the contracts actually expose:

| Probe | Where it lives | Takes |
|---|---|---|
| `EphemeralAccount::is_expired()` | child account contract | no arguments |
| `EphemeralAccount::get_info()` | child account contract | no arguments |
| `SweepController::can_sweep()` | the controller | the account address |

The point is fee discipline. A transaction that fails on-chain still costs an
inclusion fee plus whatever resource fee the network assessed before the failure
propagated, and on a shared testnet deployment it also consumes a slot other
integrators are competing for. A simulated probe costs nothing and changes
nothing.

> **The insight this document is built on:** two of these three probes return
> *misleading* values for an account that was never initialized, and they fail
> in **different ways**. `is_expired()` returns `false` — the same value a
> healthy account returns. `can_sweep()` does not return a value at all. A
> pre-flight that is just "call two things and look at the booleans" will read
> an uninitialized account as a live one and then pay to find out.

## Status

**Unverified against a live deployment.** As of the last review
(`testnet/registry/contract-status.md`, dated 2026-09-24) all four Bridgelet
contracts are recorded as **undeployed**, with contract IDs "TBD" and
`Last Verified: Never`. `deployments/testnet.json` and
`deployment-artifacts/contract-ids.txt` do contain IDs from a 2026-07-12
deployment, but `testnet/security/README.md` says to treat them as unverified
until someone confirms them on-chain per
`testnet/registry/verify-contract-live.md`.

Everything below is derived by reading `contracts/ephemeral_account/src/lib.rs`,
`contracts/sweep_controller/src/lib.rs` and
`contracts/shared/src/types.rs`. It is a description of what the source
specifies, not an observation log. The command lines are written to the
placeholder convention in [`README.md`](README.md) and will run as-is once a
verified deployment exists.

---

## What each check proves, and what it does not

| Check | Returns for an **initialized** account | Returns for an **uninitialized** instance | Proves | Does **not** prove |
|---|---|---|---|---|
| `is_expired()` | `current_ledger >= expiry_ledger` | `false` | If `true`, the account is past `expiry_ledger` and the sweep paths will reject it | That the account exists. `false` is also what a healthy, initialized, funded account returns |
| `get_status()` | stored `AccountStatus` | `Active` (`0`) | The stored lifecycle state, if you already know the instance is initialized | That the instance is initialized. See the note in [`README.md`](README.md): do not use `0` as proof of initialization |
| `get_info()` | `Ok(AccountInfo)` | `Err(NotInitialized)` — error code `2` | **That the instance is initialized**, plus creator, status, `expiry_ledger`, recovery address, payment count, payments and `swept_to` | Anything about the controller, the nonce, or whether your signature is valid |
| `can_sweep()` | `true` iff a payment is recorded **and** status is `PaymentReceived` **and** not expired | Does not return — the cross-contract call to `get_info()` fails first | The composite readiness answer, from the controller's point of view | That your signature will verify, that the nonce is yours, or that the destination is permitted |
| `get_nonce()` | the controller's `u64` nonce | `0` (the storage getter also defaults to `0` when the key is absent) | Which nonce to sign against **right now** | That the controller is initialized — `0` is both the fresh value and the absent-key value |

### Why the uninitialized case behaves differently for each

This is the part worth internalising, because it is the whole reason the
pre-flight is a *gate* and not two calls.

- **`is_expired()` short-circuits before it looks at anything.**
  `contracts/ephemeral_account/src/lib.rs` opens with
  `if !storage::is_initialized(&env) { return false; }`. There is no `Result`
  in the signature, so an uninitialized instance and a live account are
  indistinguishable from the return value alone.
- **`get_status()` has the same guard**, returning `AccountStatus::Active`,
  which is discriminant `0`. A fresh account and an uninitialized instance are
  also indistinguishable.
- **`get_info()` is the one that tells the truth.** Its signature is
  `pub fn get_info(env: Env) -> Result<AccountInfo, Error>`, and the
  uninitialized path is `return Err(Error::NotInitialized)`. That is why
  [`README.md`](README.md) calls it the strongest account-level probe.
- **`can_sweep()` inherits the error rather than swallowing it.**
  `SweepController::can_sweep` is
  `pub fn can_sweep(env: Env, ephemeral_account: Address) -> bool`, and its body
  calls `account_client.get_info()` through a `contractimport!`-generated
  client before it evaluates the boolean. The generated non-`try_` client method
  unwraps the contract's `Result`; when the child returns `NotInitialized`, the
  call does not produce `false` — it fails the whole invocation. So
  `can_sweep()` on an uninitialized account is a **hard error**, not an
  unfavourable boolean.
  *Whether that failure surfaces to your caller as the propagated child code
  (`Error(Contract, #2)`) or as a host-level conversion error needs a live
  check; the generated client in `soroban-sdk` 22.0.0 unwraps, so do not write
  a handler that depends on which.*

The practical rule: **prove initialization with `get_info()` first**, then use
the cheaper booleans. If `get_info()` errors with code `2`, the account ID is
wrong, the instance was never initialized, or the deployment is not what you
think it is — and none of the other probes will tell you that.

---

## Read-only first: `--simulate-only`

Every probe below is invoked with `--simulate-only`, which runs the invocation
through the RPC simulator and prints the result without building, signing, or
submitting a transaction. This is the flag used throughout this tree
([`README.md`](README.md), `testnet/runbooks/nonce-desync.md`); the underlying
RPC call is `simulateTransaction`, as `testnet/docs/soroban-cli-quickref.md`
describes it.

Two consequences worth stating plainly:

1. A probe costs **no fee** and mutates **nothing**. It does not consume a
   nonce, does not change status, does not extend a TTL.
2. A simulated probe still runs the real code path, so it catches the guards
   that would reject your state-changing call — `AccountExpired`,
   `AlreadySwept`, `NotInitialized` — before you pay for them.

> **Never put a signing seed or secret key in this transcript.** The probes
> below need only an ordinary funded testnet identity for fees, and in
> simulation mode not even that. See `testnet/config/README.md` for the
> no-secrets discipline.

---

## Step 1: the environment

Same preamble as [`README.md`](README.md):

```bash
export NETWORK=testnet
export RPC_URL="https://soroban-testnet.stellar.org"
export READER="<funded-testnet-identity>"

export EPHEMERAL_ACCOUNT_ID="<verified-ephemeral-account-id>"
export SWEEP_CONTROLLER_ID="<verified-sweep-controller-id>"
```

`READER` is a funded testnet identity used only as the transaction source for
simulations. It is not the creator, not the controller, and not the signer.

## Step 2: the cheap time-based check

```bash
stellar contract invoke \
  --simulate-only \
  --id "$EPHEMERAL_ACCOUNT_ID" \
  --network "$NETWORK" \
  --source "$READER" \
  -- \
  is_expired
```

`is_expired` takes **no arguments** — it is `fn is_expired(env: Env) -> bool`.
Do not pass an account address to it; you are already addressing the account
with `--id`.

## Step 3: the initialization proof

```bash
stellar contract invoke \
  --simulate-only \
  --id "$EPHEMERAL_ACCOUNT_ID" \
  --network "$NETWORK" \
  --source "$READER" \
  -- \
  get_info
```

This is the gate. If it returns `Error(Contract, #2)` (`NotInitialized`), stop
and fix the account ID before anything else.

`get_info` also carries the rest of the state you need for a decision:

| Field | Use |
|---|---|
| `status` | `0 Active`, `1 PaymentReceived`, `2 Swept`, `3 Expired` |
| `expiry_ledger` | the ledger you are racing against |
| `payment_count` / `payment_received` | whether a sweep is even possible |
| `swept_to` | set after a sweep or expiry; `None` before |
| `payments` | the assets and amounts a sweep would move |

## Step 4: the composite readiness check

```bash
stellar contract invoke \
  --simulate-only \
  --id "$SWEEP_CONTROLLER_ID" \
  --network "$NETWORK" \
  --source "$READER" \
  -- \
  can_sweep \
  --ephemeral_account "$EPHEMERAL_ACCOUNT_ID"
```

The argument name is the Rust parameter name, `ephemeral_account`
(`pub fn can_sweep(env: Env, ephemeral_account: Address) -> bool`).
> `testnet/docs/soroban-cli-quickref.md` shows this call as
> `can_sweep --account <EPHEMERAL_ADDRESS>`. The `--account` form does not
> match the source's parameter name; use `--ephemeral_account`, as
> [`README.md`](README.md) does.

## Step 5: the nonce you will sign against

```bash
stellar contract invoke \
  --simulate-only \
  --id "$SWEEP_CONTROLLER_ID" \
  --network "$NETWORK" \
  --source "$READER" \
  -- \
  get_nonce
```

Read it immediately before signing, not before the pre-flight. The nonce moves
whenever any sweep succeeds on that controller, including sweeps of other
integrators' accounts. See `testnet/config/nonce-tracking.md`.

## Step 6: the decision

```text
get_info() errors with code 2   → wrong account ID, or never initialized. Stop.
is_expired() == true            → the sweep path will return AccountExpired (11).
                                  Route to the recovery path, do not sweep.
status == 2 (Swept)             → already swept; can_sweep() will be false.
status == 0 (Active)            → no payment recorded; can_sweep() will be false.
status == 1 and not expired     → can_sweep() should be true. Proceed to sign.
```

---

## The expiry boundary is `>=`, not `>`

```rust
// contracts/ephemeral_account/src/lib.rs, is_expired()
let expiry_ledger = storage::get_expiry_ledger(&env);
let current_ledger = env.ledger().sequence();

current_ledger >= expiry_ledger
```

At **exactly** `expiry_ledger`, `is_expired()` already returns `true`. There is
no grace ledger. Testnet closes a ledger roughly every 5 seconds, so an
account with, say, 60 ledgers of headroom has about five minutes.

This is precisely where fee-wasting races happen:

- A pre-flight says `false` in ledger `expiry_ledger - 1`. You assemble, sign,
  and submit; the transaction lands in ledger `expiry_ledger` or later. The
  account is now expired, `sweep()` returns `AccountExpired` (11), and the
  transaction is rejected — and you paid for the attempt.
- The check narrows the window; it does not close it. **Leave a margin.** If
  the sweep is worth doing at all it is worth doing with ledgers to spare, so
  poll `is_expired()` and submit well before the boundary rather than racing
  it. `testnet/config/expiry-ledger-testing.md` covers computing a
  near-future `expiry_ledger` for fast iteration.

The same guard is enforced independently inside `sweep()` and `sweep_claim()`,
so there is no bypass: an expired account cannot be swept through
`execute_sweep` or `claim` either.

---

## `can_sweep()` answers a different question than `is_expired()`

They are not redundant, and they are not two views of one boolean.

| | `is_expired()` | `can_sweep(ephemeral_account)` |
|---|---|---|
| Contract | `EphemeralAccount` | `SweepController` |
| Scope | one account, one clock | one account, *as the controller sees it* |
| Question | "has the ledger passed `expiry_ledger`?" | "would a sweep succeed right now, ignoring the signature?" |
| Inputs | none | the child account address, and the controller's own cross-contract call into it |
| Returns | `bool` | `bool` |

`can_sweep()` is controller-scoped because the controller is the component that
decides whether to submit. It is `true` **iff**:

```rust
info.payment_received
    && info.status as u32 == AccountStatus::PaymentReceived as u32
    && !account_client.is_expired()
```

Note the middle term: an account can have a recorded payment and still be
unsweepable if its status is not `PaymentReceived`. An account that was swept
and then had `record_payment` appended to it can hold payments while sitting in
`Swept`. `is_expired()` will not see any of that; `can_sweep()` will.

### A `true` from `can_sweep()` is not a guarantee of success

Three things can still make your `execute_sweep` fail after `can_sweep()`
returned `true`:

1. **The nonce is global, not per account.** `SweepController::get_nonce()`
   returns one `u64` for the whole controller deployment, shared by every
   account it services. It is initialized to `0` in `initialize()` and
   incremented by one after every successful `execute_sweep()` or `claim()`.
   Another integrator's sweep between your read and your submission advances
   the nonce, and your signature is now built over a stale value. Read
   `get_nonce()` *immediately* before signing, and serialise sign-and-submit
   per controller. See `testnet/config/nonce-tracking.md` and
   `testnet/tooling/nonce-inspector-spec.md`.
2. **A signature is bound to `(destination, nonce, controller)`.** The
   verified digest is
   `SHA256(destination.to_xdr() ‖ nonce as 8-byte big-endian ‖ controller_id.to_xdr())`.
   The account address is not part of it. A signature built for a different
   destination, a different nonce, or a different controller deployment will
   not verify, and the resulting failure is a bare `ed25519_verify` trap with no
   error code to match on. See `docs/SIGNATURE_FORMAT.md` and
   `testnet/runbooks/failed-sweep-signature.md`.
3. **The destination may be locked.** If the controller was initialized with an
   `authorized_destination`, sweeps may only go there, or the call fails with
   `UnauthorizedDestination` (code `13`). There is no on-chain getter for it.

`can_sweep()` is a readiness signal for *account state*. It is silent on all
three of the above.

---

## The fee argument, and its honest limit

A failed Soroban transaction on testnet is not free:

- the **inclusion fee** is set by the transaction and is consumed whether the
  transaction succeeds or fails;
- the **resource fee** is assessed from the simulation, and simulation of a
  failing call still costs the RPC node work even though it costs you no
  inclusion fee;
- on a shared deployment, a failed attempt is also a failed attempt somebody
  else has to pay for in contention terms.

Simulating first is therefore strictly cheaper: the same guard evaluation with
no inclusion fee and no state change. That is the entire argument for the
pre-flight, and it is a real one.

**The honest caveat.** A successful simulation is **not a reservation**. Between
your `simulateTransaction` and your `sendTransaction`, the ledger advances and
state can change — including by someone else's sweep. Simulation *narrows* the
set of failure modes you can hit; it does not eliminate them. In particular:

- A successful simulation of `execute_sweep` does not reserve the nonce.
- A successful simulation of a sweep does not survive the account crossing
  `expiry_ledger` before your transaction is included.
- A `false` from `is_expired()` is a snapshot of one ledger.

Write your integration to expect a small residual failure rate, and route
failures to the runbooks below rather than blind-retrying. The fee-payer
matrix — who pays at each lifecycle step, and where fee sponsorship can shift
it — is in [`docs/fee-model.md`](../../docs/fee-model.md).

---

## When a pre-flight fails: route, do not retry

Retrying a state-changing call that has already been shown to be unsweepable
just buys the same rejection again, at a fee. Route instead:

| Pre-flight result | Likely cause | Go to |
|---|---|---|
| `is_expired()` is `true`; `get_info().status == 3` | ledger passed `expiry_ledger` | [`testnet/runbooks/expired-account-recovery-check.md`](../runbooks/expired-account-recovery-check.md) — verify where funds actually are before concluding anything |
| `is_expired()` is `true` but `status` is `0` or `1` | account will never sweep; recovery path only | [`testnet/runbooks/stuck-ephemeral-account.md`](../runbooks/stuck-ephemeral-account.md) |
| `get_info()` errors with code `2` | wrong account ID, or the instance was never initialized | `testnet/registry/verify-contract-live.md` — confirm the deployment and the ID you are addressing |
| `get_info().status == 2` (`Swept`) | already swept; `swept_to` holds the destination | reconcile against `swept_to` and the destination balance; do not re-sweep |
| `can_sweep()` is `false` with a payment recorded and no expiry | status is not `PaymentReceived` | [`testnet/runbooks/stuck-ephemeral-account.md`](../runbooks/stuck-ephemeral-account.md) |
| `get_nonce()` changed between your read and your submit | concurrent sweep on the shared controller | `testnet/config/nonce-tracking.md` and [`testnet/runbooks/nonce-desync.md`](../runbooks/nonce-desync.md) |
| A signature is rejected after `can_sweep()` was `true` | nonce, destination, controller ID, or signer key | [`testnet/runbooks/failed-sweep-signature.md`](../runbooks/failed-sweep-signature.md) |

In every one of these cases the correct action is to **read state, then decide**
— not to resubmit the same call. `testnet/examples/README.md` makes the same
point for transfers: if the result is uncertain, preserve the transaction hash
and inspect state instead of retrying blindly.

---

## Common pitfalls

| Pitfall | Symptom | Why | Fix |
|---|---|---|---|
| Treating `is_expired() == false` as "the account is fine" | Proceed on a contract you never initialized, then pay for `NotInitialized` (2) | The uninitialized branch returns `false` too | Gate on `get_info()` first |
| Treating `get_status() == 0` as "initialized" | Same trap, one level down | `get_status` returns `Active` for an uninitialized instance | `get_status` is only meaningful *after* `get_info()` succeeds |
| Treating `can_sweep() == false` as "expired" | Wrong runbook | `false` also means no payment, or status not `PaymentReceived` | Read `get_info()` to find out which |
| Passing an address to `is_expired` | Argument error | `is_expired(env)` takes no arguments | Address the account with `--id` |
| Using `--account` for `can_sweep` | Argument error | The parameter is `ephemeral_account` | Use `--ephemeral_account` |
| Probing without `--simulate-only` | Paid a real fee for a read | Default is to build and submit | Always `--simulate-only` for pre-flight |
| Caching the nonce from the pre-flight | Signature rejected | The nonce is global and moves under you | `get_nonce()` immediately before signing, holding a per-controller lock |
| Planning a sweep in the final ledger | `AccountExpired` (11), fee spent | `is_expired()` is `>=` | Leave a margin of ledgers, not seconds |
| Reading `get_nonce() == 0` as "uninitialized" | Wrong conclusion | The storage getter also defaults to `0` when the key is absent | A successful `0` is valid; do not read it as proof either way |

---

## Related Documentation

- [`README.md`](README.md) — the parent example index and placeholder preamble
- [`error-handling.md`](error-handling.md) — mapping every error code these
  probes can produce back to domain terms
- [`idempotency-considerations.md`](idempotency-considerations.md) — what a
  duplicate submission does, and the safe retry
- `testnet/config/nonce-tracking.md` — nonce semantics and the pitfalls of
  tracking the nonce locally
- `testnet/tooling/nonce-inspector-spec.md` — the proposed read-only nonce
  helper, including a `--check` mode for a signature you already hold
- [`testnet/runbooks/failed-sweep-signature.md`](../runbooks/failed-sweep-signature.md) —
  a signature that will not verify
- [`testnet/runbooks/nonce-desync.md`](../runbooks/nonce-desync.md) — local
  cache and chain disagree
- [`testnet/runbooks/expired-account-recovery-check.md`](../runbooks/expired-account-recovery-check.md) —
  reconciling an expired account
- [`testnet/runbooks/stuck-ephemeral-account.md`](../runbooks/stuck-ephemeral-account.md) —
  payment received but unsweepable
- [`testnet/docs/soroban-cli-quickref.md`](../docs/soroban-cli-quickref.md) —
  `simulateTransaction` pre-flight best practice
- [`testnet/registry/contract-status.md`](../registry/contract-status.md) —
  the deployment status this document is written against
- [`docs/api-reference.md`](../../docs/api-reference.md) — abstract interface
  for all three probes
- [`docs/fee-model.md`](../../docs/fee-model.md) — fee-payer matrix per
  lifecycle step

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-26 | Initial pre-flight status-check example using `is_expired`, `get_info` and `can_sweep` |
