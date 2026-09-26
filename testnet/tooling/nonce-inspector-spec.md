# Spec: Nonce Inspector — Read the Current `SweepController` Nonce

## Status

**Spec only. Nothing in this document is implemented.** See `testnet/tooling/decision-log.md` for why this directory holds specifications rather than runnable scripts.

## Problem

`SweepController` verifies every sweep signature against `SHA256(destination ‖ nonce ‖ controller_id)` using its **own current on-chain nonce**. Off-chain signers that track the nonce locally will eventually disagree with the contract, and the symptom is a bare host-function trap from `ed25519_verify` — no error code, no diagnostics, nothing to match on.

That failure mode is documented in `testnet/runbooks/failed-sweep-signature.md`, and it is the single most common cause of a rejected sweep signature. The workaround today is a shell one-liner that has been copy-pasted into at least three places in this repo.

A read-only helper that prints the current nonce — and, ideally, enough context to tell whether a signature you already hold is still valid — removes the highest-frequency source of confusing testnet failures.

> This tool **directly supports** the workflow in `testnet/config/nonce-tracking.md`. That document is the reference; this is a proposal for tooling around it.

## Scope

**In scope:** read-only. Query the nonce, show it, and report whether a given signature is still expected to verify.

**Explicitly out of scope:**

- Signing. That is `tools/sweep-signer`, which already exists and works.
- Submitting transactions of any kind.
- Mutating anything, ever.
- Predicting or setting the nonce.

A read-only tool cannot grief the shared deployment, cannot consume a nonce, and cannot interfere with anyone's sweep. That property is the entire design constraint.

---

## Interface

### Invocation

```bash
bridgelet-nonce --controller <SWEEP_CONTROLLER_ID> [options]
```

### Options

| Option | Default | Purpose |
|---|---|---|
| `--controller <C...>` | required | `SweepController` contract ID. Resolve names via the contract-ID lookup tool spec. |
| `--json` | off | Emit machine-readable JSON for CI. |
| `--check <HEX_SIG>` | off | Verify a signature you already hold against the current nonce. |
| `--destination <G...>` | none | Destination used with `--check`. |
| `--rounds <N>` | `3` | How many times to sample, to detect churn. |
| `--rpc-url <URL>` | `https://soroban-testnet.stellar.org` | RPC endpoint. |
| `--timeout-ms <N>` | `10000` | Per-request timeout. |

### Output — human

```
controller: CBEU4X5MNGOECBSTNEUFMBALH2YI5YV4UIH7YXRNOLR2DNLZQD4Z5KWE
network:    testnet
nonce:      7
stable:     yes   (3 samples, all 7)
next:       sign with --nonce 7, then submit promptly
```

### Output — JSON

```json
{
  "controller": "CBEU4X5MNGOECBSTNEUFMBALH2YI5YV4UIH7YXRNOLR2DNLZQD4Z5KWE",
  "network": "testnet",
  "nonce": 7,
  "stable": true,
  "samples": [7, 7, 7],
  "checked_signature": null
}
```

With `--check`:

```json
{
  "controller": "CBEU...",
  "nonce": 7,
  "stable": true,
  "checked_signature": {
    "destination": "GBEST...",
    "nonce_used": 6,
    "expected": false,
    "reason": "stale-nonce",
    "message": "signature was built over nonce 6, controller is at 7; re-sign with --nonce 7"
  }
}
```

### Exit codes

| Code | Meaning |
|---|---|
| `0` | Success. `expected: true` if `--check` was given. |
| `1` | Signature check failed / signature is stale. |
| `2` | Usage error (bad ID, missing required option). |
| `3` | RPC unreachable or timed out. |
| `4` | Controller not initialised, or not a contract. |

Distinct exit codes matter: CI needs to tell "your signature is stale, retry" (1) from "the network is down" (3). Collapsing them teaches people to ignore the failure.

---

## Behaviour

### Core operation

One `simulateTransaction` invoking `get_nonce()`, or a plain read-only `invokeHostFunction`. The SDK exposes the nonce as `u64`; it is not a `u128`, so **JSON output must carry it as a number or a decimal string, never as a float.**

### `stable` — the part that makes this worth building

A single sample is not enough to be useful. Between your read and your submission the nonce can move.

Sample `--rounds` times with a short interval and report whether they agree:

- All equal → `stable: true`. The value is probably still current.
- Any differ → `stable: false`, and print **all** samples plus the delta. The nonce is moving under you; serialise your signing and retry.

This turns an invisible race into an explicit signal, and it is cheap — a handful of read-only RPC calls.

### `--check` — verify a signature you already hold

Reconstruct the digest the contract would compute, and compare the signature against it:

```
digest = SHA256( destination.to_xdr() || nonce_be_u64(current) || controller_id.to_xdr() )
```

Then report whether `auth_signature` verifies against `digest` under the controller's `authorized_signer`.

Two implementation notes:

1. **You need the controller's `authorized_signer` to verify.** There is **no getter** for it on `SweepController`. Read it from `deployments/testnet.json` (`config.authorizedSigner`) and say so in the output — if the config file is stale relative to the deployment, the answer will be wrong and the user needs to know which input it came from.
2. **Do not hand-roll `Address::to_xdr()`.** Use `soroban-sdk`'s own serialisation, exactly as `tools/sweep-signer` does. A hand-rolled XDR encoder is the failure mode `docs/SIGNATURE_FORMAT.md` warns about at length: subtly wrong bytes produce a signature that does not verify, with no indication of why.

A useful extra: also report whether the signature verifies against the **previous** nonce, to distinguish "definitely stale" from "never was valid".

---

## Non-Goals

| Not doing | Why |
|---|---|
| Tracking the nonce across invocations | The whole point is that a local counter drifts. Storing one invites the bug. |
| Caching | A cached nonce is the failure being fixed. |
| Any write path | Keeps the tool incapable of griefing the shared deployment. |
| Submitting sweeps | Belongs in a relayer, not in a diagnostic. |
| Validating ephemeral account state | `simulate_sweep()` and `can_sweep()` already cover that; don't duplicate. |

---

## Integration

### Before signing

```bash
NONCE=$(bridgelet-nonce --controller "$SWEEP_CONTROLLER_ID" --json | jq -r '.nonce')
# ... sign with $NONCE ...
```

### In CI, gate on stability

```bash
bridgelet-nonce --controller "$SWEEP_CONTROLLER_ID" --rounds 5 --json
# non-zero exit = nonce is moving or RPC is down; do not sign
```

### With `tools/sweep-signer`

```bash
SIG=$(cargo run --manifest-path tools/sweep-signer/Cargo.toml -- sign \
  --contract-id "$SWEEP_CONTROLLER_ID" --destination "$DEST" \
  --nonce "$(bridgelet-nonce --controller "$SWEEP_CONTROLLER_ID" --json | jq -r .nonce)" \
  --signer-seed-hex "$SEED" | grep ^auth_signature | awk '{print $NF}')

# Confirm it is still expected to verify BEFORE submitting
bridgelet-nonce --controller "$SWEEP_CONTROLLER_ID" \
  --check "$SIG" --destination "$DEST" || echo "re-sign and retry"
```

That last step is the real win: a cheap local check that catches a stale signature **before** paying a fee to have it rejected on-chain.

### As a test assertion

```typescript
// A sweep helper should not submit against a nonce it assumed was current.
const { nonce, stable } = await nonceInspector(controllerId);
if (!stable) throw new Error('nonce in flux; serialise and retry');
```

---

## Open Questions

- Should `--check` be able to read `authorized_signer` from somewhere more authoritative than `deployments/testnet.json`? There is no on-chain getter. A proposal to add one would be a contract change, outside this directory's scope.
- Poll interval between `--rounds` samples. Too short and consecutive reads land in the same ledger (wasted calls, false confidence); too long and the check becomes slow. Probably one or two ledgers.
- Should the tool warn when it detects a locked `authorized_destination`? Also not readable on-chain, but inferable from `UnauthorizedDestination` behaviour.

---

## Related Documentation

- `testnet/config/nonce-tracking.md` — the workflow this supports; nonce semantics and pitfalls
- `testnet/runbooks/failed-sweep-signature.md` — what to do when a signature is rejected
- `docs/SIGNATURE_FORMAT.md` — message format, and the XDR-encoding warning
- `tools/sweep-signer/` — the existing signing tool (referenced, not modified here)
- `testnet/registry/verify-contract-live.md` — health checks, of which this is one
- `testnet/tooling/decision-log.md` — why this is a spec and not a script

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-26 | Initial nonce inspector spec |
