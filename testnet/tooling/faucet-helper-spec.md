# Spec: Faucet Helper — Fund and Initialise a Fresh Test `EphemeralAccount` in One Step

## Status

**Spec only. Nothing in this document is implemented.** See `testnet/tooling/decision-log.md` for why this directory holds specifications rather than runnable scripts.

## Problem

Every manual testnet experiment starts with the same three-step dance, and every newcomer gets it slightly wrong:

1. Create a Stellar account and **fund it with Friendbot** (XLM is required before the account exists on-ledger, and before it can pay any fee).
2. `stellar contract deploy` a new `ephemeral_account` WASM instance.
3. `stellar contract invoke ... initialize` with six arguments — two of which (`expiry_ledger` and `recovery_address`) are easy to get wrong and **cannot be corrected afterwards**.

Then, to do anything useful, a fourth step: fund the new contract with a SEP-41 token, because `record_payment()` only records bookkeeping — if the account has no balance, the eventual sweep fails with `TransferFailed`.

That is four commands, roughly ten seconds of RPC latency, and two irreversible parameters. A tester running this fifty times is fifty opportunities to type a bad `recovery_address`.

A single helper that does steps 1–3 — optionally 4 — removes the class of error entirely.

> Funding behaviour, Friendbot limits, and the workarounds for throttling are documented in `testnet/config/friendbot.md`. That document is the reference; this is a proposal for a tool layered on top of it.

## Scope

**In scope:** create → fund → deploy → initialise, with correct-by-construction parameter handling, and print a summary an integrator can copy.

**Out of scope:**

- Recording payments. The tester chooses the asset and amount; guessing would be worse than useless.
- Sweeping. Different tool, different risk profile.
- Managing Friendbot rate limits beyond surfacing them clearly.
- Anything that touches contracts not owned by this deployment.

---

## Interface

### Invocation

```bash
bridgelet-faucet [options]
```

### Options

| Option | Default | Purpose |
|---|---|---|
| `--expiry <ledgers \| 5m \| 1h \| 1d \| 1w>` | `1h` | Expiry, by ledger count or human duration. See below. |
| `--recovery <G...>` | caller's address | Recovery address. **Immutable after init** — confirm before deploying. |
| `--admin <G...>` | caller's address | Upgrade admin. Immutable after init. |
| `--controller <C...>` | shared `SweepController` ID | `authorized_controller`. Must be a real controller. |
| `--creator <G...>` | caller's address | `creator`; also authorises `initialize()`. |
| `--identity <name>` | caller's active identity | Stellar identity to operate as. |
| `--token <C...>:<amount>` | none | Optional SEP-41 funding, repeatable. |
| `--wasm <path>` | built `ephemeral_account.wasm` | WASM to deploy. |
| `--dry-run` | off | Print the full plan and exit. Deploys nothing. |
| `--json` | off | Machine-readable output. |
| `--rpc-url <URL>` | `https://soroban-testnet.stellar.org` | RPC endpoint. |

### Output

```
✔ funded          testnet-harness  GABC…  (10000 XLM)
✔ deployed        CDEF…
  recovery        GABC…          (your address)
  controller      CBEU…          (shared SweepController)
  admin           GABC…
  expiry_ledger   17894512       (+720 ≈ 1h from 17893792)
✔ initialized     expiry_ledger 17894512

  ephemeral account   CDEF…
  recovery address    GABC…
  controller          CBEU…
  admin               GABC…
  expires at ledger   17894512  (~1h)

Copy this block into your test:
  EPHEMERAL_ID=CDEF…
  RECOVERY_ADDRESS=GABC…
  SWEEP_CONTROLLER_ID=CBEU…
```

The copyable block matters. The point is not that the helper runs the commands; it is that it **emits the resulting identifiers in a form that can be pasted into a test without retyping them.**

JSON:

```json
{
  "identity": "testnet-harness",
  "ephemeral_id": "CDEF...",
  "recovery_address": "GABC...",
  "authorized_controller": "CBEU...",
  "admin": "GABC...",
  "expiry_ledger": 17894512,
  "funding": [
    { "asset": "CA3D5...", "amount": "2500000000" }
  ]
}
```

Note `amount` as a **string**. Token amounts are `i128`; JSON numbers lose precision above 2^53.

---

## Behaviour

### Stage 1 — Fund

```bash
stellar keys create --global "$IDENTITY"        # if it does not exist
stellar keys fund "$IDENTITY" --network testnet # Friendbot
```

A Stellar account must exist and be funded on-ledger before it can appear as a contract argument or pay a fee. If the identity already exists and is funded, skip — but **check** rather than assume, because an unfunded account fails later at `initialize()` with a confusing auth error.

Report Friendbot throttling distinctly from other failures. It means "slow down and retry", not "something is wrong". The limits and the workarounds are in `testnet/config/friendbot.md`.

### Stage 2 — Deploy

```bash
stellar contract deploy --wasm "$WASM" --network testnet --source "$IDENTITY"
```

If the WASM is missing, say so and print the build command. Do **not** silently build or substitute a different artefact — deploying the wrong WASM produces an account that behaves unexpectedly much later.

### Stage 3 — Initialise

Six arguments, all required, all in one call:

```bash
stellar contract invoke --id "$EPH_ID" --network testnet --source "$IDENTITY" -- initialize \
  --creator "$CREATOR" \
  --expiry_ledger "$EXPIRY" \
  --recovery_address "$RECOVERY" \
  --authorized_controller "$CONTROLLER" \
  --admin "$ADMIN"
```

`initialize()` sets every one of these exactly once. A second call returns `AlreadyInitialized` (1) and there is **no setter for any of them**. The two most consequential:

- **`recovery_address`** — the destination of the whole recovery path. Wrong means funds are unrecoverable by you.
- **`expiry_ledger`** — must be strictly in the future (`InvalidExpiry`, code 5) or the account is dead on arrival; must be far enough out that ledger timing does not surprise you.

The helper's real value is refusing to let either be wrong silently. If `--recovery` was not passed, default to the caller's own address and **echo it prominently** — a recovery address you did not consciously choose is the kind of thing nobody notices until it matters.

### `--expiry` parsing

| Input | Ledgers added | Approximate wall time |
|---|---|---|
| `5m` | 60 | ~5 min |
| `1h` | 720 | ~1 hr |
| `1d` | 17280 | ~24 hr |
| `1w` | 120960 | ~7 days |
| `3600` (raw) | 3600 | ~5 hr |

Read the live ledger first — `stellar network current-ledger` — and add. Never derive the expiry from a local clock; `expiry_ledger` is a **ledger sequence number**, and `is_expired()` uses `current >= expiry`, inclusive, so the boundary ledger is already expired. The arithmetic and its rationale are in `testnet/config/expiry-ledger-testing.md`.

### Stage 4 — Optional token funding

```bash
--token CA3D5…:2500000000
```

`record_payment()` is **bookkeeping only** — it does not move value. Without an actual balance, the sweep later reaches `transfers::execute_transfers()` and reverts with `TransferFailed`, taking the whole multi-asset sweep down with it. Offering `--token` here prevents the most common "it initialised fine but the sweep fails" report.

Do not call `record_payment()` automatically. The tester picks the asset and amount.

### `--dry-run`

Print every command, every resolved address, and the computed `expiry_ledger`, then exit without touching the network. This is how you check a `--recovery` value before committing to it.

---

## Failure Handling

| Stage | Failure | Handling |
|---|---|---|
| Fund | Friendbot throttled | Report as throttle with a retry hint; do not treat as fatal |
| Fund | Account exists, unfunded | Re-fund, continue |
| Deploy | WASM not found | Stop; print the build command |
| Deploy | Insufficient XLM | Report balance and required amount |
| Init | `InvalidExpiry` (5) | Computed expiry ≤ current ledger; recompute and retry once |
| Init | `AlreadyInitialized` (1) | Should be impossible on a fresh deploy; report loudly, as it means a hash/deploy collision |
| Init | auth failure | Caller did not authorise. The identity must be the `creator` |
| Init | `NotInitialized` (2) | Deployed but not initialised — check the WASM is really `ephemeral_account` |
| Token fund | Insufficient balance | Name the asset; do not retry blindly |
| Any | RPC unreachable | Distinct exit code from contract errors |

### Exit codes

`0` success · `1` contract returned an error · `2` usage error · `3` RPC unreachable · `4` Friendbot throttled · `5` prerequisite missing (WASM, identity)

---

## Ordering and Idempotency

The order is not arbitrary: fund → deploy → initialise → token-fund. Each stage depends on the previous one having landed on-chain, so the helper must **wait for confirmation between stages** rather than firing them optimistically.

A helper that does not wait produces accounts that initialise against an unconfirmed deploy, which fails in a way that looks like a contract bug.

**Not idempotent.** Every run creates a new Stellar account and a new contract instance. There is no "reuse" mode, deliberately: `EphemeralAccount` is single-use by design (one payment per asset, one sweep ever, immutable recovery address), so reusing one instance across experiments is a reliable source of confusing failures. Reuse the funded *Stellar* account, not the contract instance.

---

## Ethics on a Shared Deployment

The testnet deployment is shared. This helper creates state, so:

- **Never call `upgrade()`.** Admin-gated and destructive for every integrator.
- **Never call `update_authorized_destination()`.** Creator-gated and it changes routing for every account on that controller.
- **Do not sweep or expire the long-lived accounts** in `testnet/registry/known-test-accounts.md`.
- **Fund only your own accounts.**
- **Prefer short expiries** so your accounts retire rather than accumulating.
- Do not add a batch mode. `AccountFactory::batch_initialize` exists for programmatic fan-out and **swallows per-account error detail** (`success: false, error: None`) — see `testnet/runbooks/account-factory-batch-failure.md`. A one-account helper that is obviously correct is more useful than a batch helper that fails invisibly.

The broader set of shared-state hazards is in `testnet/security/known-testnet-abuse-patterns.md`.

---

## Open Questions

- Should it emit a Host.bot payment-request URL instead of calling Friendbot directly? Friendbot's canonical UX is the URL, and it avoids rate-limit interactions entirely.
- Should `--recovery` be mandatory with no default? The default is convenient; requiring it is safer. Lean toward mandatory, since the whole point is eliminating silent misconfiguration.
- Should the helper verify the WASM hash it is about to deploy against `testnet/registry/wasm-hash-reference.md`?
- Worth a `--json` schema stable enough to depend on from CI? If so, it needs a version field.

---

## Related Documentation

- `testnet/config/friendbot.md` — funding behaviour and rate limits
- `testnet/config/expiry-ledger-testing.md` — expiry arithmetic, and why not to use a clock
- `testnet/config/soroban-cli-identity-setup.md` — identity conventions used here
- `testnet/registry/known-test-accounts.md` — long-lived accounts not to disturb
- `testnet/runbooks/account-factory-batch-failure.md` — why batch creation is not wrapped
- `testnet/examples/create-and-fund-ephemeral-account.md` — the manual flow this replaces
- `docs/api-reference.md` — `initialize` parameter reference

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-26 | Initial faucet helper spec |
