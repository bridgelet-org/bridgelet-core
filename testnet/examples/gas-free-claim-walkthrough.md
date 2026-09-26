# Worked Example: Gas-Free `claim()` — Recipient Signs, Relayer Submits

## Overview

`SweepController::claim()` is architecturally different from `execute_sweep()`, and that difference is the whole reason it exists:

| | `execute_sweep()` | `claim()` |
|---|---|---|
| **Who authorizes** | A signing key, via a 64-byte Ed25519 signature over a custom digest | The **recipient**, via a native Soroban auth entry |
| **Auth mechanism** | `env.crypto().ed25519_verify()` against `authorized_signer` | `recipient.require_auth()` on the contract call itself |
| **Custom digest to construct** | Yes — `SHA256(dest ‖ nonce ‖ contract_id)` | **No** — the network handles it |
| **Fee payer** | Relayer | Relayer |
| **Consumes the nonce** | Yes | Yes |

The upshot for integrators: **there is no off-chain signing service and no message-construction step.** The recipient signs a standard Soroban auth entry, and any funded third party submits the transaction and eats the fee. The recipient never needs XLM of their own.

If you already have a working signed-sweep flow, you probably do not need this. If you do not want to run an Ed25519 signer at all, this is the path.

> Configuration-side setup for this path — identity setup, locked vs. flexible mode, expected errors — is covered in `testnet/config/gas-free-claim-testing.md`. This document is the worked end-to-end example, including how the auth entry is actually built and signed.

---

## How the Authorization Actually Works

This is the part that costs integrators the most trial-and-error, so it is worth being precise.

`claim()` does three things:

1. `recipient.require_auth()` — the recipient authorizes *this exact contract call*.
2. `validate_destination(&recipient)` — locked-mode check, if the controller has one.
3. `authorize_claim()` — builds an `InvokerContractAuthEntry` for a **sub-contract invocation** of `sweep_claim` on the `EphemeralAccount`, and satisfies it with `env.authorize_as_current_contract()`.

Step 3 is the clever part. `EphemeralAccount::sweep_claim()` requires its `authorized_controller` to authorize. `SweepController` *is* that controller, and a contract can authorize itself as the invoker. So:

```
recipient  --Soroban auth entry-->  SweepController::claim()
                                          |
                                          | authorize_as_current_contract()
                                          v
                             EphemeralAccount::sweep_claim(destination)
```

The recipient's signature therefore transitively satisfies the ephemeral account's controller check. **The recipient's auth entry is what proves they authorized this** — there is no separate signature check anywhere in this path.

> `sweep_claim` is invoked under the host-symbol name `swp_claim` (9 characters, the `symbol_short!` limit). If you are constructing the sub-invocation context yourself in a host environment or an unusual SDK, `swp_claim` is what appears on the wire, not `sweep_claim`.

---

## Prerequisites

Two funded testnet identities:

| Identity | Role |
|---|---|
| `testnet-recipient` | Signs the auth entry. Receives the funds. |
| `testnet-relayer` | Submits the transaction. Pays all fees. |

```bash
stellar keys generate --global testnet-recipient
stellar keys fund testnet-recipient --network testnet

stellar keys generate --global testnet-relayer
stellar keys fund testnet-relayer --network testnet

RECIPIENT=$(stellar keys address testnet-recipient)
RELAYER=$(stellar keys address testnet-relayer)
echo "recipient=$RECIPIENT relayer=$RELAYER"
```

Plus an `EphemeralAccount` that is initialized, has a payment recorded, and has not expired. The controller must be flexible mode (`authorized_destination = None`) unless the recipient *is* the locked destination.

---

## Step 1: Build the Auth Entry

A Soroban auth entry is not a free-form signature. It names the contract, the function, and the arguments — and the host binds the signature to exactly those. Change any argument and the signature no longer applies.

For `claim(recipient, ephemeral_account)` the entry is conceptually:

```json
{
  "credentials": {
    "address": "<RECIPIENT_ADDRESS>",
    "invocation": {
      "function": "claim",
      "subinvocations": []
    }
  },
  "root_invocation": {
    "function": "claim",
    "subinvocations": []
  }
}
```

Concretely, the `SorobanAuthorizationEntry` the recipient signs covers:

- **Credentials address** — the recipient's `G...` address. This is what `require_auth()` matches against.
- **Root invocation** — contract `CBEU...` (the `SweepController`), function `claim`, arguments `[recipient, ephemeral_account]`, no sub-invocations.
- **Signature** — over the SHA-256 of the canonical `SorobanAuthorizationEntry` XDR, with the account's own domain-separated scheme. **This is account-key signing, not the sweep digest**, and the two are not interchangeable.

### Building It in an SDK

In the JS SDK the entry is assembled alongside the transaction and the account signs the operation:

```typescript
import { SorobanRpc, Contract, Networks, TransactionBuilder, Operation } from '@stellar/stellar-sdk';

const tx = new TransactionBuilder(account, { fee: '100', networkPassphrase: Networks.TESTNET })
  .addOperation(new Operation.Contract({ contract, source: new Account(ephemeralId, '0') }))
  .setTimeout(300)
  .build();

// The account signs the operation; Soroban auth entries are derived and
// attached from that signature. No custom digest is constructed here.
await tx.sign(recipientKeypair).sign(relayerKeypair);
```

The recipient signature comes from `tx.sign(recipientKeypair)`; the relayer signature covers the envelope and pays the fee. Both signatures end up in one envelope — that is what makes the flow "gas-free" for the recipient.

### What `stellar contract invoke` Does For You

The CLI handles the entry construction, so for a manual test you do not build any of the above:

```bash
stellar contract invoke \
  --id "$SWEEP_CONTROLLER_ID" \
  --network testnet \
  --source testnet-relayer \
  -- \
  claim \
  --recipient "$RECIPIENT" \
  --ephemeral_account "$EPHEMERAL_ID"
```

**`--source` is the relayer**, and that is the entire trick. The CLI attaches a `SorobanCredentials::Address` auth entry naming `$RECIPIENT` for the `claim` invocation, and — because the recipient's secret is not available to the CLI — this succeeds only if the recipient has separately signed that entry, or if you are invoking with the recipient's own key available to the CLI.

In practice, for a manual walkthrough the reliable shapes are:

- **Recipient signs offline, relayer submits** — build the envelope in an SDK, have the recipient sign the operation, then broadcast from the relayer. This is the real production shape.
- **Recipient is also the fee payer** — `--source testnet-recipient` and the CLI supplies everything. Simple, but it defeats the purpose (the recipient pays fees, so they need XLM).

> **Do not pass `--source testnet-recipient` and expect the relayer to be free.** The `--source` identity pays the fee. The "gas-free" property is about *who signs the authorization*, not about fees disappearing — a relayer always pays, and someone must fund it.

---

## Step 2: Submit

Envelope assembled and both signatures attached:

```typescript
const result = await server.sendTransaction(tx);
```

Or, purely for a manual smoke test where the recipient's key is available to the CLI:

```bash
# Relayer-funded submission of a pre-signed envelope
stellar contract invoke \
  --id "$SWEEP_CONTROLLER_ID" \
  --network testnet \
  --source testnet-relayer \
  -- \
  claim \
  --recipient "$RECIPIENT" \
  --ephemeral_account "$EPHEMERAL_ID"
```

On success the controller emits `SweepCompleted { ephemeral_account, destination: recipient, amount }` and the ephemeral account emits `SweepExecutedMulti` plus a `ReserveReclaimed`.

---

## Step 3: Verify

```bash
# Status is Swept
stellar contract invoke --id "$EPHEMERAL_ID" --network testnet \
  --source testnet-investigator -- get_status
# → 2 (Swept)

# Nonce advanced by exactly 1
BEFORE=$(stellar contract invoke --id "$SWEEP_CONTROLLER_ID" --network testnet \
  --source testnet-investigator -- get_nonce)
# ... claim ...
AFTER=$(stellar contract invoke --id "$SWEEP_CONTROLLER_ID" --network testnet \
  --source testnet-investigator -- get_nonce)
echo "$BEFORE -> $AFTER"    # expect 7 -> 8

# Recipient balance increased
```

The nonce increment matters more here than it looks: `claim()` and `execute_sweep()` draw from the **same** counter. A `claim()` will invalidate a signature you are concurrently holding for `execute_sweep()`. This is the same shared-nonce hazard described in `testnet/config/nonce-tracking.md`.

---

## Locked Mode

If the controller was initialized with `authorized_destination`, only that address may claim:

```bash
# Succeeds — recipient IS the locked destination
stellar contract invoke --id "$SWEEP_CONTROLLER_ID" --network testnet \
  --source testnet-relayer -- claim \
  --recipient "$LOCKED_DESTINATION" --ephemeral_account "$EPHEMERAL_ID"

# Fails with UnauthorizedDestination
stellar contract invoke --id "$SWEEP_CONTROLLER_ID" --network testnet \
  --source testnet-relayer -- claim \
  --recipient "$SOMEONE_ELSE" --ephemeral_account "$EPHEMERAL_ID_2"
# → Error::UnauthorizedDestination (13)
```

This is a meaningful security difference from the signed path: **the recipient's own signature cannot override locked mode.** The destination check runs after `require_auth()` and is not signer-dependent. In flexible mode, whoever signs the auth entry chooses the destination, so the recipient address *is* the destination — there is no separate "destination" parameter to disagree with.

---

## Failure Modes

| Symptom | Cause | Fix |
|---|---|---|
| Auth failure, no specific error | Recipient did not sign the `claim` invocation | Ensure the auth entry names `claim` with args `[recipient, ephemeral_account]` |
| Auth failure, signature rejected | Entry was signed but an argument differs | Every argument is bound; `recipient` and `ephemeral_account` must match exactly |
| `UnauthorizedDestination` | Locked mode, recipient ≠ locked destination | Use the locked address, or change it while nonce is still 0 |
| `AccountNotReady` | No payment recorded, or summed amount is 0 | `record_payment()` first |
| `AccountExpired` | Past `expiry_ledger` | Expired accounts go to `recovery_address`; see `testnet/examples/expiry-and-recovery.md` |
| Trap / host error | `get_info()` on an uninitialized or non-contract address | Check the ephemeral account ID |
| Rejected at submission | Relayer unfunded | `stellar keys fund testnet-relayer --network testnet` |

The most confusing case is the first two, because a missing or mismatched auth entry surfaces as a bare auth failure rather than a named error. If `claim()` fails auth, compare the signed entry's contract, function name, and arguments against the invocation you are actually submitting before suspecting the contracts.

---

## Choosing Between This and `execute_sweep()`

Use `claim()` when:

- The recipient is a real user who can sign a transaction but should not hold XLM.
- You are building a relayer and want recipients to be self-authorizing.
- You have no off-chain Ed25519 signing infrastructure.

Use `execute_sweep()` when:

- The authorization decision is made **server-side** (a backend decides the destination; the user has no key in the flow at all).
- You need the destination to be enforced independently of who signs.
- You need a signature that survives independently of the submitting transaction.

The signed path is the more common integration. `claim()` is a genuinely different shape, not a cheaper variant.

---

## Related Documentation

- `testnet/config/gas-free-claim-testing.md` — configuration and scenario matrix for the claim path
- `testnet/config/nonce-tracking.md` — the shared nonce `claim()` also consumes
- `testnet/examples/signed-sweep-walkthrough.md` — the signed-sweep alternative, end to end
- `testnet/examples/multi-asset-sweep.md` — multi-asset sweep behaviour
- `docs/api-reference.md` — `claim()` API reference
- `docs/architecture.md` — cross-contract call graph

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-26 | Initial claim() walkthrough |
