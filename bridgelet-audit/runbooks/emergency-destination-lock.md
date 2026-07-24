# Emergency Destination Lock

**Runbook type:** Operational  
**Contract:** `SweepController`  
**Relevant functions:** `update_authorized_destination()`, `has_authorized_destination()`, `get_authorized_destination()`  
**Tracking issue:** [#284](https://github.com/bridgelet-org/bridgelet-core/issues/284)

---

## Purpose

This runbook describes the steps an operator takes to lock down—or replace—the
`authorized_destination` stored in a `SweepController` instance when a private
key compromise is suspected or confirmed.

It is a **purely descriptive** record of current on-chain behavior. No code
changes are proposed or required to execute these steps.

---

## Background

`SweepController` can be deployed in two modes:

| Mode | `authorized_destination` | Who can receive swept funds |
|------|--------------------------|-----------------------------|
| Flexible | Not set | Any destination passed to `execute_sweep` |
| Locked | Set to a specific address | Only that address |

When the contract is in **locked mode**, every call to `execute_sweep` is
rejected unless the `destination` argument matches the stored address. If the
wallet controlling the locked destination is compromised, the operator must
replace that address before any attacker can attempt a sweep.

The update window is controlled by a single invariant:

> **`update_authorized_destination` can only be called while the sweep nonce
> is still `0`.**  Once any sweep has been executed the nonce increments to `1`
> (or higher) and further updates are permanently blocked with
> `Error::AccountAlreadySwept`.

---

## Pre-conditions

Before executing these steps, confirm all of the following:

- [ ] You have access to the **creator** key that was used when `SweepController::initialize` was called.
- [ ] No sweep has yet been executed against this controller (sweep nonce = 0).  
      *If the nonce is already > 0, no destination update is possible — escalate immediately.*
- [ ] You have identified a **safe replacement destination** address and confirmed you control it.
- [ ] The Stellar network is reachable and you have sufficient XLM on the creator account for transaction fees.

---

## Steps

### 1. Confirm no sweep has occurred yet

Query the contract's sweep nonce via the Stellar CLI or your preferred SDK.
If the nonce is **not 0**, stop here — `update_authorized_destination` will
revert with `Error::AccountAlreadySwept`. Treat this as an incident and
escalate according to your incident-response process.

```bash
# Using stellar-cli (replace placeholders with real values)
stellar contract invoke \
  --id <SWEEP_CONTROLLER_CONTRACT_ID> \
  --network <NETWORK> \
  --source <CREATOR_SECRET_KEY> \
  -- \
  get_sweep_nonce
```

Expected output when safe to proceed: `0`

---

### 2. Call `update_authorized_destination`

Submit a transaction signed by the **creator** key that calls
`update_authorized_destination` with the replacement address.

```bash
stellar contract invoke \
  --id <SWEEP_CONTROLLER_CONTRACT_ID> \
  --network <NETWORK> \
  --source <CREATOR_SECRET_KEY> \
  -- \
  update_authorized_destination \
  --new_destination <REPLACEMENT_DESTINATION_ADDRESS>
```

A successful invocation emits a `DestinationUpdated` event containing:
- `old_destination` — the address being replaced (or `None` if none was set)
- `new_destination` — the replacement address you supplied

If the call fails with `Error::AuthorizationFailed`, check that the signing key
matches the creator recorded at initialization. If it fails with
`Error::AccountAlreadySwept`, the nonce is no longer 0 (see step 1).

---

### 3. Verify the new destination is active

After the transaction confirms, call both read-only helpers to verify the
state reflects the update:

**3a. Confirm a destination is now set**

```bash
stellar contract invoke \
  --id <SWEEP_CONTROLLER_CONTRACT_ID> \
  --network <NETWORK> \
  --source <ANY_ACCOUNT> \
  -- \
  has_authorized_destination
```

Expected output: `true`

**3b. Confirm it matches the replacement address**

```bash
stellar contract invoke \
  --id <SWEEP_CONTROLLER_CONTRACT_ID> \
  --network <NETWORK> \
  --source <ANY_ACCOUNT> \
  -- \
  get_authorized_destination
```

Expected output: `<REPLACEMENT_DESTINATION_ADDRESS>`

If either result is unexpected, do **not** proceed. Halt all sweep operations
and investigate.

---

### 4. Revoke access to the compromised key

Once the on-chain destination is updated, take all off-chain steps to revoke
the compromised key:

- Remove the old private key from all key stores, HSMs, and configuration files.
- Rotate any secrets that were co-located with the compromised key.
- Notify stakeholders according to your incident-response policy.

---

## Limitations and caveats

- **One-time window.** The `authorized_destination` can only be updated when
  `sweep_nonce == 0`. There is no on-chain mechanism to re-open this window
  after a sweep occurs. This behavior is intentional and is not proposed to
  change in this runbook.
- **Flexible mode.** If the controller was deployed without an initial
  `authorized_destination` (flexible mode), calling `update_authorized_destination`
  will *set* a destination for the first time, effectively locking the contract.
  The same nonce-zero constraint applies.
- **Monitoring.** Off-chain event indexers should watch for `DestinationUpdated`
  events on all active `SweepController` instances to detect unexpected changes.

---

## Related runbooks

- [`cross-check-reserve-contract-vs-hardcoded-reserve.md`](./cross-check-reserve-contract-vs-hardcoded-reserve.md) — Verify on-chain reserve configuration.
