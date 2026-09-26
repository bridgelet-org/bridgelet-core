# Runbook: Recover SweepController Nonce Desynchronization

## Purpose

Use this runbook when an off-chain signer or relayer's cached
`SweepController` nonce no longer matches the live testnet contract.

The authoritative value is always the result of:

```text
SweepController::get_nonce()
```

An off-chain counter is a cache and queue coordination aid. It must never be
used to overwrite, guess, or decrement contract state.

This nonce is distinct from a Stellar account sequence number, an
`expiry_ledger`, and any per-account counter. There is one security nonce per
`SweepController` deployment, shared by every account it services.

## Signed message and nonce lifecycle

The current controller signs and verifies this digest:

```text
SHA256(
    destination.to_xdr()
    || nonce encoded as an unsigned u64 in 8-byte big-endian order
    || sweep_controller_id.to_xdr()
)
```

The ephemeral-account address is not included in the current digest. Preserve
the exact destination and controller ID used to create a signature.

For a committed `execute_sweep` transaction, the controller verifies the
signature and advances its nonce as part of the atomic transaction. A reverted
transaction does not persist a nonce increment. Query the chain after any
uncertain result rather than predicting rollback from a local counter.

A `sweep` event can be emitted by more than one controller path in current
source. Do not infer the on-chain nonce by counting events; `get_nonce` is the
authority.

## Symptoms

Typical indicators include:

- repeated signature-verification failures after a successful sweep;
- a signature that looks correct until another relayer submits;
- a local nonce lower or higher than the contract value;
- a local increment after a failed or unknown transaction;
- multiple workers signing from the same cached value;
- stale signatures after a testnet reset or controller redeployment; or
- a signature failure caused by a different destination, contract ID, signer
  key, or XDR encoding.

A fresh signature failure does not by itself prove nonce desynchronization.

## Immediate safety actions

1. Pause signing and submission for the affected controller.
2. Preserve the local nonce, destination, controller ID, transaction hashes,
   UTC time, and approximate ledger.
3. Do not reuse any signature built with a stale or uncertain nonce.
4. Do not retry an in-flight transaction before resolving its result by hash.
5. Serialize all remaining work per controller ID.
6. Do not call a write method to "reset" the contract nonce; no such method is
   required or documented.
7. Keep signing seeds, secret keys, and full signatures out of tickets and
   public logs.

## Detection procedure

### 1. Capture incident context

Record:

- exact controller contract ID and network passphrase;
- local/cached nonce used for each failed signature;
- destination used in the invocation;
- transaction hash, if submitted;
- whether the path was `execute_sweep` or another controller method;
- approximate ledger and UTC time; and
- all workers or services that can sign for this controller.

### 2. Query the authoritative nonce

Use a confirmed read-only simulation or dry-run invocation:

```bash
stellar contract invoke \
  --simulate-only \
  --id <SWEEP_CONTROLLER_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  get_nonce
```

Record the result as an unsigned decimal integer. An automated worker should
use the SDK/RPC simulation equivalent and must not submit a transaction.

A successful `0` is valid, but the storage getter also defaults to zero when
the key is absent. Zero alone does not prove controller initialization.

If the query fails, verify the RPC, network, exact controller ID, testnet
reset status, and deployed interface. Do not reset the local cache merely
because the query failed.

### 3. Compare the values

Let:

```text
L = local/cached nonce
C = value returned by get_nonce()
```

| Comparison | Interpretation | Action |
|---|---|---|
| `L == C` | cache is currently aligned | inspect destination, controller ID, key, XDR, and deployed code if the signature still fails |
| `L < C` | signer is behind the chain | discard signatures made with `L`; fetch `C` and sign again |
| `L > C` | local state is ahead or based on a different deployment/path | pause and reconcile before resetting the cache |
| `C - L > 1` | multiple operations or a tracking bug likely occurred | stop the queue and account for every pending operation |
| `C < previous observed C` | reset, contract change, or bad history | quarantine the baseline; do not reuse the old counter |

Never modify the on-chain value to match the local cache.

### 4. Correlate committed activity

Inspect finalized controller and account activity over the relevant ledger
range. Correlate:

- controller ID;
- ephemeral-account ID;
- destination;
- transaction hash and ledger;
- final transaction result; and
- controller `sweep` and account `swept_mul` events when available.

Events support diagnosis but do not replace `get_nonce`. A failed Soroban
transaction is atomic and should not leave a nonce increment, state change,
event, or transfer committed.

### 5. Verify every signed component

If `L == C` and a new signature still fails, stop treating it as a nonce
problem. Check, in order:

1. The destination is byte-for-byte the address used in the signature.
2. The controller ID is the same testnet deployment.
3. The nonce is exactly eight bytes, unsigned big-endian.
4. Destination and controller addresses use the same Soroban `Address::to_xdr`
   encoding as the contract.
5. The private key matches the controller's `authorized_signer` public key.
6. No timestamp or extra field was added to the current message format.
7. The signature is exactly 64 bytes and was not truncated or text-mangled.
8. The deployed WASM/interface is the expected version.

A successful `get_nonce` call does not prove any of those values.

## Recovery: local nonce is behind the chain

1. Hold the per-controller signing lock.
2. Read and record `C`.
3. Mark every pending signature built with a nonce below `C` as invalid.
4. Discard those signatures.
5. Read `get_nonce` again immediately before the next signature.
6. Sign the exact destination and controller ID with that fresh value.
7. Submit while holding the lock and wait for a definitive result.
8. Re-read `get_nonce` and record the progression.

Do not change the nonce bytes inside an already signed message and expect the
old signature to remain valid.

## Recovery: local nonce is ahead of the chain

1. Pause all signers and submissions.
2. Query and record `C`.
3. Determine whether the local increment came from a failed/unknown
   transaction, a different controller deployment, a nonstandard path, or a
   bookkeeping bug.
4. Reconcile every pending transaction by hash.
5. Discard signatures that depend on the invalid local value.
6. Remove the persistent local counter or set the cache to `C` only after the
   chain state is understood.
7. Re-read `C` immediately before each new signature.

If `C` unexpectedly decreased for the same contract ID, treat the deployment
or reset epoch as changed and require fresh verification.

## Recovery: concurrent signers

Use one queue or mutex per `SweepController` contract ID:

1. acquire the controller lock;
2. read `get_nonce`;
3. sign and submit while holding the lock;
4. wait for a definitive transaction result;
5. re-read `get_nonce`; and
6. release the lock.

Do not allow two workers to sign from the same cached value. The first
successful committed sweep invalidates the other signature.

A timeout releases no lock automatically until the submitted transaction's
result is resolved.

## Recovery: fresh signature still fails

When `L == C`, build a new signature only after verifying the other components
above. If it still fails, classify the incident as destination, controller ID,
signer key, XDR encoding, deployed-version, or host-verification failure—not
as nonce desynchronization.

Preserve the diagnostic result and do not repeatedly submit different payloads
to guess which field is wrong.

## Verification and closure

The incident is closed only when:

- the controller lock/queue is consistent;
- no stale or unknown transaction remains;
- a new signature is built from a fresh authoritative nonce;
- destination and controller ID are unchanged;
- a committed `execute_sweep` advances the nonce by one, or the failure is
  reclassified with direct evidence; and
- the expected account state, controller event, and destination balance are
  correlated for a successful sweep.

For a successful sweep, also query the account's terminal state and relevant
balances. A controller event alone is not sufficient evidence of value
movement.

## Escalation

Escalate to the contract/testnet maintainers when:

- `get_nonce` fails for a previously verified controller;
- the nonce decreases in the same deployment/reset epoch;
- a transaction succeeded but the nonce did not change as expected;
- a reverted transaction appears to have committed a nonce increment;
- the same contract ID resolves to an unexpected code/interface;
- all configured IDs disappear after a suspected testnet reset; or
- the destination or signer identity may be wrong.

Attach the contract ID, local and on-chain nonces, redacted signature metadata,
transaction hashes, finalized status, relevant events, and deployed code
identity. Never attach signing seeds or secret keys.

## Prevention checklist

- [ ] Read `get_nonce` immediately before every signature.
- [ ] Serialize sign-and-submit per controller.
- [ ] Hold the lock through definitive transaction finalization.
- [ ] Reconcile uncertain submissions by hash before retrying.
- [ ] Treat testnet reset/deployment changes as a new nonce epoch.
- [ ] Log the controller ID, nonce used, destination ID, and transaction hash,
      but not the secret key or full reusable signature.
- [ ] Alert on nonce regression and unexplained jumps.
- [ ] Test concurrent-worker and unknown-transmission scenarios in the future
      signer implementation.

## Related documents

- [`../config/nonce-tracking.md`](../config/nonce-tracking.md)
- [`failed-sweep-signature.md`](failed-sweep-signature.md)
- [`../registry/event-topics.md`](../registry/event-topics.md)
- [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md)
- [`../../docs/SIGNATURE_FORMAT.md`](../../docs/SIGNATURE_FORMAT.md)
- [`../../tools/sweep-signer/`](../../tools/sweep-signer/)
