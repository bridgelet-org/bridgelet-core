# ⚠️ Warning: `EphemeralAccount::sweep()` Does Not Verify Its Signature

> **Read this before you test-call `EphemeralAccount::sweep()` directly against testnet.**
>
> `sweep()` takes an `auth_signature: BytesN<64>` argument. **It does not verify it.** The parameter is named `_signature` and discarded. A successful call does not mean any signature was checked, and passing garbage will not be rejected.
>
> Always sweep through `SweepController::execute_sweep()` or `::claim()`.

## Why this document exists

`EphemeralAccount::sweep()` looks like the safe, obvious thing to call. It takes a destination and a signature, it has a clear name, and if you call it and it succeeds, the natural conclusion is *"signature verification is working."*

**That conclusion is wrong.** The function performs no cryptographic verification whatsoever.

This is an accurate description of the deployed behaviour, not a criticism of it. `SweepController` is the component that verifies signatures, and it is the path that is meant to be used. The risk here is entirely about what an integrator concludes while experimenting on testnet.

## The actual code

From `contracts/ephemeral_account/src/lib.rs`:

```rust
pub fn sweep(env: Env, destination: Address, auth_signature: BytesN<64>) -> Result<(), Error> {
    // ... state checks: initialized, not already swept, payment received, not expired ...

    // Verify authorization signature
    // Note: In production, implement proper signature verification
    // For MVP, we trust the SDK to only call with valid signatures
    Self::verify_sweep_authorization(&env, &destination, &auth_signature)?;

    // ... state transition, events, reserve reclaim ...
}
```

And the function it delegates to:

```rust
fn verify_sweep_authorization(
    env: &Env,
    _destination: &Address,
    _signature: &BytesN<64>,      // <-- underscore: never read
) -> Result<(), Error> {
    let controller = storage::get_authorized_controller(env).ok_or(Error::Unauthorized)?;
    controller.require_auth();     // <-- the entire "verification"
    Ok(())
}
```

The signature of `verify_sweep_authorization` tells the story: both `_destination` and `_signature` are unused. Its entire body is a `require_auth()` on the stored `authorized_controller`.

## The dangerous part: it *does* fail, and that failure is misleading

If you call `sweep()` directly from an EOA, it **fails** with `Error::Unauthorized` (8) — because your address is not the `authorized_controller`, and `require_auth()` rejects it.

This is the trap. The most natural experiment produces:

> "I called `sweep()` with a random 64-byte value, and it was **rejected**. So signature checking must be working — the bad signature was caught."

It was not the signature that was rejected. It was `require_auth()`. **The function never got as far as looking at the signature.** A rejection from `sweep()` tells you nothing whatsoever about signature verification, because no signature verification exists on that path.

### The converse is the real hazard

The other direction is worse. If the caller *is* the `authorized_controller` — which is the `SweepController` contract ID, in a normal deployment — then `sweep()` succeeds **regardless of what is in `auth_signature`**. Any 64 bytes will do.

So the property you must not rely on is:

> ~~"`EphemeralAccount::sweep()` only succeeds with a valid signature."~~

It does not. It only succeeds when called by the controller, and it never inspects the signature.

## Why it is not directly exploitable

Stating this precisely matters, because overstating the risk trains people to ignore the warning.

`EphemeralAccount` contains **no `TokenClient::transfer()` call anywhere.** No SEP-41 tokens move in this function. Its own comment is explicit: *"Actual token transfers happen in the SDK via Stellar SDK. This contract enforces authorization/state transitions and reserve lifecycle."*

That means a successful direct `sweep()` call does **not** move anyone's funds. It flips status to `Swept`, records `swept_to`, emits `SweepExecutedMulti`, and reclaims a reserve counter. The balances stay where they are.

The same applies to the reserve reclaim: `reclaim_reserve_to()` only decrements internal counters and emits `ReserveReclaimed`. It transfers nothing.

**Consequence: there is no fund-theft path through this gap on the current implementation.** The impact is:

| Impact | Real? |
|---|---|
| Stealing funds by forging a signature | **No** — no tokens move, and the controller gate still applies |
| Sweeping someone else's account directly | **No** — `require_auth()` on the controller prevents it |
| Silently draining via a crafted signature | **No** — the signature is never read |
| **Concluding signature verification exists when it does not** | **Yes — this is the actual risk** |
| **Building an integration that depends on a false security property** | **Yes** |
| **Making a security assessment that is wrong in the unsafe direction** | **Yes** |

The last three are the ones that matter. The failure is in what people *believe* about the system, and that belief persists into code, audits, and architecture.

## What the real verification looks like

`SweepController` does implement real Ed25519 verification, in `contracts/sweep_controller/src/authorization.rs`:

```rust
let message = construct_sweep_message(env, destination, &contract_id);
env.crypto().ed25519_verify(&authorized_signer, &message.into(), signature);
```

Constructing that message requires XDR-serialising two `Address` values and reading the live on-chain nonce. That is non-trivial, which is exactly why the direct path is tempting — and exactly why `tools/sweep-signer` exists.

| | `EphemeralAccount::sweep()` | `SweepController::execute_sweep()` |
|---|---|---|
| Signature checked | **No** | **Yes** — `ed25519_verify` |
| Message to construct | None | `SHA256(dest ‖ nonce ‖ contract_id)` |
| Authorization | `authorized_controller.require_auth()` | Ed25519 + controller sub-invocation auth |
| Nonce consumed | No | Yes |
| SEP-41 transfers executed | **No** | **Yes** — `TokenClient::transfer()` per payment |
| `SweepCompleted` emitted | No | Yes |
| **Use this** | **No** | **Yes** |

## Cross-reference: the repository's own stub inventory

The root [`README.md`](../../README.md) documents this under **MVP Status → Current Stub Inventory**, in the row for `EphemeralAccount::verify_sweep_authorization`:

| Function | Contract | Status |
|---|---|---|
| `verify_sweep_authorization` | `EphemeralAccount` | **Not a real signature check** |
| `verify_sweep_auth` | `SweepController` | **Fully implemented** |
| `execute_transfers` | `SweepController` | **Fully implemented** |
| `batch_initialize` | `AccountFactory` | Implemented, error detail dropped |

The README's Implementation Notes say: *"**Always deploy sweeps through `SweepController::execute_sweep()` or `::claim()`.** Calling `EphemeralAccount::sweep()` directly bypasses the Ed25519 check entirely - it only works at all because of the `authorized_controller.require_auth()` gate, not because the signature was verified."*

[`docs/security.md`](../../docs/security.md) says the same thing under Known Limitations, and under Best Practices for Integrators: *"Never call `EphemeralAccount::sweep` directly, as it currently lacks active signature verification."*

If you take one thing from this document: **the stub inventory table in the README is the authoritative statement of what is and is not real in this codebase.** Check it before building on any function.

## If you are testing against the live deployment

Practical rules while experimenting on testnet:

1. **Always route through `SweepController`.** `execute_sweep()` for the signed path, `claim()` for the recipient-signs/relayer-submits path.
2. **Never treat a `sweep()` rejection as a signature check.** Read the error: `Unauthorized` (8) is the controller gate, not signature verification.
3. **Never treat a `sweep()` success as evidence of authorization.** If you somehow get a success, the only thing that was proven is that the caller was the controller.
4. **Never pass a signature you obtained from someone else into `sweep()` and infer anything from the result.** Nothing about it is checked.
5. **If you are writing a security assessment, a threat model, or an audit scope for this codebase, this gap must be in it** — with the accurate framing: not directly exploitable for theft today, because no tokens move in `ephemeral_account`; but a false security property that will become a live fund-theft vulnerability the moment token transfers are added to that contract without also adding real verification.
6. **That last point is the forward-looking risk.** The gap is a stub waiting for an implementation. If transfers are ever added to `EphemeralAccount::sweep()` before verification is, the same call becomes a signature-bypass drain. Any change to that function should be reviewed as a security change, not a feature change.

## Verifying this yourself on testnet

Once `EphemeralAccount` is deployed (see `testnet/registry/contract-status.md`):

```bash
# 1. Direct call from an EOA → Unauthorized (8).
#    This is require_auth(), NOT signature verification.
stellar contract invoke --id "$EPHEMERAL_ID" --network testnet \
  --source testnet-investigator -- sweep \
  --destination "$DESTINATION" \
  --auth_signature 00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
# → Error::Unauthorized (8)
# All-zero signature was NOT rejected as invalid. It was never examined.

# 2. Through the controller with a real signature → succeeds, tokens move.
#    This is the path with actual verification.
#    See testnet/examples/signed-sweep-walkthrough.md
```

## Related documentation

- [`README.md`](../../README.md) — **Current Stub Inventory**: the authoritative implementation-status table
- [`docs/security.md`](../../docs/security.md) — design-level security model and known limitations
- [`docs/SIGNATURE_FORMAT.md`](../../docs/SIGNATURE_FORMAT.md) — the message format that *is* verified
- `tools/sweep-signer/` — generates signatures the contract actually checks
- `testnet/examples/signed-sweep-walkthrough.md` — the correct sweep path, end to end
- `testnet/examples/gas-free-claim-walkthrough.md` — the other correct path
- `testnet/security/README.md` — index for this directory
- `contracts/ephemeral_account/src/lib.rs` — `verify_sweep_authorization`, ~line 643
- `contracts/sweep_controller/src/authorization.rs` — `verify_sweep_auth`, the real check

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-26 | Initial testnet-facing warning for the unverified signature path |
