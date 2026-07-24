# Cross-Contract Call Safety Analysis

## Overview

The bridgelet-core system consists of two primary Soroban smart contracts:

1. **SweepController** — orchestrates sweep operations and manages authorization
2. **EphemeralAccount** — holds funds temporarily and enforces lifecycle rules

SweepController makes cross-contract calls into EphemeralAccount during
`execute_sweep()` and `claim()` operations.  This document analyses the
security implications.

## Cross-Contract Call Points

### 1. `SweepController.execute_sweep()` → `EphemeralAccount.sweep()`

```
SweepController                          EphemeralAccount
    │                                         │
    ├─ verify Ed25519 signature               │
    ├─ increment nonce                        │
    ├─ authorize_as_current_contract ─────────>│  (Soroban auth entry)
    │                                         ├─ verify controller auth
    │                                         ├─ validate state
    │                                         ├─ status → Swept
    │                                         ├─ reclaim_reserve_to()
    │                                         └─ emit events
    ├─ transfers::execute_transfers() ───────>│  (token transfers)
    └─ emit_sweep_completed()                 │
```

### 2. `SweepController.claim()` → `EphemeralAccount.sweep_claim()`

```
SweepController                          EphemeralAccount
    │                                         │
    ├─ recipient.require_auth()               │
    ├─ authorize_as_current_contract ─────────>│
    │                                         ├─ verify controller auth
    │                                         ├─ validate state
    │                                         ├─ status → Swept
    │                                         └─ reclaim_reserve_to()
    └─ emit_sweep_completed()                 │
```

## Threat Analysis

### Threat 1: Malicious EphemeralAccount Implementation

**Risk:** An attacker deploys a modified EphemeralAccount that pretends to
sweep but doesn't actually transfer funds, or returns misleading state.

**Mitigation:**
- The SweepController uses `contractimport!` to embed the EphemeralAccount
  WASM hash at compile time.  In production, the controller is initialized
  with the EphemeralAccount's *deployed contract address*, not its WASM hash.
- The `sweep()` and `sweep_claim()` calls are cross-contract invocations
  with Soroban auth entries — the EphemeralAccount must accept the
  controller as an authorized invoker.
- Even if a malicious contract is called, the SweepController reads the
  account's `get_info()` to verify `payment_received` status and payment
  amounts before proceeding with transfers.
- The token transfers in `transfers::execute_transfers()` go directly to
  the SEP-41 token contracts — a malicious EphemeralAccount cannot redirect
  these because the `from` address is the *real* ephemeral account
  (determined by its contract instance address on-chain).

**Residual risk:** LOW.  The worst case is that a malicious EphemeralAccount
could cause the sweep to fail (by returning incorrect state), but it cannot
steal funds because token transfers are executed against the actual
on-chain token contracts.

### Threat 2: Cross-Contract Replay

**Risk:** A signature valid for one SweepController instance is replayed
against a different instance.

**Mitigation:**
- The signed message includes `env.current_contract_address()` — the
  SweepController's own contract address.  Different instances have
  different addresses, so cross-instance replay is impossible.
- The nonce is per-instance (stored in the controller's own instance
  storage), providing additional replay protection.

### Threat 3: EphemeralAccount Contract Hash Verification

**Risk:** Should SweepController verify the EphemeralAccount's WASM hash
before calling it?

**Current approach:** No explicit WASM hash verification is performed at
call time.  The rationale:

1. **Soroban's trust model** — Soroban contracts are identified by their
   *instance address*, not their WASM hash.  The instance address is a
   deterministic function of the deployer, WASM hash, and salt.  Once
   deployed, the contract code is immutable (unless upgraded via the
   admin-controlled `upgrade()` function).

2. **The contract is already trusted** — The EphemeralAccount's address is
   passed to `SweepController.initialize()` by the contract creator, who
   is a trusted administrator.  If the creator passes a malicious address,
   they are the attacker.

3. **WASM hash check adds complexity** — Verifying the WASM hash on every
   call would require either:
   - Storing the expected hash in SweepController storage (additional
     storage cost and admin burden), or
   - Querying the on-chain WASM hash (additional cross-contract call).

**Recommendation:** For high-value deployments, add an optional
`ephemeral_account_wasm_hash` parameter to `initialize()` and verify it
on the first call or on demand.  This is a defence-in-depth measure, not
a strict requirement.

### Threat 4: State Inconsistency Between Contracts

**Risk:** SweepController and EphemeralAccount disagree on the account state.

**Mitigation:**
- Both contracts are called within a single Soroban transaction, which
  provides atomic all-or-nothing execution.
- If any step fails (signature verification, state validation, token
  transfer), the entire transaction reverts — including any state changes
  in either contract.
- The `sweep()` function in EphemeralAccount transitions status to `Swept`
  *before* the SweepController reads the info, preventing TOCTOU (time-of-
  check-time-of-use) issues.

## Recommendations

1. **Add optional WASM hash verification** for high-security deployments.
2. **Event monitoring** — Index `SweepCompleted` and `ReserveReclaimed`
   events to detect anomalies (e.g., sweeps to unexpected destinations).
3. **Rate limiting** — The nonce mechanism already prevents replay, but
   consider adding a minimum ledger gap between sweeps for the same
   account to prevent rapid draining attacks.
4. **Admin key management** — The SweepController creator/admin has
   significant power (can update authorized destination, initiate signer
   rotation).  Use a multisig or hardware wallet for the admin key.
