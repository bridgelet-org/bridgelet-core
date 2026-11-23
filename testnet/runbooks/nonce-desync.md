# Runbook: Recover SweepController Nonce Desynchronization

## Purpose

Use this runbook when an off-chain signing system or relayer's locally tracked nonce drifts out of sync with the live testnet `SweepController` contract.

The authoritative nonce is **always** the value returned by querying the on-chain contract directly:

```text
SweepController::get_nonce()
```

An off-chain counter maintained in an integrator's backend, database, or worker memory is strictly a local queue coordination cache. It must **never** be used to overwrite, guess, or decrement contract state.

> **Contextual Reference (`tools/sweep-signer`):** Integrators often use `tools/sweep-signer` (or an equivalent backend signing library) to generate the cryptographic `auth_signature` passed to `SweepController::execute_sweep`. The CLI requires an explicit `--nonce <u64>` argument:
> ```bash
> sweep-signer sign --contract-id <CONTROLLER_ID> --destination <DEST> --nonce <NONCE> --signer-seed-hex <SEED>
> ```
> If an off-chain orchestration system tracks nonces independently without querying the contract before each signing operation, the local counter will inevitably fall out of sync with on-chain reality.

---

## Nonce Lifecycle & Digest Structure

The on-chain `SweepController` verifies signatures using this SHA-256 digest:

```text
SHA256(
    destination.to_xdr()
    || nonce (unsigned u64 in 8-byte big-endian format)
    || sweep_controller_id.to_xdr()
)
```

Key invariant rules:
1. **Scope:** There is exactly **one** global `nonce` per deployed `SweepController` instance, shared across all ephemeral accounts that route through it. It is **not** per ephemeral account, not an `expiry_ledger`, and not a Stellar account sequence number.
2. **Atomic Advancement:** When `execute_sweep` completes successfully, the contract increments `nonce += 1`.
3. **Reverted Transactions:** If a transaction fails or reverts (e.g., due to an error in the child ephemeral account), the transaction is atomic and the on-chain nonce does **not** advance.
4. **No Write Reset:** There is no contract method to reset or decrement the on-chain nonce.

---

## Symptoms & Failure Mode Detection

### The Primary Failure Signature

The definitive symptom of nonce desynchronization is:

> **Repeated signature-verification failures (`Error::SignatureVerificationFailed` / Error Code 1) on `SweepController::execute_sweep`, despite the integrator using a seemingly correct signature.**

Integrators typically observe:
- The signing key is valid and corresponds to `authorized_signer`.
- The destination address is correct and matches what was signed.
- The controller contract ID is correct and points to the live deployment.
- The signature is properly encoded (64-byte Ed25519 signature).
- **Yet the transaction repeatedly reverts with `Error(Contract, #1)`.**

Because the signature seems structurally and cryptographically sound, teams often suspect bad keys, serialization bugs, or contract regressions, when the true root cause is that the off-chain system signed with a stale or advanced nonce.

### Common Drift Scenarios

| Scenario | What Happened | Consequence |
|---|---|---|
| **Missed On-Chain Execution (`L < C`)** | Another relayer worker or test run executed a sweep, advancing the on-chain nonce from $C$ to $C + 1$. The local system retained $L = C$. | Next signature is built with stale nonce $L$. Rejected on-chain with `SignatureVerificationFailed`. |
| **Optimistic Increment on Revert (`L > C`)** | The off-chain system incremented its local counter upon transaction submission, but the transaction reverted or dropped. | The contract nonce remained at $C$, while the off-chain system now signs with $L > C$. Rejected on-chain. |
| **Testnet Reset or Contract Redeployment** | Testnet was reset or `SweepController` was redeployed. On-chain nonce reset to `0`. | Off-chain system continues signing with high nonce from previous deployment. Rejected on-chain. |
| **Concurrent Uncoordinated Workers** | Two parallel workers queried `get_nonce()` at the same time and both received $C$. | Worker A commits; Worker B's transaction fails with `SignatureVerificationFailed` because the nonce is now $C + 1$. |

---

## Immediate Safety Protocol

1. **Pause Signing Queue:** Halt all automated signing and submission jobs for the affected controller contract ID.
2. **Do Not Guess Nonces:** Never loop through arbitrary integer nonces trying to find one that succeeds.
3. **Never Retry Reverted Signatures:** An Ed25519 signature commits to the exact nonce bytes. A rejected signature cannot be reused.
4. **Preserve Incident Logs:** Record the local cached nonce, destination address, controller contract ID, and recent transaction hashes.
5. **Protect Secrets:** Never post private keys, seed phrases, or raw signing seeds in tickets or incident channels.

---

## Detection & Querying Procedure

### Step 1: Query the Authoritative On-Chain Nonce Directly

To detect desynchronization, query the live `SweepController` contract directly via a read-only simulation:

```bash
stellar contract invoke \
  --simulate-only \
  --id <SWEEP_CONTROLLER_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  get_nonce
```

*Or via Soroban RPC / SDK equivalent:*
Send a `simulateTransaction` request for the contract call `get_nonce()`. Parse the returned `xdr:ScVal` as a `u64`.

Record the result as an unsigned integer $C$.

### Step 2: Compare Local Nonce with On-Chain Nonce

Let:
- $L$ = Nonce tracked in local database / memory / `tools/sweep-signer` input
- $C$ = Authoritative on-chain value returned by `get_nonce()`

```text
                    ┌────────────────────────┐
                    │ Compare L against C    │
                    └───────────┬────────────┘
                                │
         ┌──────────────────────┼──────────────────────┐
         ▼                      ▼                      ▼
      [ L < C ]              [ L > C ]              [ L == C ]
   Signer is Behind       Signer is Ahead       Nonces Aligned
         │                      │                      │
         ▼                      ▼                      ▼
  Missed committed tx.   Optimistic increment   Check destination,
  Invalidate local cache  after tx revert.       contract ID, key,
  and update L = C.       Reconcile by tx hash   or XDR encoding.
                          and reset L = C.
```

### Evaluation Matrix

| State | Interpretation | Action |
|---|---|---|
| **$L < C$** | **Local signer is behind the chain.** One or more sweeps committed without being recorded locally. | 1. Invalidate all pending signatures generated with nonce $< C$.<br>2. Update local tracking store to $C$.<br>3. Re-sign pending sweeps using $C$. |
| **$L > C$** | **Local signer is ahead of the chain.** The off-chain system optimistically incremented for a transaction that reverted or failed in the mempool. | 1. Search Horizon / Soroban RPC for recent transaction hashes.<br>2. Discard stale pending signatures.<br>3. Reset local tracking store to $C$.<br>4. Re-sign from $C$. |
| **$L == C$** | **Nonces are aligned.** Signature failure is **not** caused by nonce desync. | Re-verify: (a) `destination` address matches signed address exactly, (b) `contract_id` matches, (c) signer key matches `authorized_signer`, (d) no extra fields in digest. |
| **$C == 0$** | **Fresh deployment or testnet reset.** | If $L > 0$, the controller was redeployed or reset. Update local baseline to 0. Verify controller initialization. |

---

## Step-by-Step Resynchronization Procedure

Follow these steps to restore healthy signing operations:

### 1. Acquire Local Mutex Lock
Ensure only one worker or process accesses the signing pipeline for `<SWEEP_CONTROLLER_ID>`.

### 2. Direct On-Chain Query
Invoke `get_nonce` directly on the target network:
```bash
AUTH_NONCE=$(stellar contract invoke \
  --simulate-only \
  --id <SWEEP_CONTROLLER_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  get_nonce | tr -d '"')

echo "Authoritative on-chain nonce: $AUTH_NONCE"
```

### 3. Clear Stale Signatures
Purge any pre-generated signatures or queued tasks that were signed using the stale nonce $L$.

### 4. Update Off-Chain Datastore
Set the local nonce tracker in your database or service configuration:
```sql
UPDATE relayer_state SET current_nonce = <AUTH_NONCE> WHERE controller_id = '<SWEEP_CONTROLLER_ID>';
```

### 5. Re-Sign Using `tools/sweep-signer`
Generate a fresh signature using the authoritative nonce:
```bash
cargo run --bin sweep-signer -- sign \
  --contract-id <SWEEP_CONTROLLER_ID> \
  --destination <DESTINATION_ADDRESS> \
  --nonce $AUTH_NONCE \
  --signer-seed-hex <SIGNER_SEED_HEX>
```

### 6. Submit and Await Finalization
Submit the transaction to testnet and hold the lock until the transaction is definitively finalized in a ledger:
```bash
stellar contract invoke \
  --id <SWEEP_CONTROLLER_ID> \
  --network testnet \
  --source <RELAYER_IDENTITY> \
  -- \
  execute_sweep \
  --account <EPHEMERAL_ACCOUNT_ID> \
  --destination <DESTINATION_ADDRESS> \
  --auth_signature <FRESH_AUTH_SIGNATURE>
```

### 7. Confirm Incremental Advancement
Verify that the on-chain nonce advanced by exactly 1:
```bash
stellar contract invoke \
  --simulate-only \
  --id <SWEEP_CONTROLLER_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  get_nonce
```
Expected output: `$AUTH_NONCE + 1`.

### 8. Release Mutex Lock
Resume normal queue processing.

---

## Architectural Best Practices to Prevent Desync

To permanently prevent nonce desynchronization in production and testnet relayers:

1. **Read-Before-Sign Pattern:** Do not maintain a long-lived local nonce counter. Query `SweepController::get_nonce()` via simulation immediately prior to constructing each signature.
2. **Queue Serialization (Single Flight):** Enforce strict FIFO submission per `SweepController` ID. Never allow multiple workers to sign sweeps concurrently for the same controller.
3. **Do Not Optimistically Increment:** Only advance local state after receiving confirmed transaction finalization from Soroban RPC (`getTransaction` status `SUCCESS`).
4. **Reconcile Uncertain Transactions by Hash:** If a submission times out or network connection drops, do **not** assume failure or retry with a new nonce. Query the transaction status by hash first. If it committed, the nonce advanced; if it dropped, the nonce did not.
5. **Handle Testnet Resets:** Reset local caches to 0 whenever a new contract deployment ID is detected.

---

## Fast Triage Checklist

```text
[ ] Paused signing queue for the controller ID.
[ ] Queried live on-chain nonce directly using get_nonce().
[ ] Compared local nonce (L) with on-chain nonce (C).
[ ] Discarded invalid/stale signatures.
[ ] Re-signed payload with the verified on-chain nonce.
[ ] Submitted sweep and verified successful execution.
[ ] Confirmed on-chain nonce advanced by exactly +1.
[ ] Resumed queue with serialized worker locks.
```

---

## Related Documentation

- [`failed-sweep-signature.md`](failed-sweep-signature.md) — Fast triage for rejected sweep signatures
- [`stuck-ephemeral-account.md`](stuck-ephemeral-account.md) — Resolving stuck accounts
- [`../config/nonce-tracking.md`](../config/nonce-tracking.md) — Architecture of controller nonce tracking
- [`../registry/event-topics.md`](../registry/event-topics.md) — Controller `sweep` and account `swept_mul` events
- [`../../docs/SIGNATURE_FORMAT.md`](../../docs/SIGNATURE_FORMAT.md) — Official specification for sweep signatures
- [`../../tools/sweep-signer/`](../../tools/sweep-signer/) — Reference implementation CLI for sweep signing
