<!--
Purpose: Explain how replay protection works for execute_sweep signatures in Bridgelet Core, including message construction, nonce incrementation, and the claim() path caveat.
Owner: @ohamamarachi474-del
Status: Documentation-only. No contracts, docs, scripts, or tools changes are introduced by this file.
-->

# FAQ: How Does Replay Protection Work for `execute_sweep` Signatures?

| Field | Value |
| :--- | :--- |
| **Category** | Frequently Asked Questions (FAQ) |
| **Topic** | Cryptographic Replay Protection & Nonce Management |
| **Owner / reviewer** | `@ohamamarachi474-del` |
| **Last reviewed** | `2026-08-26` |

---

## Quick Answer

Replay protection for `SweepController::execute_sweep()` prevents an attacker from intercepting a valid authorization signature and submitting it a second time to re-sweep funds. It works via a **message-digest nonce binding** mechanism:

1. **State-Bound Digest**: The signed message payload is a SHA-256 hash containing the target ephemeral account address, the recipient destination address, the current 64-bit on-chain sweep nonce (`u64`), and the `SweepController` contract address.
2. **Atomic Nonce Increment**: Upon verifying the Ed25519 signature on-chain, `SweepController` immediately increments the sweep nonce (`nonce = nonce + 1`) in instance storage before dispatching downstream token transfers.
3. **Instant Signature Invalidation**: Advancing the nonce ensures that the message digest reconstructed on-chain for any subsequent invocation will differ from the original signed payload, causing repeated submissions to be rejected immediately.

---

## The Replay Problem in Off-Chain Authorizations

When smart contracts rely on off-chain cryptographic signatures (such as Ed25519 authorization keys held in an HSM or backend relayer), the signature proves that an authorized key approved an operation. However, a raw signature over static data (e.g., `(account, destination)`) is valid indefinitely. 

Without state-dependent replay protection:
- An observer could capture a signed transaction from the Stellar ledger or transaction pool.
- The observer could re-submit the identical payload to drain newly deposited funds or trigger unexpected state transitions.

Bridgelet Core prevents this vulnerability through a multi-layered replay defense model:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Bridgelet Replay Defenses                         │
├──────────────────────────┬──────────────────────────────────────────────────┤
│ Defense Layer            │ Mechanism & Target Invariant                     │
├──────────────────────────┼──────────────────────────────────────────────────┤
│ 1. Sweep Nonce           │ Monotonic counter hashed into signature payload  │
│ 2. Ephemeral State Guard │ Account transitions Active -> Swept (single use) │
│ 3. Domain Separation     │ Contract ID & Account ID bound in message digest │
└──────────────────────────┴──────────────────────────────────────────────────┘
```

---

## Summary Walkthrough of the `execute_sweep` Mechanism

### Step 1: Off-Chain Message Digest Construction & Signing
Before issuing a sweep transaction, the off-chain relayer or SDK must:
1. Query the live on-chain nonce via `SweepControllerClient::get_nonce()`.
2. Assemble the exact byte sequence in the canonical layout:
   $$\text{Payload} = \text{XDR}(\text{account}) \mathbin{\Vert} \text{XDR}(\text{destination}) \mathbin{\Vert} \text{nonce}_{\text{BE-u64}} \mathbin{\Vert} \text{XDR}(\text{contract\_id})$$
3. Compute the 32-byte SHA-256 hash over the payload:
   $$\text{Digest} = \text{SHA-256}(\text{Payload})$$
4. Sign the 32-byte digest using the authorized Ed25519 private key to produce a 64-byte signature.

### Step 2: On-Chain Reconstruction & Ed25519 Verification
When `SweepController::execute_sweep(env, ephemeral_account, destination, auth_signature)` is invoked:
1. **Retrieve Stored Nonce**: `authorization::construct_sweep_message()` reads `storage::get_sweep_nonce(env)`.
2. **Rebuild Digest**: The contract rebuilds the identical byte buffer using canonical XDR serialization for addresses, 8-byte big-endian encoding for the current nonce, and `env.current_contract_address()`.
3. **Verify Cryptographic Signature**: The contract calls `env.crypto().ed25519_verify(&authorized_signer, &digest, &auth_signature)`. If the signature does not match or the payload has been altered, the transaction panics and aborts.

### Step 3: Nonce Increment (Checks-Effects-Interactions)
Immediately following signature verification—and **prior** to invoking external transfers on `EphemeralAccount` or token contracts—the controller mutates state:
```rust
authorization::increment_nonce(&env);
```
This increments `SweepNonce` from $N$ to $N + 1$ in persistent contract storage.

### Step 4: Replay Rejection
If an attacker attempts to resubmit the exact same `auth_signature`:
- The on-chain contract reconstructs the message digest using the updated nonce ($N + 1$).
- Because $N + 1 \neq N$, the reconstructed SHA-256 digest does not match the digest over which the signature was generated.
- `env.crypto().ed25519_verify()` fails, reverting the transaction.

---

## Architecture Flow

```
[Off-Chain Signer]
       │  1. Read live nonce (N) from on-chain RPC
       │  2. Build SHA256(account || dest || N || controller_id)
       │  3. Sign digest with Ed25519 private key
       ▼
[SweepController::execute_sweep]
       │
       ├─► 4. construct_sweep_message(account, dest, controller_id) -> reads on-chain nonce (N)
       ├─► 5. ed25519_verify(authorized_signer, digest, signature)  -> OK
       ├─► 6. increment_nonce()                                     -> storage: nonce becomes N + 1
       └─► 7. EphemeralAccount::sweep()                             -> transfer tokens & reclaim reserve
```

---

## Important Caveat: The `claim()` Path Nonce Bypass

While `execute_sweep()` enforces signature replay protection via the sweep nonce, Bridgelet provides an alternative gas-free path: `claim()`.

### The Difference in Mechanism:
- **`execute_sweep()`**: Intended for relayer-driven sweeps. Authorizes via off-chain Ed25519 signature, checks the signed digest, and **increments the sweep nonce**.
- **`claim()`**: Intended for recipient-driven direct claims. Authorizes via Soroban native `recipient.require_auth()` and **does NOT read or increment the sweep nonce**.

### Security & Operational Implications:
1. **Fund Protection Intact**: Funds cannot be double-claimed through `claim()` because `EphemeralAccount` transitions its internal status to `AccountStatus::Swept`. Subsequent calls to `sweep()` or `sweep_claim()` will fail with `Error::AlreadySwept`.
2. **Destination Lock Bypass**: `SweepController::update_authorized_destination()` uses `nonce > 0` as a guard to lock destination address modifications post-sweep. Because `claim()` leaves `nonce` at `0`, an admin can still alter `authorized_destination` even after accounts have been swept via `claim()`.
3. **Off-Chain Integrator Rule**: Systems auditing sweep status or relying on the destination immutability guarantee must not check `get_nonce() > 0` alone; they must cross-reference `AccountStatus` and emitted `SweepExecutedMulti` / `SweepCompleted` events.

---

## Deeper Technical References

For detailed byte layouts, state transition rules, and postmortem analysis:

- [`bridgelet-audit/glossary/sha256-message-construction.md`](../glossary/sha256-message-construction.md) — Canonical byte-level layout of the sweep message and domain separation rules.
- [`bridgelet-audit/glossary/sweep-nonce.md`](../glossary/sweep-nonce.md) — Storage mechanics, big-endian encoding details, and live-query rules for off-chain signers.
- [`bridgelet-audit/postmortems/claim-nonce-bypass.md`](../postmortems/claim-nonce-bypass.md) — Comprehensive postmortem analyzing how `claim()` bypasses the sweep nonce and affects the destination lock.
- [`bridgelet-audit/glossary/replay-protection.md`](../glossary/replay-protection.md) — High-level conceptual overview of replay defense patterns in the protocol.
- [`bridgelet-audit/glossary/sweep-vs-sweep-claim.md`](../glossary/sweep-vs-sweep-claim.md) — Architectural comparison between `execute_sweep()` and `claim()`.
- [`bridgelet-audit/threat-models/replay-nonce-protections.md`](../threat-models/replay-nonce-protections.md) — Threat model matrix evaluating replay coverage across all system operations.
- [`bridgelet-audit/checklists/replay-protection-checklist.md`](../checklists/replay-protection-checklist.md) — Production deployment checklist for verifying nonce and status guards.
