<!--
Purpose: Operational workaround and verification steps for confirming whether
SweepController's destination lock is still in force after one or more sweep
paths have been exercised.
Owner: @JudeDaniel6 (closes #288 — bridgelet-audit/ knowledge-base initiative).
Status: Documentation-only. This runbook describes an operator-side workaround,
not a code fix.
-->

# Verifying Nonce State Before Trusting `update_authorized_destination`'s Lock

> **Purpose.** Document an **operational workaround** for the fact that
> `SweepController::get_nonce()` can be ambiguous about *which* sweep path was
> taken. Operators must confirm the destination lock is still in force by
> reading the nonce **and** cross-checking against `SweepCompleted` events
> before invoking `update_authorized_destination`.

This runbook is **not** a code fix. The contract logic is intentionally
shared across two sweeps paths (`execute_sweep` and `claim`); the gap
described here is a documentation gap for operators.

| Field | Value |
| :--- | :--- |
| **Related issue** | [#288](https://github.com/bridgelet-org/bridgelet-core/issues/288) |
| **Owner / reviewer** | `_operator-name_` |
| **Target controller contract** | `_C...controller-address_` |
| **Target network** | `testnet` / `mainnet` |
| **Last reviewed** | `_ISO-8601 date_` |

## Table of Contents

1. [Why This Runbook Exists](#why-this-runbook-exists)
2. [What "the lock is still in force" Means](#what-the-lock-is-still-in-force-means)
3. [Pre-Conditions](#pre-conditions)
4. [Step 1 — Read `get_nonce()` Directly](#step-1--get_nonce-directly)
5. [Step 2 — Enumerate `SweepCompleted` Events](#step-2--enumerate-sweepcompleted-events)
6. [Step 3 — Cross-Check and Decide](#step-3--cross-check-and-decide)
7. [Step 4 — If You Decide Not To Call `update_authorized_destination`](#step-4--if-you-decide-not-to-call-update_authorized_destination)
8. [Operational Caveats](#operational-caveats)
9. [Related Issues](#related-issues)

---

## Why This Runbook Exists

`SweepController` exposes two distinct sweep paths:

| Path | Function | Authorization | Nonce impact | Emits `SweepCompleted`? |
| :--- | :--- | :--- | :--- | :--- |
| `execute_sweep` | `execute_sweep(ephemeral_account, destination, auth_signature)` | Ed25519 signature from `authorized_signer`, verified against `hash(destination + nonce + contract_id)`. | **Increments** the nonce after successful verification (`increment_nonce`). | Yes (after transfer). |
| `claim` | `claim(recipient, ephemeral_account)` | Soroban auth entry from `recipient.require_auth()`. The recipient signs via auth entries; a relayer/SDK can submit the transaction on the recipient's behalf. | Does **not** increment the nonce directly. (See §[Operational Caveats](#operational-caveats).) | Yes. |

Because the two paths share storage but not authorization, and because
`update_authorized_destination` blocks when `nonce > 0` with the error
`Error::AccountAlreadySwept`, an operator who has *only* routed traffic
through `claim` may receive a misleading signal: `errorCode` says
"already swept" but no Ed25519-authorized sweep has actually occurred.

> **Workaround, not fix.** The contract requires `nonce > 0` as the
> lock-condition for `update_authorized_destination`. Reading the nonce in
> isolation is **not sufficient**; this runbook adds the event cross-check
> so the operator can decide whether the lock is the kind of "swept already"
> they care about.

---

## What "the lock is still in force" Means

The destination lock is in force when **at least one of these is true**:

1. A non-zero nonce is reported by `get_sweep_nonce()` **and** at least one
   `SweepCompleted` event with a positive `amount` is observable on the
   ledger.
2. A `SweepCompleted` event has been observed whose `destination` matches the
   currently-stored `authorized_destination` (or, in flexible mode, whose
   destination is the one the operator intends to overwrite).

If neither of those holds, the lock is **not** behaving as expected and
`update_authorized_destination` should be treated as suspect.

---

## Pre-Conditions

- [ ] Access to a Stellar RPC or Horizon endpoint that returns contract events
      for the controller instance.
- [ ] The controller contract address (`_C...`) and the well-known
      `sweep` topic name used by `SweepController::emit_sweep_completed`.
- [ ] A reference time window (ledger range) inside which the operator
      believes the relevant sweeps may have occurred.

---

## Step 1 — Read `get_nonce()` Directly

```bash
NETWORK='testnet'   # or 'mainnet'
CONTROLLER='CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAHOT3'

soroban contract invoke \
    --id "$CONTROLLER" \
    --network "$NETWORK" \
    -- \
    get_sweep_nonce
```

Record the response as `NONCE`. Note:

- `0` ⇒ the lock-condition in `update_authorized_destination` is **not**
  triggered. The contract will accept a destination change.
- `> 0` ⇒ the contract-side guard *will* reject a destination change with
  `Error::AccountAlreadySwept`. Do not assume yet which sweep path took us
  here; continue to step 2.

---

## Step 2 — Enumerate `SweepCompleted` Events

The `SweepCompleted` event is emitted from the controller instance with the
topic `sweep`. Pull all events emitted on the controller contract over the
window in question:

```bash
# Pull the latest 200 events; widen if you need a deeper history.
soroban events \
    --network "$NETWORK" \
    --contract "$CONTROLLER" \
    --topic "sweep" \
    --count 200
```

For each event, decode the JSON payload:

```json
{
  "ephemeral_account": "C...",
  "destination": "G...",
  "amount": 1000000
}
```

Tally events with `amount > 0`. Record:

- `N_SWEEP_COMPLETED_POSITIVE` — total positive-amount events observed.
- A list of distinct `destination` values.

> **Tooling caveat.** The exact CLI surface and JSON shape vary across
> `soroban-cli` versions; what matters is that you enumerate every event
> on `(controller, "sweep")` over the window, regardless of the renderer.

---

## Step 3 — Cross-Check and Decide

| `NONCE` | `N_SWEEP_COMPLETED_POSITIVE` | Decision |
| :--- | :--- | :--- |
| `0` | `0` | **Safe to call `update_authorized_destination` per the contract check**, but still confirm the off-chain intended-mode (locked vs flexible — see [`SweepController initialization checklist`](../checklists/sweep-controller-initialization-checklist.md)) before invoking. |
| `> 0` | `≥ 1` | **Do not call `update_authorized_destination`.** Lock is in force both contract-side and event-side; the destination cannot be rotated. |
| `> 0` | `0` | **Operate as if locked.** This is the ambiguous state that justifies this runbook: the contract blocked the call, but no positive-amount `SweepCompleted` is observable. Treat the lock as in force for safety. Investigate why events might be missing (RPC lag, partial state, replay of a reverted transaction). |
| `0` | `≥ 1` | **Anomaly.** A `SweepCompleted` event exists without a corresponding nonce increment. Treat the lock as in force. Do not call `update_authorized_destination` until an investigator signs off that this is expected (e.g. a once-off migration of the controller's storage). |

In every "do not call" row, **stop** and proceed to §4 below.

---

## Step 4 — If You Decide Not To Call `update_authorized_destination`

You have concluded that the destination cannot be safely rotated. Capture the
following in the operator ticket:

- [ ] The recorded `NONCE` from step 1.
- [ ] The full event list (timestamps + destinations) from step 2; attach as
      `events.json` to the ticket.
- [ ] The decision from step 3 with the row label that drove it.
- [ ] The contact who authorized the no-call (operator + on-call lead). The
      destination rotation request must be tracked to completion *somewhere*
      even though it is not being executed on-chain tonight.

Do **not** "just retry later" without first re-running this runbook from
step 1. Ledger state can change between retries.

---

## Operational Caveats

- **Nonce is incremented *before* the ephemeral sweep completes.** In
  `SweepController::sweep_account(..., increment_nonce: true)`, the controller
  calls `authorization::increment_nonce(env)` **before** invoking
  `EphemeralAccount::sweep` and **before** the token transfer. As a result, a
  nonce bump with no positive-amount `SweepCompleted` is not a mystery —
  it is the expected failure pattern when the ephemeral sweep reverts
  mid-flow or when the token transfer itself fails mid-flow. The runbook's
  "treat as locked" rule already covers this scenario; this caveat is here so
  an operator does not waste time hunting a non-bug.
- **`claim`-path `SweepCompleted` events do not prove funds movement.**
  `SweepController::claim` performs the ephemeral account state transition
  and emits a `SweepCompleted` event whose `amount` is computed from the
  payments recorded on the ephemeral account, but it does **not** call
  `transfers::execute_transfers`. Only `execute_sweep` actually moves tokens.
  Treat a positive-amount `SweepCompleted` from `claim` as evidence of
  *state transition*, not as evidence that funds reached the destination —
  verify movement out-of-band (e.g. payment-operation history on Horizon).
- **RPC lag.** Event indexing can lag the underlying ledger state by a few
  seconds. If `N_SWEEP_COMPLETED_POSITIVE = 0` and `NONCE > 0`, wait at least
  one ledger close (~5 seconds on Stellar public networks) and re-run steps
  1 and 2 before locking the no-call decision in.
- **Truncated event windows.** A bounded event pull may not show the original
  sweep that pushed the nonce up. When `NONCE > 0` but no `SweepCompleted`
  is observable in the window, widen the window before reverting to the
  contract check.
- **Migration pauses.** If the controller was migrated (different wasm hash)
  recently, `get_sweep_nonce` from the *new* wasm may not reflect sweeps that
  happened on the prior instance. Re-derive the live controller address from
  `bridgelet-audit/checklists/sweep-controller-initialization-checklist.md`'s
  binding-evidence step before trusting the nonce reading.
- **Relayer submission.** A `claim`-driven sweep may, in unusual edge cases,
  surface in storage but not in the event stream if the relayer's submission
  path was interrupted. This runbook consciously treats that scenario as
  "locked" to err on the side of preserving the destination.

---

## See Also

- `bridgelet-audit/checklists/sweep-controller-initialization-checklist.md` —
  captures go-live posture so this runbook has stable inputs after deployment.
- `bridgelet-audit/runbooks/validate-batch-initialize-salt-uniqueness.md` —
  the pre-flight that pairs with the go-live checklist.
- `docs/security.md` — `SweepController` is the only layer performing actual
  Ed25519 signature verification for sweeps; do not waive its checks based on
  a permissive reading of storage state.

---

## Related Issues

- **#288** — this runbook.
- **#295** — companion checklist for the deployment posture that feeds into
  this runbook's pre-conditions.
- **#290** — sibling runbook for `batch_initialize` salt uniqueness.
