# `tools/sweep-signer` Integration Notes

## Purpose

This is **usage notes for a tool that exists and works**, not a spec for a tool
that might be built. Everything below was read from
`tools/sweep-signer/src/main.rs` and `tools/sweep-signer/Cargo.toml` and
describes the tool's current actual behaviour. That behaviour can change: this
document describes version `0.1.0` of the crate, it does not specify it.

**Why this document exists.** `.env.example` documents exactly one invocation
of this tool, in a commented example line, and it is the `pubkey` subcommand:

```text
# run the ./target/release/sweep-signer pubkey --signer-seed-hex <seed>
```

That is the "before deploying" step. It is not the step an integrator needs
when they already have a deployed `SweepController` and want to produce an
`auth_signature` for `execute_sweep`. That is the `sign` subcommand, and until
now nothing in `testnet/` documented its flags, its environment variables, or
its output. This document closes that gap.

For the narrative walkthrough version of the same procedure, see
[`../examples/signed-sweep-walkthrough.md`](../examples/signed-sweep-walkthrough.md).
This document is the reference: complete flag list, exact output strings, and
the failure modes.

## Scope

**In scope:** how to build the tool, how to invoke both subcommands, what the
output looks like, and how to get from a deployed controller to a submitted
sweep.

**Out of scope:**

- Modifying the tool. Nothing here changes `tools/sweep-signer/`. It is
  referenced and documented, never edited.
- Anything in `scripts/`. This batch does not touch it.
- Building your own signer in another language. If you are doing that, read
  [`../../docs/SIGNATURE_FORMAT.md`](../../docs/SIGNATURE_FORMAT.md) first —
  and note the XDR-encoding warning below before you write an encoder.
- Key custody. Where a signing key is generated and stored is governed by
  [`../security/admin-key-hygiene.md`](../security/admin-key-hygiene.md).

---

## The one thing to get right before anything else

`tools/sweep-signer/Cargo.toml` pins:

```toml
soroban-sdk = "22.0.0"
```

with the comment that the pin exists **so `Address::to_xdr()` serialises
identically to what the deployed contract expects**, and that it should be
updated if bridgelet-core's `soroban-sdk` version changes. All four contracts
build against `soroban-sdk = "22.0.0"`.

**This is a correctness requirement, not dependency hygiene.** The signed
message embeds `destination.to_xdr()` and `contract_id.to_xdr()`. If the
signer's SDK encodes an `Address` differently from the contract's SDK — a
different discriminant, a different length prefix — the digest is different,
the digest is different from the one the contract computes, and the signature
does not verify. The failure surfaces as a bare `ed25519_verify` host-function
trap with **no error code and no diagnostic**. See
[`../runbooks/failed-sweep-signature.md`](../runbooks/failed-sweep-signature.md).

Two practical consequences:

1. If you write a signer in another language, do not hand-roll the XDR.
   [`../../docs/SIGNATURE_FORMAT.md`](../../docs/SIGNATURE_FORMAT.md) says so
   at length, names the specific mistakes, and recommends shelling out to this
   tool instead. Its own source uses `soroban_sdk::Address::to_xdr()` through a
   throwaway local `Env::default()` — no network involved — precisely so the
   bytes come from the SDK rather than from a hand-written encoder.
2. If bridgelet-core's contracts ever move off `soroban-sdk = "22.0.0"`, this
   pin must be revisited before the tool is trusted again.

---

## Building and invoking it

`tools/sweep-signer/Cargo.toml` contains its own `[workspace]` table. That is
deliberate: every member of the parent `bridgelet-core` workspace is built with
`--target wasm32-unknown-unknown`, and this tool is a **native CLI binary**
that should not be part of that build.

The practical effect is that the usual `cargo run -p sweep-signer` from the
repository root does not work, and neither does a plain `cargo build` at the
root picking it up. **Point cargo at the manifest instead.** This is the most
likely first stumbling block:

```bash
# From the repository root:
cargo run --manifest-path tools/sweep-signer/Cargo.toml -- --help
```

Or build a binary you can invoke directly:

```bash
cargo build --release --manifest-path tools/sweep-signer/Cargo.toml
./tools/sweep-signer/target/release/sweep-signer --help
```

Note the path difference: because the crate is its own workspace, cargo puts
build output under `tools/sweep-signer/target/`, not the repository-root
`target/`. The commented line in `.env.example` refers to
`./target/release/sweep-signer`, which is a **documentation artefact** — see
"Discrepancies worth knowing about" below.

### Subcommands

The tool is defined with two clap subcommands:

| Subcommand | When to run it | What it needs |
|---|---|---|
| `pubkey` | **Before deploying.** Derives the Ed25519 public key to register as `AUTHORIZED_SIGNER_PUBLIC_KEY` and to pass to `SweepController::initialize()`. | Only the signing key |
| `sign` | **After deploying**, once there is a real `contract_id` and a current nonce. | The signing key, the controller ID, a destination, and the nonce |

Both subcommands take the same signer-key group.

### `pubkey`

```bash
export SWEEP_SIGNING_KEY_SEED="<your 32-byte Ed25519 seed, hex>"

cargo run --quiet --manifest-path tools/sweep-signer/Cargo.toml -- pubkey
```

Output is two lines: the literal header, then the hex public key.

```text
Public key (hex) - put this in AUTHORIZED_SIGNER_PUBLIC_KEY:
<64 hex characters>
```

That second line is the value you set in the deployment environment and that
`scripts/deploy-testnet.sh` passes to `initialize()` as `--authorized_signer`.
`pubkey` performs no network I/O whatsoever.

### `sign` — flags

| Flag | Type | Required | Meaning |
|---|---|---|---|
| `--contract-id <C...>` | `String`, parsed as a Soroban `Address` | yes | The `SweepController` address. It is part of the signed message, so it must be the exact deployed instance. |
| `--destination <G...>` | `String`, parsed as a Soroban `Address` | yes | Where funds will be swept. Also part of the signed message. |
| `--nonce <u64>` | `u64` | yes | The controller's current nonce. Read it immediately before signing. |
| `--signer-seed-hex <64 hex chars>` | `String` | exactly one of these two | Raw 32-byte Ed25519 seed, hex-encoded. Also read from `SWEEP_SIGNING_KEY_SEED`. |
| `--signer-secret <S...>` | `String` | exactly one of these two | Stellar secret key in `S...` strkey form, decoded via `stellar-strkey`. Also read from `AUTHORIZED_SIGNER_SECRET`. |

The two signer options are a clap group declared
`#[group(required = true, multiple = false)]`, which has two consequences you
will feel immediately:

- Supplying **neither** is a usage error. clap rejects it, with
  `error: the following required arguments were not provided:` followed by
  `<--signer-seed-hex <SIGNER_SEED_HEX>|--signer-secret <SIGNER_SECRET>>`.
  (The tool's own `Provide either --signer-seed-hex or --signer-secret` message
  is a fallback inside `to_signing_key()` that the required group makes
  unreachable in practice.)
- Supplying **both** is also an error, even if they are the same key:
  `error: the argument '--signer-seed-hex <SIGNER_SEED_HEX>' cannot be used
  with '--signer-secret <SIGNER_SECRET>'`.

The tool is explicit about this in its own help text: `--signer-seed-hex` is
described as the recommended form, because this key is signing-only and never
needs to be a funded Stellar account, so there is no reason to wrap it in a
Stellar strkey.

### Prefer the environment-variable form

Both signer flags declare an `env =` binding, so the key never has to appear on
the command line:

```dotenv
SWEEP_SIGNING_KEY_SEED=<your 32-byte Ed25519 seed, hex>
```

**Prefer this form.** A `--signer-seed-hex` argument is visible in your shell
history, in `ps` output for the lifetime of the process, and in any shell
transcript or CI log that echoes the command. The environment-variable form
narrows all three. The same applies to `AUTHORIZED_SIGNER_SECRET` for
`--signer-secret`.

### Seed format requirements

- `--signer-seed-hex` must hex-decode to **exactly 32 bytes**. Anything else is
  rejected, and the tool exits with a message of the form
  `--signer-seed-hex must decode to exactly 32 bytes, got N`.
- Invalid hex produces `Invalid --signer-seed-hex: <parse error>`.
- An undecodable `S...` strkey produces `Invalid --signer-secret: <error>`.
- If neither option resolves — including the case where the environment
  variable is unset — the tool prints
  `Provide either --signer-seed-hex or --signer-secret` and exits `1`. In
  practice the required clap group rejects this first, so you will usually see
  clap's usage error instead.
- All of these seed/strkey exits are `exit(1)`, and the messages go to
  **stderr**. Output on **stdout** is only the result lines documented above,
  which is what makes the `grep`/`awk` piping safe.

### `sign` — exact output

`sign` prints, in this order, on stdout:

```text
auth_signature (hex, pass to execute_sweep): <128 hex characters>
signer public key (hex, sanity-check against AUTHORIZED_SIGNER_PUBLIC_KEY): <64 hex characters>

⚠️  Nonce used: <N>. Confirm this matches SweepController::get_nonce() on the
   live contract at the moment you sign - a stale nonce produces a
   signature the contract will reject.
```

Line 1 is the value to pass as `auth_signature`. Line 2 is the public key
derived from the key you just used — read it, do not pipe past it, because it
is one of the two values most likely to be wrong. Line 3 is a blank line. The
remaining three lines are the tool's own warning, which repeats the nonce it
used so you can compare it against the chain.

The signature is 64 bytes, so 128 hex characters. Do not expect error codes or
a JSON payload; this tool prints human-readable text and nothing else.

### Piping the signature out

The pattern used elsewhere in this tree:

```bash
SIG=$(cargo run --quiet --manifest-path tools/sweep-signer/Cargo.toml -- \
  sign \
  --contract-id "$SWEEP_CONTROLLER_ID" \
  --destination "$DESTINATION" \
  --nonce "$NONCE" \
  | grep '^auth_signature' | awk '{print $NF}')
```

If you must pass a secret-bearing argument inline, prefer
`--signer-seed-hex` reading from `SWEEP_SIGNING_KEY_SEED` over
`--signer-secret "$AUTHORIZED_SIGNER_SECRET"`; the flag form puts the key in
the process table.

### Sanity-checking the signer public key

The public key the tool prints must be identical to the value the controller
was initialised with. Read it and compare:

```bash
cargo run --quiet --manifest-path tools/sweep-signer/Cargo.toml -- \
  sign --contract-id "$SWEEP_CONTROLLER_ID" --destination "$DESTINATION" \
  --nonce "$NONCE" | grep '^signer public key' | awk '{print $NF}'
```

That printed hex is `AUTHORIZED_SIGNER_PUBLIC_KEY` from the deployment
environment, and it is the same value recorded as `config.authorizedSigner` in
`deployments/testnet.json`. The two are the same value by construction; if they
disagree, you are signing with a key the controller does not know about, and
**every signature will fail**.

> **`SweepController` has no on-chain getter for `authorized_signer`.** It is
> set once by `initialize()` and cannot be read back. `deployments/testnet.json`
> is therefore the only local record of it, and it can be stale relative to the
> deployment. This is exactly the same problem
> [`nonce-inspector-spec.md`](nonce-inspector-spec.md) calls out for its own
> `--check` mode: a correct answer derived from a stale input. If the public
> key comparison fails and the nonce and destination are both confirmed,
> suspect the config file before you suspect the signer.

---

## End-to-end procedure

1. **Derive or confirm the signer public key.** Before the controller is
   deployed, run `pubkey` and record the hex output as
   `AUTHORIZED_SIGNER_PUBLIC_KEY`. The deployment script passes that value to
   `SweepController::initialize()` as `--authorized_signer`. If the controller
   is already deployed, skip to step 4 and use the `sign` output's second line
   as the check.

2. **Deploy the controller.** `scripts/deploy-testnet.sh` requires
   `AUTHORIZED_SIGNER_PUBLIC_KEY`, `RECOVERY_ADDRESS`, `CREATOR_ADDRESS` and
   `SIGNER_SECRET_KEY`. Note that as of the last review the registry records
   the contracts as **not deployed** — see
   [`../registry/contract-status.md`](../registry/contract-status.md) and
   [`../security/README.md`](../security/README.md).

3. **Read the current nonce.** This is the step that gets skipped and then
   debugged. `SweepController::get_nonce()` is a read-only call:

   ```bash
   NONCE=$(stellar contract invoke \
     --id "$SWEEP_CONTROLLER_ID" \
     --network testnet \
     --source "$READER" \
     -- \
     get_nonce)
   ```

   The nonce is **global to the controller, not per account**. There is one
   counter per `SweepController` deployment, shared by every account that
   deployment services. It starts at `0` at `initialize()` and increments by 1
   after every successful `execute_sweep()` or `claim()`. Fetch it
   **immediately before** signing, and **serialise submissions** so two workers
   cannot read the same value and race. See
   [`../config/nonce-tracking.md`](../config/nonce-tracking.md), and
   [`nonce-inspector-spec.md`](nonce-inspector-spec.md) for the proposed
   read-only tool that automates exactly this read.

4. **Sign with that nonce.** Using the environment-variable form, immediately
   after step 3:

   ```bash
   export SWEEP_SIGNING_KEY_SEED="<your 32-byte Ed25519 seed, hex>"

   SIG=$(cargo run --quiet --manifest-path tools/sweep-signer/Cargo.toml -- \
     sign \
     --contract-id "$SWEEP_CONTROLLER_ID" \
     --destination "$DESTINATION" \
     --nonce "$NONCE" \
     | grep '^auth_signature' | awk '{print $NF}')
   ```

   The signed message is, exactly:

   ```text
   SHA256( destination.to_xdr() || nonce as u64 big-endian (8 bytes) || contract_id.to_xdr() )
   ```

   There is no timestamp, no fourth component, and no inclusion of the
   ephemeral account address. A signature is therefore **not bound to a
   specific account** — it is bound to `(destination, nonce, controller)`. That
   is what makes resubmission and nonce races behave the way they do, and it is
   the subject of [`../runbooks/nonce-desync.md`](../runbooks/nonce-desync.md).

5. **Submit via `SweepController::execute_sweep`.** Promptly — the signature
   is only valid while the nonce is current:

   ```bash
   stellar contract invoke \
     --id "$SWEEP_CONTROLLER_ID" \
     --network testnet \
     --source "$RELAYER" \
     -- \
     execute_sweep \
     --ephemeral_account "$EPHEMERAL_ACCOUNT_ID" \
     --destination "$DESTINATION" \
     --auth_signature "$SIG"
   ```

   The **relayer** submitting this transaction is the party that needs XLM. The
   signing key does not, and must not be, a funded account.

### Idempotency note

`SweepController::claim(recipient, ephemeral_account)` is the gas-free relayer
path: the recipient signs a Soroban auth entry, the relayer submits and pays
fees, and the controller uses `authorize_as_current_contract()`. **It takes no
signature.** Any retry or idempotency advice that assumes a signature is
involved does not apply to `claim`. Because the nonce is global to the
controller, retry discipline for `execute_sweep` is a property of the
*controller*, not of the individual account — two accounts sweeping through the
same controller contend with each other.

---

## Troubleshooting

| Symptom | Likely cause | Check |
|---|---|---|
| `Provide either --signer-seed-hex or --signer-secret` | Neither flag nor the matching env var was set | `SWEEP_SIGNING_KEY_SEED` or `AUTHORIZED_SIGNER_SECRET` exported in *this* shell. The required clap group normally rejects this first |
| `--signer-seed-hex must decode to exactly 32 bytes, got N` | Wrong length seed, or a value that is not hex | Recount; a 32-byte seed is 64 hex characters |
| `Invalid --signer-secret: <error>` | Malformed `S...` strkey, or a `G...` public address passed by mistake | Re-generate; the flag takes a **secret** |
| `error: the argument '--signer-seed-hex <SIGNER_SEED_HEX>' cannot be used with '--signer-secret <SIGNER_SECRET>'` | Both were supplied | The group is `required = true, multiple = false`. Supply one, or let both come from the environment |
| `error: the following required arguments were not provided` | Neither was supplied, and neither env var was set | Export the seed rather than relying on the flag |
| Cargo cannot find the package, or the root build tries to build this for `wasm32-unknown-unknown` | Invoked without `--manifest-path` | Use `cargo run --manifest-path tools/sweep-signer/Cargo.toml -- …`; it declares its own `[workspace]` |
| `./target/release/sweep-signer: no such file` | Expected the repository-root target dir | The crate is its own workspace; the binary is under `tools/sweep-signer/target/release/` |
| **`ed25519_verify` host-function trap, no error code** | **Usually a stale or contended nonce, not a signing bug** | Compare the nonce you signed with against `get_nonce()` now. If it advanced, re-sign. See below |
| `UnauthorizedDestination` (13) | The controller was initialised with a locked `authorized_destination` and your destination differs | This is checked **before** signature verification, so it is a clean error, not a trap |
| `SignatureVerificationFailed` in a log, but the transaction actually trapped | Error 9 is defined in `contracts/sweep_controller/src/errors.rs` but the current `verify_sweep_auth` path calls `env.crypto().ed25519_verify` and returns `Ok(())`; a bad signature traps rather than returning a contract error | Treat the trap as the symptom. Do not wait for a code you will not receive |
| Signature valid, sweep still fails | Destination, nonce, and contract ID were all correct but the **ephemeral account** is not sweepable, or lacks balance | Cross-check `can_sweep` and the SEP-41 balance. `record_payment` is bookkeeping only |
| Signer public key printed by `sign` ≠ `config.authorizedSigner` | Wrong seed loaded, or a stale `deployments/testnet.json` | Both are the same value by construction. There is no on-chain getter, so the config file is the only record — see the staleness note above |

**The `ed25519_verify` trap is a nonce problem far more often than a signing
problem.** `SweepController` always verifies against its own current on-chain
nonce, and the deployment is shared: another party's successful sweep advances
the same counter yours depends on. A digest you can independently confirm is
correct, followed by a bare trap, points at contention, not at your Ed25519
implementation. Triage it as such:
[`../runbooks/failed-sweep-signature.md`](../runbooks/failed-sweep-signature.md)
for the full diagnostic sequence, and
[`../security/known-testnet-abuse-patterns.md`](../security/known-testnet-abuse-patterns.md)
for the shared-deployment abuse patterns that produce exactly this symptom.
`SignatureVerificationFailed` (9) and `InvalidNonce` (11) are defined in the
error enum, but note that the current verification path traps instead of
returning them, so treat the enum as a code reference, not as a description of
what you will observe.

---

## Key handling

The signing key this tool needs is **signing-only**. It should never be a
funded Stellar account, never be a key reused from a personal wallet or a
sandbox, and never be the deployment/admin key. `tools/sweep-signer` exists
partly to keep those two roles separate; conflating them means a leak of the
cheap key compromises sweep authority and a leak of the deployer key exposes a
funded account. See [`../security/admin-key-hygiene.md`](../security/admin-key-hygiene.md).

The no-secrets discipline in [`../config/README.md`](../config/README.md) applies
in full, and it applies with extra force to a document that documents a tool
which accepts a key:

- **Never commit a seed or a secret key** — not to this repository, not to a
  fork, not to a gist.
- **Never paste one into an issue, a PR, or a chat.** If a key is exposed,
  treat it as compromised and rotate the controller's signer by redeploying;
  there is no rotation function and no on-chain getter.
- **Never record one in any `testnet/` document**, including this one. This
  document contains no key material of any kind, real or illustrative.
- **Prefer the environment-variable form** for the reason given above.

### Discrepancies worth knowing about

`.env.example` carries a commented example invocation that includes a concrete
seed-like hex value. **That value is a documentation artefact, not a usable
key, and it must not be used to sign anything.** Do not copy it into a command,
and do not treat it as an example of "safe to paste" key material. Generate
your own:

```bash
node -e "console.log(require('crypto').randomBytes(32).toString('hex'))"
```

The same file's commented path (`./target/release/sweep-signer`) also does not
match where the binary actually lands, for the workspace reason given above.
Both are documentation, not instructions.

---

## Open Questions

- Should the tool emit JSON, or a `--quiet` mode that prints only the signature?
  Today it prints prose and callers parse it with `grep`/`awk`, which is
  brittle. Changing it is a change to the tool, which is out of scope for this
  document.
- Should `sign` accept a contract *name* rather than only a `C...` address?
  [`contract-id-lookup-spec.md`](contract-id-lookup-spec.md) proposes the
  resolution half of this.
- Should there be a way to check an existing signature locally, as
  [`nonce-inspector-spec.md`](nonce-inspector-spec.md) proposes? Doing it
  correctly requires the same XDR coupling described above, which is the main
  argument for not reimplementing it.
- Does the crate need a version bump policy? It is currently `0.1.0` with no
  published distribution channel, so "which version am I documenting" is
  currently answerable only from the source tree.

---

## Related Documentation

- [`decision-log.md`](decision-log.md) — why this directory holds specs, and where the one real tool actually lives
- [`../examples/signed-sweep-walkthrough.md`](../examples/signed-sweep-walkthrough.md) — the narrative end-to-end walkthrough
- [`../config/nonce-tracking.md`](../config/nonce-tracking.md) — nonce semantics, and the read procedure this depends on
- [`nonce-inspector-spec.md`](nonce-inspector-spec.md) — spec for the read-only nonce tool this document assumes
- [`contract-id-lookup-spec.md`](contract-id-lookup-spec.md) — spec for resolving a contract name to an ID
- [`../runbooks/failed-sweep-signature.md`](../runbooks/failed-sweep-signature.md) — diagnosing a rejected signature
- [`../runbooks/nonce-desync.md`](../runbooks/nonce-desync.md) — recovering from a drifted or contended nonce
- [`../security/known-testnet-abuse-patterns.md`](../security/known-testnet-abuse-patterns.md) — shared-deployment interference
- [`../security/admin-key-hygiene.md`](../security/admin-key-hygiene.md) — handling the deployer and signing keys
- [`../config/README.md`](../config/README.md) — the no-secrets standard
- [`../../docs/SIGNATURE_FORMAT.md`](../../docs/SIGNATURE_FORMAT.md) — message format, and the XDR-encoding warning
- `tools/sweep-signer/` — the tool itself (referenced, not modified)

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-26 | Initial usage notes for `tools/sweep-signer`, read from the source of crate version 0.1.0 |
