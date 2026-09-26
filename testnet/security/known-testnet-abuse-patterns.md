# Known Testnet Abuse Patterns

## Purpose

This is a triage reference for the question: **is the testnet deployment broken, or is someone deliberately messing with shared state?**

The two produce similar symptoms. A sweep that fails with an opaque trap, a factory call that returns `success: false`, an account that will not sweep — each is either a bug, a misconfiguration, or interference by another party, and the observable evidence overlaps.

For each pattern below: what an attacker would actually do, **what the contracts actually do in response** (with the code that determines it), and what you would see. Knowing the expected behaviour turns "testnet seems broken" into a specific hypothesis you can check.

> Everything here is derived from the contract source. Where a pattern turns out **not** to be exploitable, that is stated as explicitly as the exploitable cases — knowing the boundary is as useful as knowing the hole, and it stops this document from being read as alarmist.

---

## Triage Table

Start here. Find the symptom, follow the link.

| Symptom | Most likely cause | Check |
|---|---|---|
| `execute_sweep` traps with no error code, digest verified correct | **Nonce contention** — someone else's sweep advanced the nonce, or your nonce read is stale | [P2](#p2-replay-and-front-running) · `testnet/config/nonce-tracking.md` |
| Your `SweepCompleted` never lands, others' do | **Nonce churn** by a holder of the sweep-signing key | [P3](#p3-nonce-churn--liveness-denial) |
| `UnauthorizedDestination` on every sweep, for everyone | **Destination repointed** while the nonce was still 0 | [P4](#p4-destination-repointing) |
| A contract's behaviour changed; address unchanged; no event | **Unauthorised `upgrade()`** | [P5](#p5-unauthorised-upgrade) |
| `batch_initialize` returns `success: false` with no error detail | **Index-slot exhaustion** by a previous caller | [P1](#p1-accountfactory-index-slot-exhaustion) |
| Accounts deployed by the factory are not the expected WASM | **`AccountFactory::initialize` is unauthenticated** | [P1b](#p1b-factory-wasm-hash-overwrite-no-auth-required) |
| `TransferFailed` on a sweep where the recorded amounts look right | **Test-token balance poisoning** | [P6](#p6-test-token-balance-poisoning) |
| Your account will not sweep; a payment you never recorded is in `get_info()` | **Unauthenticated `record_payment()` injection** | [P7](#p7-payment-injection--the-one-that-needs-no-credentials-at-all) |
| An account that lapsed cannot be recovered by you | **Someone called permissionless `expire()` first** | [P8](#p8-expiry-griefing) |
| Everything is slow; RPC timing out for unrelated queries | **Resource spam** | [P9](#p9-resource-and-rpc-spam) |
| Someone else's funds could plausibly be at risk | (rare) Requires a leaked signature | [P10](#p10-signature-leakage--the-enabler) |

Two of these — **P7 and P1b** — require **no credentials at all**. No admin key, no signer key, nothing but a funded testnet account, which Friendbot provides for free. They are the ones to suspect first.

---

## P1: AccountFactory index-slot exhaustion

### What the attacker does

Call `batch_initialize` with a long `requests` vector. Each entry deploys one `EphemeralAccount`, so a 500-element batch consumes 500 index-derived deployment slots.

`batch_initialize` requires `creator.require_auth()`, so the attacker needs their own funded account — free on testnet, and they need no role in this deployment at all.

### What the contract actually does

The deployment salt is derived **only from the loop index**:

```rust
let mut salt_bytes = [0u8; 32];
salt_bytes[28..32].copy_from_slice(&(index as u32).to_be_bytes());
let salt = BytesN::from_array(&env, &salt_bytes);
let account_address = env
    .deployer()
    .with_current_contract(salt)
    .deploy_v2(wasm_hash.clone(), ());
```

The salt does not include the caller, the request contents, or any caller-supplied entropy. A deployed address is therefore determined by `(factory, index, wasm_hash, args)` — and **index 0 is index 0 for everyone.**

So the attacker's batch and a victim's batch collide on every overlapping index. The victim's `deploy_v2` returns an error for those indices, and that error is caught and discarded:

```rust
let result = match client.try_initialize(/* ... */) {
    Ok(_) => AccountInitResult { account_address, success: true, error: None },
    Err(_) => AccountInitResult { account_address, success: false, error: None },
    //                                   ^^^^^^^^ detail dropped, on purpose for now
};
```

**Expected behaviour, precisely:**

1. The attacker's transaction **succeeds**. It is a legitimate call from a legitimate funded account.
2. Every one of the victim's indices that overlaps the attacker's range returns `success: false, error: None`.
3. The victim's indices **beyond** the attacker's maximum still deploy successfully. Exhaustion is a prefix, not a total block — an attacker who used 100 indices blocks 0–99, and index 100 still works.
4. The victim gets **no indication of the reason**. `error: None` is the entire diagnostic.
5. No contract event records any of this. `AccountFactory` emits no events at all.

### What you would observe

- `batch_initialize` results with a contiguous run of `success: false` from index 0.
- `batch_initialize_count` reporting the full request count while far fewer accounts exist.
- Deployment transactions for consecutive indices from a `creator` that is not yours.
- Accounts that exist but are permanently un-initialised at those addresses.

### Notes

- `AccountFactory` is currently **not built by `scripts/build.sh`, not in CI, and not deployed** by `scripts/deploy-testnet.sh`. This pattern is a forecast until it is live — see `testnet/registry/account-factory-status.md`.
- Per-account diagnosis when `error` is `None`: `testnet/runbooks/account-factory-batch-failure.md`.
- A salt-collision recovery runbook was written and then **reverted** upstream (`docs: add account-factory salt-collision recovery runbook`, reverted on the `revert-434-...` branch), so there is no such runbook to consult. The index-only salt is the root cause either way, and this section is the current reference.

---

## P1b: Factory WASM-hash overwrite (no auth required)

### What the attacker does

```rust
pub fn initialize(env: Env, ephemeral_account_wasm_hash: BytesN<32>) {
    env.storage().instance().set(&DataKey::EphemeralAccountWasmHash, &ephemeral_account_wasm_hash);
}
```

**There is no `require_auth()` and no one-time guard.** Anyone can call this on a live `AccountFactory` and replace the stored WASM hash. Every subsequent `batch_initialize` then deploys *that* WASM.

No key, no role, no prior access. This is the most severe permissionless issue in the codebase.

### What the contract actually does

- The call succeeds for anyone, at any time, any number of times.
- Because the deployment address derivation includes the WASM hash, the resulting account addresses differ from the honest ones — so this does **not** collide with P1's slots; it is a separate mechanism.
- Every future batch deployment produces a contract at a legitimate-looking `C...` address running code that is not `ephemeral_account`.

**Expected behaviour:** total compromise of the factory's output. An integrator calling `batch_initialize` gets accounts running attacker-chosen code at a plausible address, with no signal that anything is wrong.

### What you would observe

- Deployed accounts that do not respond to `get_status`, or that return unexpected shapes.
- `batch_initialize` succeeding but the resulting accounts not behaving like `EphemeralAccount`.
- An `initialize` transaction on the factory from an unexpected address, at an unexpected time.

### Triage

If the factory is deployed, **do not trust any account it produced** until the stored hash is verified on-chain against `testnet/registry/wasm-hash-reference.md`. This is a contract defect, not a key-hygiene issue — see `testnet/security/admin-key-hygiene.md`.

---

## P2: Replay and front-running

This is the pattern most likely to be *suspected* and least likely to be real. Stating the boundary clearly.

### What an attacker would do

Observe a pending `execute_sweep` transaction in the ledger. It contains the destination and the 64-byte signature in cleartext. Then either replay that signature, or submit a competing sweep using it.

### What the contract actually does

`verify_sweep_auth` reconstructs the digest using the **contract's own current nonce**, not anything supplied by the caller:

```rust
fn construct_sweep_message(env: &Env, destination: &Address, contract_id: &Address) -> BytesN<32> {
    let nonce = storage::get_sweep_nonce(env);      // <-- read from chain, always current
    // destination.to_xdr() || nonce_be_u64 || contract_id.to_xdr(), then SHA-256
}
```

**A replayed signature can never verify.** A signature built over nonce *N* is only ever checked against nonce *N*, and after one successful sweep the contract is at *N+1*. Submitting the spent signature aborts.

**And front-running with a captured signature does not redirect funds.** The digest binds `destination`, `nonce`, and `controller_id`. Changing the destination changes the digest and breaks verification. Replaying someone else's signature sweeps to *their* destination, which is no use to the attacker. There is no "capture and redirect" path.

The attacker submitting a competing garbage signature simply traps and wastes their own fee.

### Expected behaviour, precisely

| Attempt | Result |
|---|---|
| Replay a spent signature | Traps in `ed25519_verify`. Whole tx aborts. No error code. |
| Replay an in-flight signature verbatim | Wins the race **for the original destination** — the original submitter's sweep then traps. Liveness loss, not theft. |
| Modify the destination, reuse the signature | Digest mismatch → traps |
| Forge a signature | Requires the signing key; without it, traps |
| Do any of the above | **Nonce is not consumed.** Increment happens only after successful verification. |

The last row matters: failed attempts leave the nonce untouched, so a failed sweep is safely retryable with a fresh read.

### What you would observe

A transaction that aborts with no contract error, where the digest was constructed correctly. Almost always this, and only this.

### Triage

```bash
stellar contract invoke --id "$SWEEP_CONTROLLER_ID" --network testnet \
  --source testnet-investigator -- get_nonce
```

If the on-chain nonce differs from the one you signed with, it is nonce contention. Full procedure: `testnet/runbooks/failed-sweep-signature.md`.

**Note the shared-nonce amplification:** the testnet controller is one controller for many integrators, so this fires routinely under load and is not evidence of an attack. Treat a single occurrence as normal operation.

---

## P3: Nonce churn — liveness denial

### What the attacker does

Hold the sweep-authorization key and submit valid sweeps in a loop, advancing the shared nonce. Each one requires an ephemeral account with a payment — the attacker can create their own.

### What the contract actually does

- Every successful `execute_sweep`/`claim` increments the single global nonce.
- There is no rate limit, no per-account nonce, and no cooldown.
- Each churn sweep requires a real funded ephemeral account, so it costs real XLM — this is rate-limiting by expense, not by design.

**Expected behaviour:** the nonce climbs steadily while **other people's** `SweepCompleted` events stop appearing. Their in-flight signatures all become stale simultaneously.

**Severity: liveness only.** Sweeps fail; no funds move anywhere they should not. Recoverable the moment the attacker stops or the key is rotated — and note that `authorized_signer` has **no rotation function**, so recovery means deploying a new controller.

### What you would observe

- `get_nonce()` rising with no corresponding `SweepCompleted` events for your accounts.
- A burst of successful `sweep`-topic events on the controller from one relayer.
- Your sweeps failing intermittently with the P2 signature, resolving on retry.

### Requires

The sweep-signing key. This is a **key-compromise** scenario, not a permissionless one. See `testnet/security/admin-key-hygiene.md` for how that key should be kept.

---

## P4: Destination repointing

### What the attacker does

Call `update_authorized_destination` on the shared `SweepController` with their own address — during the window before the controller's first sweep.

### What the contract actually does

- Gated by the controller's `creator.require_auth()`.
- Blocked once `get_nonce() > 0` with `Error::AccountAlreadySwept` (7) — **the repoint window closes permanently at the first successful sweep.**
- Emits `DestinationUpdated { old_destination, new_destination }` (topic `dest_upd`). **Observable.**

**Expected behaviour:** a clean failure for everyone else. `validate_destination` runs *before* signature verification, so victims get `UnauthorizedDestination` (13) rather than an opaque trap:

```rust
pub fn execute_sweep(env: Env, /* ... */) -> Result<(), Error> {
    Self::validate_destination(&env, &destination)?;   // <-- first
    // signature verification happens after
}
```

**Severity: high during the window, then permanently closed.** And unlike P5, it is detectable from events alone.

### What you would observe

- A `dest_upd` event, or a `dest_auth` at `initialize`, from an unexpected address.
- `UnauthorizedDestination` on every sweep, for every integrator, at once.

### Triage

```bash
stellar contract events --contract-id "$SWEEP_CONTROLLER_ID" \
  --start-ledger <LOOKBACK> --filter '{"topics": [["dest_upd"]]}' --network testnet
```

If a `dest_upd` you cannot account for appears, the controller's creator key is the suspect. Cross-check `testnet/registry/admin-addresses.md`.

---

## P5: Unauthorised upgrade

### What the attacker does

Call `EphemeralAccount::upgrade(new_wasm_hash)` on instances whose `admin` is the testnet admin key.

### What the contract actually does

```rust
let admin = storage::get_admin(&env).ok_or(Error::NotUpgradeAdmin)?;
admin.require_auth();
env.deployer().update_current_contract_wasm(new_wasm_hash);
```

- Gated by the per-instance `admin`. `admin` is set at `initialize()` and has **no setter**.
- **No contract event is emitted.** Nothing in-band records this.
- The contract ID is unchanged, so nothing an integrator has configured reveals it happened.
- Reversible only by another `upgrade()` with a prior hash. No timelock, no multi-sig, no delay.

**Expected behaviour: silent global behaviour change.** Every integrator using that instance sees new behaviour with no notification and no on-chain trace.

### What you would observe

- Behaviour that does not match the source you are reading.
- A WASM hash on the deployed contract that does not match `testnet/registry/wasm-hash-reference.md`.
- **No corresponding entry in `testnet/registry/upgrade-history.md`** — the absence is the signal.
- Changed behaviour with no deploy or commit that would explain it.

### Triage

```bash
# Compare the live WASM hash against the reference
# then check whether upgrade-history.md accounts for it
```

Absence from the log means either an undocumented upgrade or a bug. The distinction requires no judgement: check the transaction history for an `upgrade` invocation.

### Prevention

Document every upgrade, immediately, in `testnet/registry/upgrade-history.md`. That log is the only defence against this being invisible.

---

## P6: Test-token balance poisoning

### What the attacker does

A SEP-41 test token on testnet is typically issuer-controlled. An attacker with the issuer key can burn or transfer balances they do not own — including the balances of ephemeral accounts they know nothing about.

### What the contract actually does

`transfers::execute_transfers()` calls `TokenClient::transfer()` for every recorded payment, and any failure is mapped to a single opaque error:

```rust
transfers::execute_transfers(env, &ephemeral_account, &destination, &payments_vec)
    .map_err(|_| Error::TransferFailed)?;
```

Soroban transactions are atomic, so **one** unfunded asset reverts the **entire** sweep — including the assets that are fully funded. There is no partial sweep and no best-effort mode.

**Expected behaviour:** a total `TransferFailed` (2) that looks like a bug in the integration and is not. The controller's `InsufficientBalance` variant exists but is not what surfaces here; every token error becomes `TransferFailed`.

### What you would observe

- Recorded payments in `get_info()` that look correct and are nonetheless unsweepable.
- Real balances on the ephemeral account that are **less than** the recorded amounts.
- A token transfer or burn from the issuer that predates the failure.

### Triage

```bash
# Recorded amounts say what SHOULD be there
stellar contract invoke --id "$EPHEMERAL_ID" --network testnet \
  --source testnet-investigator -- get_info

# Compare against what IS there, per asset, on the token contract
```

The gap between recorded and actual is the whole diagnosis. Assuming `InsufficientBalance` will be the error code wastes time — it will not be.

**Mitigation for your own suite:** assert real balances before sweeping, and prefer a small number of well-funded assets.

---

## P7: Payment injection — the one that needs no credentials at all

The most permissionless attack in this document, and the easiest to miss because it leaves no trace of an attacker.

### What the attacker does

Call `record_payment` on **your** ephemeral account. Note who is authorised to do this:

```rust
pub fn record_payment(env: Env, amount: i128, asset: Address) -> Result<(), Error> {
    if !storage::is_initialized(&env) { return Err(Error::NotInitialized); }
    if amount <= 0 { return Err(Error::InvalidAmount); }
    if storage::get_payment(&env, &asset).is_some() { return Err(Error::DuplicateAsset); }
    let payment_count = storage::get_total_payments(&env);
    if payment_count >= 10 { return Err(Error::TooManyPayments); }
    let payment = Payment { asset, amount, timestamp: env.ledger().timestamp() };
    storage::add_payment(&env, payment);      // <-- no require_auth() anywhere
    // ...
}
```

**No `require_auth()`. No caller check.** Any funded address can append a payment to any `EphemeralAccount`. The appended entry becomes part of the set the sweep later acts on.

### What the contract actually does

Three concrete effects, all from an unauthenticated call:

1. **Sweep denial.** Record a payment for an asset the account does not hold. The sweep now includes that transfer, it fails, and the whole sweep reverts with `TransferFailed`. Your account becomes **permanently unsweepable** — there is no way to remove a recorded payment. This combines with P6's mechanism but needs no issuer key at all.
2. **Slot exhaustion.** Fill all 10 asset slots. Your own intended `record_payment` then fails with `DuplicateAsset` or `TooManyPayments`.
3. **Evidence tampering.** `get_info()` now shows payments you never made, so any monitoring or reconciliation built on it is wrong.

**Expected behaviour:** the attacker's calls succeed silently. No event distinguishes a legitimate payment from an injected one — `PaymentReceived` for the first, `MultiPaymentReceived` after. Nothing records the caller, because the function does not know who the caller is.

### What you would observe

- A payment in `get_info()` that you did not record, with a `timestamp` you cannot place.
- A `PaymentReceived`/`MultiPaymentReceived` event at a ledger where you made no transaction.
- An account that reports `payment_count` higher than expected, or that fails to sweep with `TransferFailed` despite adequate balances for the payments *you* recorded.

### Triage

```bash
stellar contract events --contract-id "$EPHEMERAL_ID" \
  --start-ledger <LEDGER_BEFORE_YOUR_FUNDING> \
  --filter '{"topics": [["payment"], ["multi_pay"]]}' --network testnet
```

Correlate every recorded payment against the transactions you actually submitted. Anything else was injected.

**Severity: high, and trivially executable.** No key, no role, no prior access — just Friendbot. Treat unexplained entries in `get_info()` as an active interference signal, not as a data bug.

---

## P8: Expiry griefing

### What the attacker does

Watch for an account passing its `expiry_ledger` and call `expire()` on it before the legitimate owner acts.

### What the contract actually does

- `expire()` is **deliberately permissionless** — no `require_auth()`. This is intentional, so funds cannot be stranded waiting for a keeper. It is not a bug, and it should not be "fixed."
- It is **one-shot and final.** Status becomes `Expired`; a second call returns `InvalidStatus` (12).
- Afterwards `sweep()` and `claim()` both fail with `AccountExpired` (11), permanently. There is no way to reverse it.

**Expected behaviour:** the attacker's call succeeds. The owner's intended sweep is foreclosed. The `expired` event is emitted — with the configured `recovery_address` and the summed `amount_returned`.

### Severity: low, but irreversible

In practice this costs little today, because expiry recovery **does not move tokens** anyway — `ephemeral_account` has no `TokenClient::transfer()`, so `expire()` sets state and emits an event while balances stay put. What is lost is the *sweep*, and the state transition is permanent.

It matters more for accounts with a long expiry: an attacker who simply waits can foreclose a sweep long after you stopped watching. See `testnet/examples/expiry-and-recovery.md`.

### Triage

An `expired` event at a ledger where you submitted nothing, on an account you control.

---

## P9: Resource and RPC spam

### What the attacker does

Floods `simulateTransaction` / read-only invokes, or submits many tiny deployments to bloat the ledger.

### What the contract actually does

Nothing — these are legitimate public operations. This is an attack on **shared infrastructure**, not on the contracts.

**Expected behaviour:** RPC latency and error rates rise for everyone, including integrators doing nothing wrong. Ledger close time and fee floors drift. No contract state changes.

### Triage

Genuinely ambiguous, and worth saying so. A slow testnet is far more often RPC provider load, a network incident, or an ingestion pause than an attack. **Do not report latency as abuse without corroborating transaction volume.** Check for an unusual burst of transactions from a single source first.

Note that this is a legitimate reason *not* to submit sweeps in a tight retry loop — see the bounded-retry guidance in `testnet/examples/integration-test-harness-notes.md`.

---

## P10: Signature leakage — the enabler

Not an on-chain attack, but the precondition for most of the above, and the one that quietly converts P2 from "annoying" into "serious".

### The sharp edge

The signed digest binds exactly three things:

```
SHA256( destination.to_xdr() || nonce_be_u64 || controller_id.to_xdr() )
```

**It does not bind the ephemeral account, the asset list, or the amounts.**

So a leaked signature is not a permission to sweep one account. It is a permission to sweep **any account the controller can reach, to that destination, at that nonce** — which may be far more valuable than the account it was issued for. And the signature grants the destination, not a choice over it.

### Where signatures leak

- CI logs and build output — `sweep-signer sign` prints the signature to stdout by design.
- Shell history, pasted terminal transcripts, bug reports.
- PR descriptions and issue comments. **Forks and PRs are public.**
- Screenshots, Slack, shared documents.
- Client-side code in a web integration.

### Expected behaviour

The signature is valid and usable until the nonce advances. After one use it is dead. Nothing about possessing it is detectable on-chain — `sweep-signer` prints a warning about the nonce and nothing else, because from the contract's perspective there is nothing to warn about.

### Prevention

- Sign and submit in one step; do not persist signatures.
- Never paste one into an issue or a PR.
- If a signature may have been disclosed, treat the nonce it was built over as compromised and expect it to be used before you can rotate anything.

---

## What Is Not Abusable

Stated explicitly, because a triage document that only lists attack surface is as misleading as one that lists none.

| Attempted | Result | Why |
|---|---|---|
| Forging a sweep signature | Impossible without the signing key | Real `ed25519_verify` in `SweepController` |
| Replaying a spent signature | Always fails | Digest uses the current nonce, not a supplied one |
| Redirecting a captured sweep | Impossible | `destination` is inside the digest |
| Sweeping someone else's account directly | Impossible | `authorized_controller.require_auth()` |
| Stealing via `EphemeralAccount::sweep()` | **No** — the signature is unchecked, but no tokens move either | No `TokenClient::transfer()` in `ephemeral_account` |
| Re-initialising an account | Returns `AlreadyInitialized` (1) | One-time guard, and no setter for any `initialize()` parameter |
| Changing `expiry_ledger` or `recovery_address` after init | Impossible | No setters exist |
| Repointing the destination after the first sweep | Returns `AccountAlreadySwept` (7) | `nonce > 0` check |
| Removing an injected payment | Impossible | No delete path — which is what makes P7 permanent |

The two that look like holes and are not: **`EphemeralAccount::sweep()`'s missing verification is not exploitable today**, because that contract moves no tokens; and **the shared nonce causing constant signature failures is normal operation** on a busy multi-tenant controller, not an attack.

The one to keep an eye on: `EphemeralAccount::sweep()` is a stub awaiting implementation. If token transfers are ever added to that function before real verification is, the same call becomes a signature-bypass drain. Review any change to it as a security change.

---

## Detection Playbook

In rough order of cost, cheapest first.

```bash
# 1. Nonce sanity — explains most opaque traps
stellar contract invoke --id "$SWEEP_CONTROLLER_ID" --network testnet \
  --source testnet-investigator -- get_nonce

# 2. Did the controller's destination change?
stellar contract events --contract-id "$SWEEP_CONTROLLER_ID" \
  --start-ledger <LOOKBACK> --filter '{"topics": [["dest_auth"], ["dest_upd"]]}' --network testnet

# 3. Did any ephemeral account get upgraded?
#    No event exists — check the log, then the transaction history.
#    Absent from testnet/registry/upgrade-history.md but present on-chain = P5.

# 4. Payments you did not record (P7)
stellar contract events --contract-id "$EPHEMERAL_ID" --start-ledger <BEFORE_YOUR_TX> \
  --filter '{"topics": [["payment"], ["multi_pay"]]}' --network testnet

# 5. Expired by someone else (P8)
stellar contract events --contract-id "$EPHEMERAL_ID" --start-ledger <WINDOW> \
  --filter '{"topics": [["expired"]]}' --network testnet

# 6. Is the shared deployment's WASM still what the reference says?
#    Compare against testnet/registry/wasm-hash-reference.md
```

## Reporting

Report a suspected pattern with the **observation** first, the hypothesis second, and no accusation without evidence. Include the contract ID, the ledger range, transaction hashes, and the exact error. Mechanism-level reports are what make this document better; naming a party without evidence is noise, and on a shared deployment it is also unfair.

Mechanics for the diagnostics live in `testnet/runbooks/`, not here.

---

## Related Documentation

- `testnet/security/README.md` — index and scope of this directory
- `testnet/security/admin-key-hygiene.md` — P3, P4, P5 all require a key; this is the defence
- `testnet/security/unverified-signature-path-warning.md` — the P1b-adjacent confusion about `sweep()`
- `testnet/runbooks/failed-sweep-signature.md` — diagnostic for P2
- `testnet/runbooks/account-factory-batch-failure.md` — diagnostic for P1
- `testnet/runbooks/stuck-ephemeral-account.md` — diagnostic for P6, P7, P8
- `testnet/config/nonce-tracking.md` — the nonce mechanics behind P2 and P3
- `testnet/registry/upgrade-history.md` — where a P5 upgrade should have been recorded
- `testnet/registry/admin-addresses.md` — who holds the keys P3/P4/P5 require
- `testnet/registry/event-topics.md` — topic encodings used in the queries above
- `docs/security.md` — design-level threat model; different concern
- `contracts/ephemeral_account/src/lib.rs`, `contracts/sweep_controller/src/`, `contracts/account_factory/src/lib.rs` — the code cited above

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-26 | Initial known testnet abuse patterns and triage reference |
