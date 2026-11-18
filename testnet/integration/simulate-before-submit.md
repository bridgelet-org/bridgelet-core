# Simulate Before Submit: Testnet Pre-flight for State-Changing Calls

## Overview

`simulateTransaction` is the Soroban RPC method that executes a transaction against a recent ledger snapshot without committing state or charging a fee. On a shared testnet deployment it is the cheapest way to learn that a state-changing invocation is malformed, unauthorized, or would fail against current state.

This document covers the RPC pre-flight, the CLI `--send no` equivalent, what simulation cannot guarantee, and a retry workflow that does not turn a pre-flight into a nonce race.

> **Deployment status.** `testnet/registry/contract-status.md` is the canonical record and lists all four contracts as undeployed, with IDs `TBD` and `Last Verified: Never`. Every ID below is a placeholder. Do not treat an ID copied from an older artifact as evidence that a contract is live.

---

## When to simulate

Simulate every state-changing call before signing and submitting: `initialize`, `record_payment`, `sweep`, `execute_sweep`, `claim`, `expire`, `reclaim_reserve`, token `transfer`, and any configuration change. Read-only calls are already cheap, but `--send no` is also how you probe them without a fee.

| Concern | `simulateTransaction` | Submitted transaction |
|---|---|---|
| State committed | No | Yes |
| Inclusion fee charged | No | Yes |
| Contract code actually executed | Yes, against a recent snapshot | Yes, against current state |
| Can be used as a pre-flight gate | Yes | No — it is the thing being gated |

Bridgelet also exposes `EphemeralAccount::simulate_sweep`, a contract helper that returns `(Vec<Payment>, u32)` with the error code in the return value (`0` on success). That is a dry run of sweep logic, not a replacement for RPC simulation: see `docs/api-reference.md` and `testnet/examples/error-handling.md`.

---

## What simulation catches

| Failure class | Typical symptom | What to do |
|---|---|---|
| **Authorization** | `require_auth()` rejects the source account or a missing Soroban auth entry (for example, `claim` signed by the wrong address) | Fix the signer or auth entry before submitting. Do not retry unchanged |
| **Budget / resource limits** | Instruction, memory, or disk limits exceeded; a read that returns too much state | Reduce work — use `get_info_paginated` / `simulate_sweep_paginated` rather than raising the retry count. See `docs/api-reference.md` |
| **Insufficient balance** | A token transfer that would fail on balance in the simulated snapshot, surfaced as the invoked contract's transfer error | Fund or reduce the amount. Note that `SweepController`'s `InsufficientBalance` variant is not constructed by the current source; match the error the contract actually returns |
| **Wrong contract ID** | No contract at the address, or an uninitialized instance (`NotInitialized`, code `2` on `EphemeralAccount`) | Re-resolve the ID from config and verify it per `testnet/registry/verify-contract-live.md`. Stop — this is not transient |
| **Wrong interface** | Unknown function name, wrong argument type or arity, WASM/ABI drift | Correct the call against `docs/api-reference.md`; do not guess argument encodings |
| **Contract state errors** | `DuplicateAsset` (13), `AlreadySwept` (7), `AccountExpired` (11), and other contract-scoped codes | Treat as state facts, not transport failures. See `testnet/examples/error-handling.md` |
| **Signature failure at the simulated ledger** | An `ed25519_verify` trap for `execute_sweep` when the nonce, destination, or controller ID does not match the current snapshot | Re-read the nonce and rebuild the signature. Simulation proves nothing about the nonce at submit time — see below |

Fee-payer XLM sufficiency and inclusion are enforced when the transaction is submitted, not by the simulated contract execution. Check the payer balance separately.

---

## What simulation cannot guarantee

| It cannot guarantee | Why |
|---|---|
| **The nonce is still current** | Simulation reads a snapshot; it does not reserve the controller nonce. Another integrator can consume it between pre-flight and submit. A signature that simulated cleanly can still trap on submission. See `testnet/config/nonce-tracking.md` |
| **State has not changed** | Status, payment list, balances, and locked destination can all change after the snapshot. A simulation that returned success is not a claim about submission-time state |
| **Ledger expiry will not happen** | Ledgers advance roughly every five seconds. `is_expired()` is inclusive (`current >= expiry_ledger`), so a call that simulated just before the boundary can fail just after it |
| **Fee or resource cost is fixed** | `minResourceFee`, budget consumption, and ledger-dependent state may differ at submission. Re-simulate on a genuine retry |
| **Transaction inclusion or finality** | Nothing is submitted. A delayed or lost submission still has to be resolved by transaction hash, not by re-sending |
| **Value moved or an event was emitted** | Simulation does not commit. A contract event and a balance change are separate facts, both to be checked after confirmation — see `testnet/examples/README.md` |

A clean simulation is a gate on *this attempt with this snapshot*, not a promise about the next one.

---

## CLI pre-flight: `--send no`

```bash
export NETWORK=testnet
export RPC_URL="https://soroban-testnet.stellar.org"
export READER="<funded-testnet-identity>"
export SWEEP_CONTROLLER_ID="<verified-sweep-controller-id>"
export EPHEMERAL_ACCOUNT_ID="<verified-ephemeral-account-id>"
export DESTINATION="<destination-address>"

stellar contract invoke \
  --send no \
  --id "$SWEEP_CONTROLLER_ID" \
  --network "$NETWORK" \
  --source "$READER" \
  -- \
  execute_sweep \
  --ephemeral_account "$EPHEMERAL_ACCOUNT_ID" \
  --destination "$DESTINATION" \
  --auth_signature "<64-byte-signature-from-fresh-nonce>"
```

`testnet` is the built-in CLI network. Its passphrase must be `Test SDF Network ; September 2015`, exactly, including the spaces around `;`. If you register the network yourself:

```bash
stellar network add testnet \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015"
```

The same flag is useful for the nonce read that precedes a signature:

```bash
stellar contract invoke --send no \
  --id "$SWEEP_CONTROLLER_ID" --network "$NETWORK" --source "$READER" \
  -- get_nonce
```

---

## Raw JSON-RPC: `simulateTransaction`

Build the transaction envelope with your SDK, base64-encode it, and pass it as `params.transaction`:

```bash
curl -fsS "$RPC_URL" \
  -H 'Content-Type: application/json' \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "simulateTransaction",
    "params": {
      "transaction": "<BASE64_TX_ENVELOPE>"
    }
  }'
```

A successful simulation returns success fields only (values are placeholders, not observations):

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "latestLedger": 0,
    "results": [
      {
        "xdr": "<base64 invoke-host-function-result>"
      }
    ],
    "events": ["<base64 diagnostic event>"],
    "minResourceFee": "0"
  }
}
```

A failed simulation carries `error` **instead of** `minResourceFee` and `results`:

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "latestLedger": 0,
    "error": "<simulation error>"
  }
}
```

`result.events` is an array of base64 strings; event strings are not nested under each `results[]` item. Other success fields such as `transactionData` and `restorePreamble` vary by SDK and RPC version. Decode `results[].xdr` and `result.events[]` with the same SDK version that built the transaction. Do not hand-assemble the XDR envelope or decode results with string matching.

In JavaScript/TypeScript the SDK exposes this as `server.simulateTransaction(transaction)` on an `rpc.Server`; the payload above is what that call sends on the wire.

---

## Recommended workflow

1. **Resolve IDs from configuration**, not from source or a pasted example. Confirm the current status against `testnet/registry/contract-status.md` and verify resolvability per `testnet/registry/verify-contract-live.md`.
2. **Read fresh state.** Fetch the latest ledger and the controller nonce immediately before signing. Hold a per-controller lock across read → sign → simulate → submit → resolve.
3. **Build the invocation** with that nonce, the exact destination, and the exact controller ID. Never cache a signature across attempts.
4. **Simulate.** If the result contains an error, stop and fix the cause. Do not submit a known-bad transaction to learn the same thing again.
5. **Submit once** and retain the transaction hash.
6. **Resolve by hash**, with a bounded poll and timeout. A timeout does not release the lock and does not mean the transaction failed: see `testnet/runbooks/nonce-desync.md`.
7. **On a genuine miss**, re-read state and nonce, re-simulate, and make one of at most 2–3 bounded attempts with backoff. A tight loop against a shared controller is itself an abuse pattern — see `testnet/security/known-testnet-abuse-patterns.md`.
8. **Reconcile after confirmation.** For a sweep check final transaction status, `get_status == 2`, `get_info.swept_to`, `can_sweep == false`, nonce movement, and the destination balance. Event presence alone proves none of it.

```typescript
async function submitWithPreflight(
  readFreshNonce: () => Promise<bigint>,
  buildTx: (nonce: bigint) => Promise<Transaction>,
  simulate: (tx: Transaction) => Promise<{ error?: string }>,
  submit: (tx: Transaction) => Promise<string>,
  resolveByHash: (hash: string) => Promise<'success' | 'failure' | 'unknown'>,
): Promise<string> {
  for (let attempt = 1; attempt <= 3; attempt++) {
    const nonce = await readFreshNonce();
    const tx = await buildTx(nonce);
    const preflight = await simulate(tx);
    if (preflight.error) {
      throw new Error(`pre-flight failed: ${preflight.error}`);
    }
    const hash = await submit(tx);
    const outcome = await resolveByHash(hash);
    if (outcome === 'success') return hash;
    if (outcome === 'unknown') {
      throw new Error(`submission unresolved; inspect hash ${hash}`);
    }
  }
  throw new Error('retry budget exhausted: check nonce contention');
}
```

The harness notes in `testnet/examples/integration-test-harness-notes.md` set the latency budget, sequencing rules, and shared-deployment etiquette this loop assumes.

---

## Related Documentation

- `testnet/config/nonce-tracking.md` — the global controller nonce and why simulation cannot reserve it
- `testnet/examples/integration-test-harness-notes.md` — live-testnet suite design, latency, and etiquette
- `testnet/registry/contract-status.md` — canonical deployment status; all four contracts are undeployed
- `testnet/registry/verify-contract-live.md` — confirming an ID before use
- `testnet/docs/network-config.md` — exact passphrase and endpoints
- `testnet/docs/soroban-cli-quickref.md` — CLI invocation shapes
- `testnet/examples/checking-account-status.md` — read-only state gates before a submission
- `testnet/examples/error-handling.md` — contract discriminants versus traps
- `testnet/examples/idempotency-considerations.md` — why a duplicate sweep is not a retry
- `testnet/runbooks/nonce-desync.md` — resolving an ambiguous result
- `testnet/integration/sdk-checklist.md` — the wrapper-level checklist that includes this pre-flight
- `docs/api-reference.md` — signatures, return types, and error tables

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-11-18 | Initial testnet pre-flight guidance for `simulateTransaction` and `--send no` |
