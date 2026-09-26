# Worked Example: Signed Sweep via `SweepController::execute_sweep()`

## Overview

This is the **primary** way to move funds out of an `EphemeralAccount`, and the only path that performs real Ed25519 signature verification. This walkthrough covers the three things integrators usually get wrong:

1. Constructing the message as `SHA256(destination.to_xdr() || nonce_be_u64 || contract_id.to_xdr())` — the exact order and encoding the contract computes.
2. Obtaining a signature without hand-rolling an XDR encoder.
3. Submitting `execute_sweep()` and recognising success vs. each common failure.

> **Deploy an `EphemeralAccount` first.** This walkthrough starts from an initialized account that has recorded at least one payment. The create-then-pay flow is covered separately in `testnet/examples/create-and-fund-ephemeral-account.md`; if that document is not in your checkout yet, `testnet/config/expiry-ledger-testing.md` has the exact `deploy` + `initialize` + `record_payment` commands. Contract IDs: `testnet/registry/contract-status.md`.

> **Non-obvious:** do **not** try to call `EphemeralAccount::sweep()` directly. That function accepts an `auth_signature` argument and then ignores it — it only checks that the caller is the `authorized_controller`. Calling it directly gives you a *false* sense that signature checking works. See `testnet/security/unverified-signature-path-warning.md`.

---

## The Signed Message

### Exact Format

The contract builds the digest in `contracts/sweep_controller/src/authorization.rs::construct_sweep_message()`:

```
message = SHA256(
    destination.to_xdr()      // Soroban SDK XDR bytes of the destination Address
 || nonce.to_be_bytes()       // u64, 8 bytes, big-endian
 || contract_id.to_xdr()      // Soroban SDK XDR bytes of the *SweepController* address
)

signature = Ed25519_sign(message, authorized_signer_private_key)
```

There are exactly three components. **There is no timestamp, no expiry, and no account address in the signed payload.** Older revisions of `docs/SIGNATURE_FORMAT.md` included a timestamp; that was wrong and has been corrected.

### Component Gotchas

| Component | Requirement | Common mistake |
|---|---|---|
| `destination.to_xdr()` | Soroban SDK's own serialization | Hand-rolling XDR, or hashing the `G...` strkey string |
| `nonce` | Read from the **live** contract via `get_nonce()` | Using a locally tracked counter |
| `contract_id` | The `SweepController` address, **not** the `EphemeralAccount` | Using the ephemeral account ID, or a mainnet ID |

The `contract_id` component is what stops a signature for one deployment from being replayed against another. If you deploy a new `SweepController` and reuse the same signer key, **all previously issued signatures are dead** because they were bound to the old contract address.

### Why You Should Not Hand-Roll the XDR

`Address::to_xdr()` produces `ScVal::Address` XDR — a discriminant byte, a 4-byte length, then the payload. The discriminant differs between account addresses (`G...`) and contract addresses (`C...`). Getting it subtly wrong produces a signature that fails verification with **no useful error message** (see [Failure Modes](#failure-modes)).

Use `tools/sweep-signer`, which uses `soroban-sdk` itself to serialize addresses, so the bytes are guaranteed to match what the contract computes on-chain.

---

## Step 1: Read the Current Nonce

The contract always verifies against its own current on-chain nonce. Read it immediately before signing.

```bash
SWEEP_CONTROLLER_ID=<SWEEP_CONTROLLER_CONTRACT_ID>

NONCE=$(stellar contract invoke \
  --id "$SWEEP_CONTROLLER_ID" \
  --network testnet \
  --source testnet-investigator \
  -- \
  get_nonce)

echo "Current nonce: $NONCE"
# → Current nonce: 7
```

A read-only invoke needs no funds beyond the identity's own balance, so a separate `testnet-investigator` identity is enough — the sweep's fee payer can be a different account entirely.

**Do not cache this value.** The gap between reading the nonce and submitting the sweep is the entire race window. See [Race Conditions](#race-conditions).

---

## Step 2: Sign the Message

### One-Time: Derive the Public Key

If you have not yet registered a signer, derive the public key **before** deploying, and pass it to `SweepController::initialize()` as `authorized_signer`:

```bash
# Generate a signing-only seed (this key never needs to be a funded Stellar account)
SEED=$(node -e "console.log(require('crypto').randomBytes(32).toString('hex'))")
echo "$SEED"   # store securely; this is the key that can authorize sweeps

# Derive the public key to register
cargo run --quiet --manifest-path tools/sweep-signer/Cargo.toml -- \
  pubkey --signer-seed-hex "$SEED"
# → Public key (hex) - put this in AUTHORIZED_SIGNER_PUBLIC_KEY:
# → 16ac79d642e33ac696e822cc7175ad7ddd5587a9dd8b6942640cbb832a1ab4bf
```

This same command is what the repo's `.env.example` documents as `AUTHORIZED_SIGNER_PUBLIC_KEY`.

### Per Sweep: Produce the Signature

```bash
DESTINATION=<RECIPIENT_G_ADDRESS>

SIG=$(cargo run --quiet --manifest-path tools/sweep-signer/Cargo.toml -- \
  sign \
  --contract-id "$SWEEP_CONTROLLER_ID" \
  --destination "$DESTINATION" \
  --nonce "$NONCE" \
  --signer-seed-hex "$SEED" \
  | grep '^auth_signature' \
  | awk '{print $NF}')

echo "Signature: $SIG"
```

`sign` also prints the signer's public key and the nonce it used — both are worth reading rather than piping straight through, because they are the two values most likely to be wrong.

> `tools/sweep-signer` is referenced here but not modified by this document. It accepts the key as either `--signer-seed-hex` (raw 32-byte seed, recommended) or `--signer-secret` (an `S...` strkey). It performs no network I/O.

**The signing key does not need to be a funded Stellar account.** It is a signing-only Ed25519 key. Only the *relayer* that submits `execute_sweep()` needs XLM.

---

## Step 3: Pre-Flight (Optional but Cheap)

`EphemeralAccount::simulate_sweep()` reports what a sweep *would* move and whether anything would block it, without changing state:

```bash
stellar contract invoke \
  --id "$EPHEMERAL_ID" \
  --network testnet \
  --source testnet-investigator \
  -- \
  simulate_sweep \
  --destination "$DESTINATION"
# → (payments, error_code) where error_code == 0 means no blocking error
```

`simulate_sweep()` does **not** check the signature, so a passing simulation says nothing about whether your signature is valid. It rules out state problems only.

There is also a controller-side readiness check:

```bash
stellar contract invoke \
  --id "$SWEEP_CONTROLLER_ID" \
  --network testnet \
  --source testnet-investigator \
  -- \
  can_sweep \
  --ephemeral_account "$EPHEMERAL_ID"
# → true
```

---

## Step 4: Submit the Sweep

The fee payer (`--source`) needs XLM and needs **no** signature-authorizing role — it is just relaying a pre-authorized sweep.

```bash
stellar contract invoke \
  --id "$SWEEP_CONTROLLER_ID" \
  --network testnet \
  --source testnet-relayer \
  -- \
  execute_sweep \
  --ephemeral_account "$EPHEMERAL_ID" \
  --destination "$DESTINATION" \
  --auth_signature "$SIG"
```

### What Happens On-Chain

1. `validate_destination()` — if the controller was initialized with a locked `authorized_destination`, this must match exactly.
2. `AuthContext::verify()` — reconstructs the digest from the **current on-chain nonce** and calls `ed25519_verify`. A failure here traps.
3. `increment_nonce()` — burns the nonce, invalidating this signature for reuse.
4. `authorize_as_current_contract()` — authorizes the `SweepController` as the invoker of the ephemeral account's `sweep`.
5. `EphemeralAccount::sweep()` — checks state, flips status to `Swept`, emits `SweepExecutedMulti`, reclaims reserve.
6. `transfers::execute_transfers()` — calls SEP-41 `TokenClient::transfer()` once per recorded payment.
7. `SweepCompleted` is emitted on the controller.

Note the ordering: **the nonce is incremented before the transfer**. If a transfer later fails, the whole transaction reverts atomically and the nonce is *not* burned — so a failed sweep does not consume a nonce.

---

## Step 5: Verify Success

```bash
# Account status is now Swept
stellar contract invoke \
  --id "$EPHEMERAL_ID" \
  --network testnet \
  --source testnet-investigator \
  -- \
  get_status
# → 2 (Swept)

# Nonce advanced
stellar contract invoke \
  --id "$SWEEP_CONTROLLER_ID" \
  --network testnet \
  --source testnet-investigator \
  -- \
  get_nonce
# → 8

# The controller emitted SweepCompleted
stellar contract events \
  --contract-id "$SWEEP_CONTROLLER_ID" \
  --start-ledger "$START_LEDGER" \
  --filter '{"topics": [["sweep"]]}' \
  --network testnet
```

`SweepCompleted.amount` is the **sum of all recorded payment amounts across every asset**. With one USDC payment and one XLM payment it is the arithmetic sum of both, which is not a meaningful monetary figure — do not present it to users as "the amount swept." The per-asset breakdown is in the `SweepExecutedMulti.payments` vector on the `EphemeralAccount` contract. See `testnet/examples/multi-asset-sweep.md`.

---

## Expected Outputs

| Situation | Result |
|---|---|
| Happy path | `SweepCompleted` + `SweepExecutedMulti` events; status `Swept`; nonce +1 |
| Destination mismatch vs. locked mode | `UnauthorizedDestination` (code 13) |
| Bad/stale signature | Transaction traps in `ed25519_verify`; no error code returned |
| Account already swept | `AlreadySwept` (ephemeral, code 7) |
| Past `expiry_ledger` | `AccountExpired` (ephemeral, code 11) |
| No payment recorded | `NoPaymentReceived` (ephemeral, code 10) |
| Token transfer rejected | `TransferFailed` (controller, code 2) |
| Controller not initialized | `AuthorizedSignerNotSet` (code 10) |

---

## Failure Modes

### Stale Nonce

The most common failure by a wide margin. You read nonce 7, someone else's sweep landed, the contract is now at 8, and your signature was built over 7.

The symptom is a **trap with no error code** — the host `ed25519_verify` function fails and aborts the transaction, so there is no `Error` variant to match on. Re-read `get_nonce()` and re-sign.

Full diagnosis: `testnet/runbooks/failed-sweep-signature.md`.

### Wrong Contract ID in the Digest

Signing against a locally-run controller, a different testnet deployment, or mainnet. The digest binds to `contract_id`, so verification fails exactly as with a bad signature. Check that `--contract-id` matches the ID you pass to `stellar contract invoke --id`.

### Wrong Signing Key

`authorized_signer` is baked in at `initialize()` and has no getter and no rotation function. If the stored key differs from the one you are signing with, every signature fails. `sweep-signer sign` prints the public key it used — compare it against the value in `deployments/testnet.json` under `config.authorizedSigner`.

### Locked-Mode Destination

If the controller was initialized with an `authorized_destination`, only that address is accepted. This surfaces as `UnauthorizedDestination`, checked *before* signature verification, so it is a clean error rather than a trap.

While the nonce is still 0 the creator can correct this with `update_authorized_destination()`. Once any sweep has succeeded the nonce is > 0 and that call fails with `AccountAlreadySwept` — the lock is permanent.

---

## Race Conditions

`SweepController` has a single global nonce shared by **all** ephemeral accounts it controls. Any concurrent sweep — yours, another integrator's, a load test — consumes it.

```
Signer A reads nonce 7 ──┐
Signer B reads nonce 7 ──┼──► Both build a valid-looking signature
Signer A submits  ────────┤    A succeeds, nonce → 8
Signer B submits  ────────┘    B traps in ed25519_verify
```

**The testnet controller is shared**, so this is not hypothetical — see `testnet/security/known-testnet-abuse-patterns.md`. If you run parallel sweeps:

- Serialize signing per controller (a mutex or a single-flight queue), and
- Treat an `ed25519_verify` trap as a signal to re-read the nonce and retry, not as a hard failure.

Do not build a retry loop that hammers a shared controller; back off and cap attempts.

---

## Full Script

```bash
#!/bin/bash
# signed-sweep.sh - one signed sweep against the shared testnet controller
set -euo pipefail

SWEEP_CONTROLLER_ID="${SWEEP_CONTROLLER_ID:?set to the testnet SweepController ID}"
EPHEMERAL_ID="${EPHEMERAL_ID:?set to the initialized EphemeralAccount ID}"
DESTINATION="${DESTINATION:?set to the recipient G address}"
SEED="${SEED:?set to the authorized signer seed (64 hex chars)}"

# 1. Read the live nonce immediately before signing
NONCE=$(stellar contract invoke \
  --id "$SWEEP_CONTROLLER_ID" --network testnet --source testnet-investigator \
  -- get_nonce)
echo ">> nonce: $NONCE"

# 2. Sign
SIG=$(cargo run --quiet --manifest-path tools/sweep-signer/Cargo.toml -- sign \
  --contract-id "$SWEEP_CONTROLLER_ID" \
  --destination "$DESTINATION" \
  --nonce "$NONCE" \
  --signer-seed-hex "$SEED" \
  | grep '^auth_signature' | awk '{print $NF}')

# 3. Submit (relayer pays fees; needs XLM but no special role)
stellar contract invoke \
  --id "$SWEEP_CONTROLLER_ID" --network testnet --source testnet-relayer \
  -- execute_sweep \
  --ephemeral_account "$EPHEMERAL_ID" \
  --destination "$DESTINATION" \
  --auth_signature "$SIG"

# 4. Verify
stellar contract invoke \
  --id "$EPHEMERAL_ID" --network testnet --source testnet-investigator \
  -- get_status   # expect 2 (Swept)
```

---

## Related Documentation

- `docs/SIGNATURE_FORMAT.md` — message format spec, with per-language examples
- `testnet/config/nonce-tracking.md` — nonce behaviour and pitfalls in depth
- `testnet/runbooks/failed-sweep-signature.md` — diagnosing a rejected signature
- `testnet/examples/gas-free-claim-walkthrough.md` — the recipient-signs/relayer-submits alternative
- `testnet/examples/multi-asset-sweep.md` — sweeping more than one asset
- `testnet/security/unverified-signature-path-warning.md` — why not to call `EphemeralAccount::sweep()` directly
- `tools/sweep-signer/` — the signing tool (referenced, not modified here)

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-26 | Initial signed sweep walkthrough |
