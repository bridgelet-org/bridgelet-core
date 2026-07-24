# Cross-Checking ReserveContract's Configured Value Against the Hardcoded Constant

**Runbook type:** Operational  
**Contracts:** `ReserveContract`, `EphemeralAccount`  
**Relevant functions / constants:** `ReserveContract::get_base_reserve()`, `BASE_RESERVE_STROOPS = 1_000_000_000`  
**Tracking issue:** [#292](https://github.com/bridgelet-org/bridgelet-core/issues/292)

---

## Purpose

This runbook describes how an operator confirms whether the base reserve value
stored in the on-chain `ReserveContract` matches the compile-time constant
(`BASE_RESERVE_STROOPS`) that `EphemeralAccount` contracts use at initialization.

This comparison **must currently be done manually** because no on-chain logic
automatically enforces that the two values agree. This runbook frames the check
as a standing operational responsibility until (if ever) the two are wired
together in code.

---

## Background

### Two sources of truth for the base reserve

| Source | Location | Visibility | Mutable? |
|--------|----------|------------|----------|
| `BASE_RESERVE_STROOPS` compile-time constant | `contracts/ephemeral_account/src/lib.rs` | Compile-time only | Requires a contract redeploy |
| `ReserveContract::get_base_reserve()` | On-chain `ReserveContract` storage | Readable at any time | Admin can update via `set_base_reserve` |

**Current value of the compile-time constant:**

```rust
const BASE_RESERVE_STROOPS: i128 = 1_000_000_000; // 100 XLM
```

When an `EphemeralAccount` is initialized it calls `init_reserve_tracking` with
exactly this constant. From that moment forward the account tracks reserve
lifecycle based on `1_000_000_000` stroops, regardless of what `ReserveContract`
reports.

`ReserveContract` is a separately deployed on-chain store. Its purpose is to
expose a readable, admin-updatable reference value for the network's base
reserve. The two values *should* agree, but there is no on-chain assertion that
enforces this invariant.

---

## When to run this check

Run this verification:

- After any `ReserveContract::set_base_reserve` call.
- After any redeploy of the `EphemeralAccount` contract (which may change the compile-time constant).
- As part of a periodic operational audit (recommended: at least monthly, or after any Stellar network upgrade that changes base reserve requirements).
- Whenever a discrepancy is suspected (e.g. an `EphemeralAccount` reserve reclaim returns an unexpected amount).

---

## Pre-conditions

- [ ] You know the deployed contract ID of the target `ReserveContract` instance.
- [ ] You know the deployed contract ID (or source code version) of the `EphemeralAccount` instance(s) you are auditing.
- [ ] You have access to the Stellar CLI or equivalent SDK tooling.
- [ ] The Stellar network is reachable.

---

## Steps

### 1. Read the on-chain value from `ReserveContract`

```bash
stellar contract invoke \
  --id <RESERVE_CONTRACT_ID> \
  --network <NETWORK> \
  --source <ANY_ACCOUNT> \
  -- \
  get_base_reserve
```

**Possible outputs:**

| Output | Meaning |
|--------|---------|
| `Some(<integer>)` | The admin has set a base reserve; note the value. |
| `None` | No base reserve has been configured yet on this `ReserveContract`. |

If the result is `None`, the contract has not been initialized with a reserve
value. This is a misconfiguration that should be remediated before relying on
this contract as a reference.

---

### 2. Identify the compile-time constant in the deployed `EphemeralAccount`

The constant is set at compile time in the contract source. Check the version
of the source that was used to build the deployed WASM:

```
contracts/ephemeral_account/src/lib.rs
const BASE_RESERVE_STROOPS: i128 = 1_000_000_000;
```

If you are auditing a live deployment and do not have the source, use the
deployment artifact metadata or the deployment record in
`deployments/testnet.json` to identify the commit/tag, then inspect the source
at that revision.

**Current known value:** `1_000_000_000` stroops (= 100 XLM).

---

### 3. Compare the two values

| Scenario | Interpretation | Action |
|----------|---------------|--------|
| `ReserveContract` value equals `BASE_RESERVE_STROOPS` | [OK] Values are consistent. No action required. | Record the check in your audit log. |
| `ReserveContract` value differs from `BASE_RESERVE_STROOPS` | [WARN] Values are out of sync. | See remediation guidance below. |
| `ReserveContract` returns `None` | [WARN] Reference contract is unconfigured. | Initialize it via `set_base_reserve`. |

---

### 4. (If out of sync) Determine which value is authoritative

Because no on-chain logic automatically reconciles the two, you must determine
the correct value from context:

- **If the Stellar network base reserve changed:** Update `ReserveContract` to
  the new network value *and* plan a redeploy of `EphemeralAccount` with an
  updated constant.
- **If `ReserveContract` was updated by an admin:** Assess whether the new
  value is intentional. If it was an error, revert it via another
  `set_base_reserve` call.
- **If `EphemeralAccount` was redeployed with a different constant:** Verify
  the new constant is correct and update `ReserveContract` to match.

Any active `EphemeralAccount` instances initialized before the constant was
changed will continue to use the old value for the lifetime of those accounts.
Only newly initialized accounts pick up a redeployed constant.

---

### 5. Record the outcome

Document the result of the check, including:

- Date and time of the check.
- Network (testnet / mainnet).
- `ReserveContract` ID queried.
- `EphemeralAccount` source version / commit audited.
- Value returned by `get_base_reserve()`.
- Compile-time constant value.
- Whether values matched.
- Any remediation steps taken.

---

## Limitations and caveats

- **No automatic enforcement.** Nothing on-chain prevents the two values from
  diverging. This check is purely manual and advisory.
- **Per-account snapshot.** Each `EphemeralAccount` captures `BASE_RESERVE_STROOPS`
  at initialization time. If the constant changes, already-initialized accounts
  are unaffected — only new accounts pick up the new value.
- **Standing check.** Until the two sources are wired together in contract code,
  this check should remain part of the regular operational runbook rotation.

---

## Related runbooks

- [`emergency-destination-lock.md`](./emergency-destination-lock.md) — Lock down the sweep destination during a key compromise.
