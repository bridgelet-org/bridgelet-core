# Worked Example: What Happens If You Submit the Same Sweep Twice

## Overview

Retry logic on `SweepController` is not a safety net, it is a second source of
bugs. This document works through what a duplicate submission actually does,
why the answer is completely different for `execute_sweep` than for `claim`,
and what the only safe retry actually is.

The short version:

| What you resubmitted | What happens | What to do instead |
|---|---|---|
| The **same `execute_sweep` with the same signature** | The signature is bound to a nonce that is now **consumed**. Verification fails and **traps** — no error code | Do not retry. Read state and reconcile |
| A **re-signed `execute_sweep` with the old nonce** | Fails the same way, and now you have burned a fee proving it | Do not retry. Re-read `get_nonce()` |
| A **re-signed `execute_sweep` with a fresh nonce** | Verifies, but the account is already `Swept`, so the sweep fails on state | Do not retry. Read `get_info` first |
| A **duplicate `claim`** | No signature involved at all. It is a **state** question, not a signature question | Do not retry. `can_sweep` is already `false` |

Only one of those is ever a no-op, and it is not a retry — it is a state read.

## Status

**Reasoned from source. Not observed.** See
[Why this is not an observation log](#why-this-is-not-an-observation-log)
below — the short version is that there is nothing deployed to observe.

---

## Why this is not an observation log

The issue for this document asks for behaviour "from observed testnet
behavior". That cannot be supplied honestly.
`testnet/registry/contract-status.md` records all four contracts as
**undeployed** (❌ No, contract ID "TBD", `Last Verified: Never`), dated
2026-09-24, and `testnet/security/README.md` says the IDs in
`deployments/testnet.json` must be treated as unverified until someone confirms
them on-chain. `testnet/docs/getting-started.md` does carry a line claiming its
script "was run successfully against testnet on 2026-09-23", which contradicts
the status registry; the registry is the canonical record and nothing here
should be read as confirming the other.

So instead of a fabricated observation, every claim below is **derived from
source and is checkable**: the file, the function, and the line of reasoning
are given so you can disagree with it. The honest upgrade path is in
[What would confirm this](#what-would-confirm-this).

---

## The payload a signature covers

From `contracts/sweep_controller/src/authorization.rs`,
`construct_sweep_message()`:

```rust
let mut message = soroban_sdk::Bytes::new(env);

let dest_bytes = destination.to_xdr(env);
message.append(&dest_bytes);

message.push_back(((nonce >> 56) & 0xFF) as u8);
message.push_back(((nonce >> 48) & 0xFF) as u8);
message.push_back(((nonce >> 40) & 0xFF) as u8);
message.push_back(((nonce >> 32) & 0xFF) as u8);
message.push_back(((nonce >> 24) & 0xFF) as u8);
message.push_back(((nonce >> 16) & 0xFF) as u8);
message.push_back(((nonce >> 8) & 0xFF) as u8);
message.push_back((nonce & 0xFF) as u8);

let contract_bytes = contract_id.to_xdr(env);
message.append(&contract_bytes);

env.crypto().sha256(&message).into()
```

```text
SHA256(
  destination.to_xdr()                     ← Soroban Address XDR
  ‖ nonce as an unsigned u64, 8 bytes BE    ← (nonce >> 56) down to (nonce >> 0)
  ‖ controller_id.to_xdr()                 ← env.current_contract_address()
)
```

Two properties of this payload drive everything that follows.

**1. The nonce is 8-byte big-endian, pushed byte-by-byte from the high shift
down.** It is not little-endian and it is not a decimal string. Getting this
wrong produces a signature that *never* verifies, with no diagnostic pointing at
the encoding — which is the failure mode `docs/SIGNATURE_FORMAT.md` and
`testnet/tooling/nonce-inspector-spec.md` both warn about. Use
`soroban_sdk::Address::to_xdr()` itself rather than a hand-rolled XDR encoder;
`tools/sweep-signer` does exactly that, and the tool is its own workspace
pinned to `soroban-sdk = "22.0.0"` **specifically so `Address::to_xdr()`
serialises identically** to what the contract computes.

**2. The ephemeral account address is not in the digest.** The payload is
`(destination, nonce, controller)` and nothing else. A signature is therefore
not bound to a particular account. `verify_sweep_auth` even takes the account as
a parameter and ignores it — the signature is `_account: &Address` in the
function signature. This is a real design property, and it is what makes the
nonce, and only the nonce, the replay boundary.

---

## The nonce is global, and it is consumed

`SweepController::get_nonce(env) -> u64` returns **one** value for the whole
controller deployment:

- It is set to `0` in `initialize()` (`storage::init_sweep_nonce`).
- It is incremented by `authorization::increment_nonce(env)`, which calls
  `storage::increment_sweep_nonce`.
- In `sweep_account()` that increment is the **first** thing that happens when
  `increment_nonce` is true, before the account is called and before any
  transfer:

```rust
fn sweep_account(env, ephemeral_account, destination, auth_signature, increment_nonce)
    -> Result<(), Error>
{
    if increment_nonce {
        // Increment nonce after successful verification to prevent replay attacks.
        authorization::increment_nonce(env);
    }
    Self::authorize_ephemeral_sweep(env, &ephemeral_account, &destination, &auth_signature);
    let account_client = EphemeralAccountClient::new(env, &ephemeral_account);
    account_client.sweep(&destination, &auth_signature);
    // ...
}
```

`execute_sweep` passes `increment_nonce = true`. `claim` does **not** go through
`sweep_account` at all — it calls `authorize_claim` directly, so
**`claim` does not increment the nonce**, even though the doc comment on
`get_nonce` and `testnet/config/nonce-tracking.md` both say the nonce
increments after every successful `execute_sweep()` **or** `claim()`.

> **Discrepancy found in the repository.** The `get_nonce` doc comment
> (`contracts/sweep_controller/src/lib.rs`) and
> `testnet/config/nonce-tracking.md` both state the nonce "increments by 1
> after every successful `execute_sweep()`/`claim()` call". Reading
> `sweep_account`'s only caller and `claim`'s body, `claim` never reaches the
> increment. This document follows the **code**, and flags the comment. It is
> a contract-behaviour question and needs a live test to settle — see
> [What would confirm this](#what-would-confirm-this). Do not build a
> "count the sweeps" reconciliation on either statement until then.

Two consequences follow directly:

- **The nonce is the only replay boundary.** Because the account is not in the
  digest, a signature is valid for *any* destination-sweep on that controller
  at that nonce, and invalid the moment the nonce moves. Replay protection is
  global, not per account.
- **A consumed nonce invalidates the signature retroactively.** The digest is
  recomputed from the *current* on-chain nonce at verification time, not from
  anything the signature carries. Once the nonce advances, the old signature is
  a signature over bytes that no longer exist.

The storage getter also has a subtlety worth knowing before you trust a `0`:

```rust
pub fn get_sweep_nonce(env: &Env) -> u64 {
    env.storage().instance().get(&DataKey::SweepNonce).unwrap_or(0u64)
}
```

An absent key returns `0`. So `0` is both "freshly initialized" and "never
initialized", and a successful `0` is not proof the controller was set up.
`testnet/runbooks/nonce-desync.md` makes the same point.

---

## What a duplicate `execute_sweep` actually does

Take a concrete sequence. The controller nonce is `7`.

```text
  t0  read get_nonce()                  → 7
  t1  sign SHA256(dest ‖ 7 ‖ controller) → SIG_7
  t2  submit execute_sweep(account, dest, SIG_7)
  t3  ... the response never arrives. Dropped connection. Timeout.
```

**You do not know whether `t2` committed.** That is the only genuinely
dangerous moment, and it is a transport problem, not a contract one. Note that
`testnet/examples/README.md` already says the right thing about transfers: if
the result is uncertain, **preserve the transaction hash and inspect state
instead of retrying blindly**. This document is the same rule for sweeps.

Now the three responses:

### Resubmitting `SIG_7` — the same signature

The nonce is now `8` (or was already `8`, if somebody else beat you). The
contract recomputes the digest with `8`, compares against `SIG_7`, and
`env.crypto().ed25519_verify(...)` **traps**. The transaction aborts.

There is **no error code**. `SweepController::Error::SignatureVerificationFailed`
is `9` and is **never constructed anywhere in the crate** — `verify_sweep_auth`
calls `ed25519_verify` and returns `Ok(())`; there is no `Err` arm to convert.
See [`error-handling.md`](error-handling.md) for the full reachability table.
What you see is a bare host-function failure with no diagnostic, and it is
indistinguishable from a genuinely bad signature. The correct response is to
re-read `get_nonce()` and compare — not to conclude that your signing is broken.

### Re-signing with the **old** nonce — the dangerous retry

```bash
# WRONG. Re-signing with the nonce you already used.
cargo run --manifest-path tools/sweep-signer/Cargo.toml -- sign \
  --contract-id "$SWEEP_CONTROLLER_ID" \
  --destination "$DESTINATION" \
  --nonce 7 \
  --signer-seed-hex "$SWEEP_SIGNING_KEY_SEED"
```

This produces a *valid* signature — over a nonce that is no longer current. It
will not verify, and you have just paid an inclusion fee to learn that. This is
the specific mistake this document exists to prevent: a "retry" that re-signs
with a value the caller cached earlier, rather than re-reading it.

`testnet/runbooks/nonce-desync.md` is blunt about it: *"Do not change the nonce
bytes inside an already signed message and expect the old signature to remain
valid."* Re-signing is not that mistake, but it is the same mistake wearing a
different hat — the signature is built from a value that is not current.

### Re-signing with a **fresh** nonce — also not a retry

```bash
NONCE=$(stellar contract invoke --simulate-only \
  --id "$SWEEP_CONTROLLER_ID" --network "$NETWORK" --source "$READER" \
  -- get_nonce)
```

This signature **will** verify, because the nonce is current. But if the
original sweep did commit, the account's status is now `Swept`, so
`EphemeralAccount::sweep()` returns `AlreadySwept` (7) and the transaction
aborts. If the original sweep did *not* commit, this is simply the first
attempt, and you should have had no reason to be here.

Either way you spent a fee to discover something a read-only call would have
told you for free. That is the whole argument.

---

## `claim` is a different animal: no signature at all

```rust
pub fn claim(env: Env, recipient: Address, ephemeral_account: Address) -> Result<(), Error>
```

`claim` takes **no `auth_signature` parameter**. The recipient signs a Soroban
auth entry for `claim(recipient, ephemeral_account)`, a relayer submits and pays
the fees, and the controller uses `env.authorize_as_current_contract()` to
satisfy the account's `authorized_controller.require_auth()` check via
`sweep_claim`.

A duplicate `claim` is therefore a **state** question, not a signature question:

| | `execute_sweep` | `claim` |
|---|---|---|
| Takes a signature | yes, `auth_signature: BytesN<64>` | **no** |
| Who signs | an off-chain Ed25519 key (`authorized_signer`) | the recipient, as a Soroban auth entry |
| Duplicate fails at | `ed25519_verify` (a trap) | account state (`AlreadySwept`) |
| Advanceable per attempt | re-signable, and tempting | re-signable, also tempting |

**Do not carry retry advice from one path to the other.** "Re-sign and retry"
is a signature-level move and is meaningless for `claim`; the correct duplicate
handling for `claim` is a state read. The gas-free path is covered in
`testnet/examples/gas-free-claim-walkthrough.md` and
`testnet/config/gas-free-claim-testing.md`.

---

## The decision table

This is the deliverable. Three questions, in order.

| Did I get a result? | Is the state consistent? | What may I safely do next? |
|---|---|---|
| **No** — timeout, dropped connection, unknown | **Do not know yet** | **Resolve by hash first.** Look the transaction up before doing anything else. Do not resubmit, do not re-sign. A timeout releases no lock until the result is resolved (`testnet/runbooks/nonce-desync.md`) |
| No | `get_status == 2` (`Swept`) and `swept_to` is the destination you intended | **Treat as success.** The sweep committed. Verify the destination balance changed; the contract event alone is not proof of value movement |
| No | `get_status == 2` but `swept_to` is a *different* destination | **Escalate.** Do not re-sweep. See `testnet/runbooks/nonce-desync.md` and `testnet/runbooks/expired-account-recovery-check.md` |
| No | `get_status == 1` (`PaymentReceived`) — the sweep did **not** commit | **Safe to proceed**, but as a fresh attempt, not a retry: re-read `get_nonce()`, re-sign, submit — while holding the per-controller lock |
| No | `get_info` errors with code `2` (`NotInitialized`) | **Stop.** Wrong account ID, or the deployment is not what you think. Do not resubmit |
| **Yes**, and it succeeded | anything | **Done.** Confirm all of: final status success, `get_status == 2`, `get_info.swept_to` is the destination, `can_sweep` is now `false`, the nonce advanced, destination balances changed |
| **Yes**, and it failed with an `ed25519_verify` trap | `get_nonce()` has advanced since you signed | **Stale signature.** Discard `SIG`. Re-read `get_nonce()` and re-sign. See `testnet/runbooks/failed-sweep-signature.md` |
| Yes, failed with a trap | `get_nonce()` has **not** advanced | **Not a nonce problem.** The destination, controller ID, signer key, or XDR encoding is wrong. Stop re-signing; verify the other components (`testnet/runbooks/nonce-desync.md`, step 5) |
| Yes, failed with `AlreadySwept` (7) | `get_status == 2` | **The sweep already happened.** Reconcile and treat as success; do not resubmit |
| Yes, failed with `AccountExpired` (11) | `is_expired() == true` | **Terminal.** Route to recovery. `testnet/runbooks/expired-account-recovery-check.md` |
| Yes, failed with `UnauthorizedDestination` (13) | controller is in locked mode | **Fix the destination and re-sign.** The digest binds the destination, so the old signature was never going to work for the right one either |

Two rules that hold across every row:

1. **Never resubmit the same signature.** It is bound to a consumed nonce.
2. **Never re-sign with a nonce you did not just read.** Read
   `SweepController::get_nonce()` from the chain, immediately before signing,
   while holding the per-controller lock.

---

## The safe retry loop

What "safe" means here: every attempt re-reads authoritative state, holds a
per-controller lock for the whole read-sign-submit-resolve cycle, and never
reuses a signature. The seed is supplied by environment variable so it never
appears in a transcript — `tools/sweep-signer` reads `SWEEP_SIGNING_KEY_SEED`
for `--signer-seed-hex` and `AUTHORIZED_SIGNER_SECRET` for `--signer-secret`,
and its arg group is `required = true, multiple = false`, so exactly one must
be present.

> **No secrets, ever.** This loop is written so that the key material exists
> only in the environment. Do not paste a seed into a shell you are sharing, do
> not add `--signer-seed-hex "$SEED"` where `SEED` is a literal, and do not echo
> a full signature into a ticket — a signature is a bearer artefact for one
> nonce. `testnet/config/README.md` sets out the discipline.

```bash
export NETWORK=testnet
export RPC_URL="https://soroban-testnet.stellar.org"
export READER="<funded-testnet-identity>"
export RELAYER="<sweep-submitter>"
export SWEEP_CONTROLLER_ID="<verified-sweep-controller-id>"
export EPHEMERAL_ACCOUNT_ID="<verified-ephemeral-account-id>"
export SWEEP_DESTINATION="<canary-destination-address>"

# The key lives in the environment only, never in this file or a transcript.
# export SWEEP_SIGNING_KEY_SEED=...   # 64 hex chars = 32 raw Ed25519 seed bytes

signer() {
  cargo run --quiet --manifest-path tools/sweep-signer/Cargo.toml -- sign \
    --contract-id "$SWEEP_CONTROLLER_ID" \
    --destination "$SWEEP_DESTINATION" \
    --nonce "$1"
}

# One attempt: read the authoritative nonce, sign it, submit, resolve by hash.
# Caller must already hold the per-controller lock.
submit_once() {
  local nonce sig
  nonce="$(stellar contract invoke --simulate-only \
    --id "$SWEEP_CONTROLLER_ID" --network "$NETWORK" --source "$READER" \
    -- get_nonce)" || return 1
  [ -n "$nonce" ] || return 1
  echo ">> nonce read from chain: $nonce"

  sig="$(signer "$nonce" | awk '/^auth_signature/ {print $NF}')"
  [ ${#sig} -eq 128 ] || return 1   # 64 bytes, hex
  echo ">> signed fresh nonce $nonce"

  # This is the real submission: it builds, signs and sends.
  # Add --simulate-only here first if you want a free pre-flight pass
  # before paying a fee — see checking-account-status.md.
  stellar contract invoke \
    --id "$SWEEP_CONTROLLER_ID" --network "$NETWORK" --source "$RELAYER" \
    -- execute_sweep \
    --ephemeral_account "$EPHEMERAL_ACCOUNT_ID" \
    --destination "$SWEEP_DESTINATION" \
    --auth_signature "$sig"
}
```

The driver that wraps it, with the decision table turned into code:

```bash
# Bounded attempts. Each iteration re-reads the nonce; none of them reuses a
# signature. Stop as soon as the account reaches a terminal state.
for attempt in 1 2 3; do
  status="$(stellar contract invoke --simulate-only \
    --id "$EPHEMERAL_ACCOUNT_ID" --network "$NETWORK" --source "$READER" \
    -- get_status 2>/dev/null | tr -dc '0-9')"

  case "$status" in
    2) echo "already Swept — reconcile swept_to and balances; do not resubmit"; break ;;
    3) echo "Expired — route to the recovery runbook; do not resubmit";        break ;;
    1) echo "attempt $attempt: PaymentReceived, proceeding" ;;
    0) echo "no payment recorded — record one first; do not submit";           break ;;
    *) echo "get_status failed or returned an error — stop and investigate";   break ;;
  esac

  submit_once || { echo "submit ambiguous — resolve by transaction hash"; break; }
  sleep 5
done
```

Three properties this loop has on purpose:

- **`get_nonce` is read inside the attempt, not before the loop.** A cached
  value is the bug.
- **The signature is produced and consumed inside one function**, so it cannot
  outlive the nonce it was built for.
- **The state check comes first, every iteration**, so a duplicate that
  already committed short-circuits to "reconcile" rather than to a fee.

If you run this against a shared controller, hold a per-controller lock
around `submit_once` end to end, including the result resolution.
`testnet/tooling/nonce-inspector-spec.md` specifies a `--rounds`-based
stability check for exactly this, and its CI guidance is: a non-zero exit means
the nonce is moving or the RPC is down, so do not sign.

---

## Replay is not the only idempotency hazard

A duplicate submission is one case. These are the others, and each is verified
against the source.

| Call | Guard | Discriminant | Duplicate behaviour |
|---|---|---|---|
| `EphemeralAccount::sweep` | `if storage::get_status(&env) == AccountStatus::Swept` | `AlreadySwept` = **7** | Terminal. Benign for a duplicate: the sweep already happened, `swept_to` holds the destination |
| `EphemeralAccount::expire` | status `== Swept \|\| == Expired` → `InvalidStatus`; then `!is_expired()` → `NotExpired` | `InvalidStatus` = **12**, `NotExpired` = **6** | `InvalidStatus` is terminal and benign (the one-shot transition is consumed). `NotExpired` means you called too early — **wait, do not retry** |
| `EphemeralAccount::recover` | same two guards, then a creator/recovery check | `InvalidStatus` = **12**, `NotExpired` = **6**, `Unauthorized` = **8** | Same as `expire` |
| `EphemeralAccount::record_payment` | `get_payment(&env, &asset).is_some()`; then `payment_count >= 10` | `DuplicateAsset` = **13**, `TooManyPayments` = **14** | The rule is **one payment per asset, up to 10 distinct assets** — not one payment per account. A duplicate with a *different* asset succeeds |
| `SweepController::update_authorized_destination` | `if nonce > 0` | `AccountAlreadySwept` = **7** | Terminal. Note this is a statement about the **controller's** nonce, not about any one account: one successful sweep on *any* account makes the destination immutable for all of them |
| `EphemeralAccount::reclaim_reserve` | status must be `Swept` or `Expired` | `InvalidStatus` = **12** | **Genuinely idempotent.** Once fully reclaimed it returns `0` and still emits a `ReserveReclaimed` event with `amount: 0`, so event *count* is not a proxy for value moved — filter on `amount` |

Note that `PaymentAlreadyReceived` (`3`) is declared in the enum and **never
constructed**. `record_payment` returns `DuplicateAsset` (13). Do not write a
handler for `3` expecting it to fire — see
[`error-handling.md`](error-handling.md).

> `record_payment` has **no `require_auth()`** in the current source, and
> neither status guard on it. A payment can be recorded on an account that is
> already `Swept` or `Expired`, producing an account that can never be swept.
> `testnet/examples/expiry-and-recovery.md` flags this; if you monitor an
> account you care about, gate on `is_expired()` in your own code.

---

## What would confirm this

Everything above is a derivation. These are the live tests that would turn it
into an observation, in the order they are cheapest to run.

| # | Test | Settles |
|---|---|---|
| 1 | Sweep an account, then resubmit the **identical** `execute_sweep` with the same signature | The exact failure rendering of a consumed nonce — trap text, and whether anything at all is recoverable |
| 2 | `get_nonce()` before and after a `claim` | Whether `claim` increments the nonce. The doc comment and `nonce-tracking.md` say yes; the code path says no |
| 3 | Sweep an account, then call `claim` on the same account | The exact duplicate-`claim` error and whether any nonce movement accompanies it |
| 4 | Re-sign with a stale nonce and submit | Confirms the "dangerous retry" failure mode empirically |
| 5 | `record_payment` twice with the same asset, then with a different asset | Confirms `DuplicateAsset` (13) and that the per-asset, not per-account, rule is what ships |
| 6 | `update_authorized_destination` after any sweep on the controller | Confirms the `AccountAlreadySwept` (7) gate is controller-wide |

Until those run, treat this document as a map of the source, not a log. Record
the results in `testnet/docs/changelog.md` when they exist, following the
entry template there.

---

## Common pitfalls

| Pitfall | Symptom | Why | Fix |
|---|---|---|---|
| Resubmitting the same signature | Bare `ed25519_verify` trap, no code | The nonce is consumed; the digest no longer exists | Do not retry. Read state, re-read `get_nonce()` |
| Re-signing with a cached nonce | Same trap, one fee later | The digest is built from the *current* on-chain nonce | Read `get_nonce()` immediately before signing |
| Assuming `get_nonce() == 0` proves anything | Wrong conclusion about initialization | The storage getter also defaults to `0` when the key is absent | Use `0` as a value, not as a proof |
| Expecting `SignatureVerificationFailed` (9) | Never fires | It is never constructed; `ed25519_verify` traps | Handle the trap; see `error-handling.md` |
| Carrying `execute_sweep` retry advice to `claim` | Pointless re-signing | `claim` takes no signature; duplicates are a state question | Read `can_sweep` / `get_status` instead |
| Encoding the nonce little-endian | Signature never verifies, no diagnostic | The contract pushes bytes from `(nonce >> 56)` down | 8-byte big-endian; use `tools/sweep-signer` |
| Hand-rolling `Address::to_xdr()` | Same silent failure | Subtle encoding differences | Use `soroban_sdk` 22.0.0's own serialisation |
| Releasing the lock on timeout | Two workers sign the same nonce | A timeout resolves nothing | Hold the lock until the result is resolved by hash |
| Counting `sweep` events to derive the nonce | Wrong number | A `sweep` event can come from more than one controller path | `get_nonce()` is the authority (`testnet/runbooks/nonce-desync.md`) |
| Treating a `SweepCompleted` event as proof of value movement | Wrong reconciliation | An event and a balance change are separate facts | Check the destination balance directly |
| Assuming code `3` fires for a duplicate payment | Handler never runs | `DuplicateAsset` (13) is what is returned | Map `13`; treat `3` as unreachable |

---

## Related Documentation

- [`error-handling.md`](error-handling.md) — every discriminant above, with what
  to do about it
- [`checking-account-status.md`](checking-account-status.md) — the read-only
  pre-flight that resolves an uncertain submission for free
- [`signed-sweep-walkthrough.md`](signed-sweep-walkthrough.md) — the primary
  `execute_sweep` path end to end
- [`gas-free-claim-walkthrough.md`](gas-free-claim-walkthrough.md) — the
  recipient-signs, relayer-submits path
- [`README.md`](README.md) — the parent example index and placeholder preamble
- [`docs/SIGNATURE_FORMAT.md`](../../docs/SIGNATURE_FORMAT.md) — the message
  format, and the XDR-encoding warning
- `testnet/config/nonce-tracking.md` — nonce semantics and pitfalls
- `testnet/tooling/nonce-inspector-spec.md` — the proposed read-only nonce
  helper, including a `--check` mode
- [`testnet/runbooks/failed-sweep-signature.md`](../runbooks/failed-sweep-signature.md) —
  a signature that will not verify
- [`testnet/runbooks/nonce-desync.md`](../runbooks/nonce-desync.md) — recovery
  procedures for a desynchronised cache
- [`testnet/runbooks/expired-account-recovery-check.md`](../runbooks/expired-account-recovery-check.md) —
  reconciling an expired account
- `testnet/config/gas-free-claim-testing.md` — exercising the `claim` path
- `tools/sweep-signer/` — the signing CLI, referenced and not modified here
- `testnet/docs/changelog.md` — where a confirmed result should be recorded

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-26 | Initial duplicate-submission analysis for `execute_sweep` and `claim`, derived from source and pending a live test |
