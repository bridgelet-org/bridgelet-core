# Synthetic Create-Pay-Sweep Canary Plan

## Status

> **Proposal only.** This repository does not contain a running canary,
> scheduler, signing service, event collector, or alert notifier. This document
> defines the contract and evidence a future implementation must provide.

The canary is a scheduled, disposable end-to-end workflow. Unlike a basic RPC
liveness check, it proves that a real asset reaches a new ephemeral account,
that the payment is recorded, and that the signed sweep reaches the expected
terminal state.

## Goals

One successful run must prove all of the following:

1. The configured contracts exist on Stellar Testnet and expose the expected
   interface.
2. A fresh `EphemeralAccount` is deployed and initialized with the canary
   `SweepController` as its authorized controller.
3. Native XLM is actually transferred to the new account contract.
4. `record_payment` records the exact asset and amount that was observed.
5. `SweepController::can_sweep` becomes `true`.
6. A signature made with a fresh on-chain nonce is accepted.
7. `execute_sweep` reaches a successful final transaction status.
8. Account state, controller state, events, nonce progression, and destination
   balances agree.

An HTTP success, submitted transaction, or contract event is not sufficient on
its own.

## Proposed operating profile

| Setting | Initial proposal |
|---|---|
| Schedule | Every 30 minutes, with a bounded 15-minute run timeout |
| Concurrency | One active run; no overlapping canaries |
| Asset | Native XLM Stellar Asset Contract |
| Amount | `1,000,000` stroops (`0.1` XLM) unless a lower value is approved |
| Expiry margin | At least 1,000 ledgers beyond the start ledger |
| Destination | Dedicated canary-only address |
| Controller | Dedicated, initialized `SweepController` where practical |
| Retries | Never reuse an account, signature, or `run_id` |

These are starting values, not a tested SLO. Approve the interval and amount
after observing real testnet fees and ledger behavior.

## Required configuration

The future runner must receive these values from an external, non-committed
configuration:

| Setting | Requirement |
|---|---|
| `NETWORK_PASSPHRASE` | Exactly `Test SDF Network ; September 2015` |
| `RPC_URL` | Reachable testnet Soroban RPC; use only a reviewed fallback |
| `EA_WASM_HASH` | Current `EphemeralAccount` WASM installed on testnet |
| `CANARY_CONTROLLER_ID` | Initialized controller used by this workflow |
| `CREATOR_IDENTITY` | Can initialize the disposable account |
| `PAYER_IDENTITY` | Funded source for the XLM transfer and fees |
| `RELAYER_IDENTITY` | Submits the signed sweep |
| `DESTINATION_ADDRESS` | Approved public canary destination |
| `SIGNING_PUBLIC_KEY` | Matches the controller's `authorized_signer` |
| `XLM_SAC_ID` | `CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC` |

Signing seeds and transaction-submission credentials must come from a secret
manager. Do not place them in this repository, result labels, or public logs.

## Why a dedicated controller is preferred

`SweepController` stores one security nonce for all accounts it serves. Every
committed `execute_sweep` advances it. A shared controller therefore creates
signing races and makes unrelated activity look like canary failure.

A dedicated controller should use a dedicated signer and destination, with one
sign-and-submit operation in flight. If a shared controller is unavoidable,
serialize all operations and read `get_nonce` immediately before each
signature.

## Run state machine

```text
preflight
  -> deploy
  -> initialize
  -> transfer XLM
  -> confirm destination balance
  -> record payment
  -> confirm can_sweep
  -> read fresh controller nonce
  -> sign and submit sweep
  -> wait for final transaction result
  -> verify state, nonce, events, and balances
  -> emit one terminal result
```

Each arrow advances only after the previous step has a definitive result.

## Phase 1: Preflight

Before making any network change:

1. Confirm the RPC serves the exact testnet passphrase.
2. Record the latest finalized ledger and any testnet-reset epoch.
3. Verify the controller with read-only `get_nonce`.
4. Verify the configured `EphemeralAccount` WASM hash is installed.
5. Verify all configured public addresses and the XLM SAC are on testnet.
6. Confirm the payer, creator, and relayer can pay.
7. Confirm the signing service is unlocked for the intended controller only.
8. Confirm no previous canary run still holds the controller lock.

A failed preflight is `preflight_failed`; it is not evidence that the sweep
path failed.

## Phase 2: Create and initialize

1. Deploy a fresh `EphemeralAccount` from `EA_WASM_HASH`.
2. Store the deployment transaction hash and new contract ID.
3. Initialize it with:
   - the canary creator;
   - `start_ledger + expiry_margin_ledgers`;
   - an approved recovery address; and
   - the canary `SweepController` as `authorized_controller`.
4. Store the initialization transaction hash.
5. Require `get_status() == 0` and confirm `get_info().expiry_ledger`.

Never reuse a pre-existing child account for a canary run.

## Phase 3: Pay

1. Transfer `canary_amount_stroops` from the funded payer to the new account
   through the native XLM SAC.
2. Wait for a definitive transaction result.
3. Read the new account's XLM SAC balance.
4. Require a balance of at least the exact configured amount.
5. Store the payment transaction hash and before/after balance evidence.

If a transfer result is ambiguous, quarantine the account. Do not transfer
again or record a payment merely because the request timed out.

## Phase 4: Record

Call:

```text
record_payment(
  canary_amount_stroops,
  native_xlm_sac_address
)
```

Then require:

- `get_status() == 1` (`PaymentReceived`);
- `get_payment_count() == 1`;
- `get_info().payments` contains the expected asset and amount; and
- `can_sweep(ephemeral_account) == true`.

`record_payment` records metadata. The canary succeeds only when Phase 3 proves
that the real token transfer also occurred.

## Phase 5: Authorize and sweep

Immediately before signing:

1. Read `SweepController::get_nonce`.
2. Construct the current-source digest:

   ```text
   SHA256(
     destination.to_xdr()
     || nonce as unsigned u64 in 8-byte big-endian order
     || sweep_controller_id.to_xdr()
   )
   ```

3. Sign the digest with the dedicated Ed25519 key.
4. Verify that the signer reports the expected public key.
5. Submit `execute_sweep` while holding the per-controller lock.
6. Store the transaction hash, nonce used, and submission time.

A submitted transaction remains pending until its result is known. Preserve
the signature as short-lived authorization material and do not log it.

## Phase 6: Verify success

All checks are required:

| Check | Pass condition |
|---|---|
| Transaction | Final status is successful |
| Account state | `get_status() == 2` (`Swept`) |
| Account destination | `get_info().swept_to` equals the requested address |
| Readiness | `can_sweep()` is `false` |
| Nonce | Final `get_nonce()` equals the initial value plus one |
| Account event | One `swept_mul` event matches the run ledger and account |
| Controller event | One `sweep` event names the same account and destination |
| Asset balance | Destination balance increased by the exact recorded amount |
| Source balance | The new account no longer holds the canary amount |

A controller `sweep` event alone is not proof of balance movement. Retain both
the transaction result and the SEP-41 balance evidence.

## Failure taxonomy

| Class | Examples | Handling |
|---|---|---|
| `preflight_failed` | Missing contract, wrong network, signer unavailable | Do not mutate; repair configuration |
| `create_failed` | Deployment or initialization failed | Retire the account; use a new run for a retry |
| `pay_failed` | SAC transfer rejected or result unknown | Resolve balance and transaction state first |
| `record_failed` | Duplicate, invalid amount, or unexpected state | Inspect `get_info`; do not blindly retry |
| `not_sweepable` | `can_sweep == false` or account expired | Fail preconditions and investigate |
| `signature_failed` | Host verification failure or stale nonce | Fetch a fresh nonce and sign under a new run |
| `sweep_failed` | Contract/resource/auth failure | Preserve the transaction and failure diagnostics |
| `verification_failed` | Successful transaction with wrong state or balance | Stop retries and investigate as an integrity incident |
| `timeout` | No final result before the deadline | Resolve the transaction before any retry |
| `reset_detected` | Ledger epoch or contract identity changed | Stop writes and require redeployment/reverification |

Soroban transactions are atomic. A reverted transaction must not leave the
canary nonce, account state, events, or transfer committed.

## Immutable result record

Store one result per `run_id` with at least:

- `run_id` and network-reset epoch;
- controller ID, account ID, and WASM hash;
- asset ID and exact amount;
- destination;
- `nonce_before` and `nonce_after`;
- deployment, initialization, payment, recording, and sweep transaction hashes;
- start/end ledgers and timestamps;
- terminal outcome and failure class;
- source/destination balances before and after; and
- observed event identifiers.

Do not use account IDs, transaction hashes, signatures, or nonces as metric
labels. Keep them in detailed run records and link to them from bounded metric
tables.

## Alerting proposal

- **Immediate investigation:** a successful transaction followed by an
  unexpected destination, state, event, or balance mismatch.
- **Warning:** one finalized canary failure, while automatic investigation
  gathers evidence.
- **Page proposal:** two consecutive finalized failures.
- **Page proposal:** no strict success within two schedule intervals plus the
  run timeout.
- **Informational:** a testnet reset requiring fresh contract verification.

No alert destination is configured in this repository.

## Activation checklist

- [ ] Verify current contract IDs and WASM after the latest testnet reset.
- [ ] Deploy and initialize a dedicated canary controller.
- [ ] Provision dedicated public identities and a secret-managed signer.
- [ ] Fix and integration-test any unresolved token-transfer authorization
      requirements for the deployed code.
- [ ] Implement overlap prevention and per-controller serialization.
- [ ] Record finalized successes and failures, including balance evidence.
- [ ] Test RPC, submission, and finalization timeouts.
- [ ] Document reset, redeployment, and signer-lockout handling.
- [ ] Approve the schedule, amount, owner, and alert routes.

Until these gates are complete, the operational status is:

```text
synthetic_canary: not_configured
```

## Related documents

- [`README.md`](README.md)
- [`dashboards-plan.md`](dashboards-plan.md)
- [`explorer-links.md`](explorer-links.md)
- [`../registry/event-topics.md`](../registry/event-topics.md)
- [`../config/nonce-tracking.md`](../config/nonce-tracking.md)
- [`../runbooks/failed-sweep-signature.md`](../runbooks/failed-sweep-signature.md)
