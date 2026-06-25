# Contract Upgrade Guide and Rollback Procedure

**Version:** 1.0
**Last Updated:** June 24, 2026
**Status:** MVP

---

## Table of Contents

1. [Overview](#overview)
2. [How update_current_contract_wasm() Works](#how-update_current_contract_wasm-works)
3. [Pre-Upgrade Checklist](#pre-upgrade-checklist)
4. [Testnet Verification Before Mainnet](#testnet-verification-before-mainnet)
5. [In-Flight Claims During an Upgrade](#in-flight-claims-during-an-upgrade)
6. [Performing the Upgrade](#performing-the-upgrade)
7. [Post-Upgrade Verification](#post-upgrade-verification)
8. [Rollback Procedure](#rollback-procedure)
9. [Upgrade History Log](#upgrade-history-log)

---

## Overview

Soroban contracts are immutable by default. To ship a new version of a deployed
contract without changing its address or losing its storage state, Soroban
exposes `update_current_contract_wasm()`. This function replaces the WASM bytecode
bound to the current contract address in a single atomic operation while leaving
all instance storage (account state, payments, reserve tracking, etc.) intact.

**Contracts covered by this guide:**

| Contract | Package | Upgradeable? |
|---|---|---|
| `EphemeralAccount` | `contracts/ephemeral_account` | Yes |
| `SweepController` | `contracts/sweep_controller` | Yes |
| `ReserveContract` | `contracts/reserve_contract` | Yes |

**Who can upgrade?**

Only the `authorized_controller` stored during `initialize()` may call the upgrade
entry point. Any other caller will receive `Error::Unauthorized`.

---

## How update_current_contract_wasm() Works

`update_current_contract_wasm()` is a host function provided by the Soroban
environment. Calling it replaces the WASM module associated with the current
contract instance. The replacement takes effect at the **next invocation** after
the upgrading transaction closes.

### What changes

- The WASM bytecode (contract logic) is replaced.
- The contract address stays the same.
- All instance storage is preserved exactly as-is.
- The contract version number (stored under `DataKey::ContractVersion`) should
  be incremented by the new WASM as part of its own initialization guard.

### What does not change

- Contract address.
- Storage keys and their values.
- Outstanding ledger TTL for instance storage.
- Any off-chain references (SDK, indexers, client apps) pointing to the contract
  address continue to work without reconfiguration.

### Upgrade entry point pattern

Add an `upgrade()` function to the contract that gates the call behind
`authorized_controller`:

```rust
/// Replace the contract WASM with a new version.
///
/// # Errors
/// Returns `Error::Unauthorized` if the caller is not the authorized controller.
/// Returns `Error::NotInitialized` if the contract has not been initialized.
pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>) -> Result<(), Error> {
    if !storage::is_initialized(&env) {
        return Err(Error::NotInitialized);
    }

    let controller = storage::get_authorized_controller(&env)
        .ok_or(Error::Unauthorized)?;
    controller.require_auth();

    env.deployer().update_current_contract_wasm(new_wasm_hash);
    Ok(())
}
```

The `new_wasm_hash` is the SHA-256 hash of the new WASM blob, obtained after
uploading the blob with `stellar contract upload` (see
[Performing the Upgrade](#performing-the-upgrade)).

---

## Pre-Upgrade Checklist

Complete every item before triggering an upgrade on any network.

### Code readiness

- [ ] New contract version compiles cleanly: `cargo build --target wasm32-unknown-unknown --release`
- [ ] All unit tests pass: `cargo test -p <contract_name>`
- [ ] `DataKey` enum is backward-compatible -- no existing keys renamed or removed
- [ ] `CONTRACT_VERSION` constant incremented in the new WASM
- [ ] Any new storage keys are handled with `unwrap_or` / `unwrap_or_else` defaults
  so old instances without those keys do not panic on first call
- [ ] Rustdoc updated for any changed public functions

### Operational readiness

- [ ] Testnet upgrade completed and verified (see next section)
- [ ] SDK team notified if function signatures changed
- [ ] Indexers and event consumers updated for any new event payloads
- [ ] Rollback WASM hash noted (the hash of the **currently deployed** WASM)
- [ ] Upgrade window chosen to minimise in-flight activity
- [ ] On-call engineer available during and after the upgrade window

---

## Testnet Verification Before Mainnet

Always upgrade on testnet first and run the full verification sequence before
touching mainnet.

### Step 1 -- Build the new WASM

```bash
cd contracts/ephemeral_account
cargo build --target wasm32-unknown-unknown --release
```

The compiled artefact is at:
`target/wasm32-unknown-unknown/release/ephemeral_account.wasm`

### Step 2 -- Upload the WASM blob to testnet

```bash
stellar contract upload \
  --wasm target/wasm32-unknown-unknown/release/ephemeral_account.wasm \
  --network testnet \
  --source <DEPLOYER_SECRET_KEY>
```

The command returns the WASM hash. **Save this value** -- it is required for the
upgrade call and for rollback documentation.

```
# Example output
WASM hash: a1b2c3d4e5f6...
```

### Step 3 -- Record the current WASM hash (rollback reference)

```bash
stellar contract info \
  --id <CONTRACT_ID> \
  --network testnet
```

Note the `wasm_hash` field. This is what you will re-upload if you need to
roll back.

### Step 4 -- Invoke the upgrade on testnet

```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  --source <AUTHORIZED_CONTROLLER_SECRET_KEY> \
  -- upgrade \
  --new_wasm_hash <NEW_WASM_HASH>
```

### Step 5 -- Verify the new version is active

```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  --source <ANY_ACCOUNT> \
  -- version
```

The returned value should match `CONTRACT_VERSION` in the new source.

### Step 6 -- Smoke-test core flows on testnet

Run through each of these manually or via the test scripts:

| Flow | Expected outcome |
|---|---|
| `initialize()` with future expiry | Returns `Ok(())` |
| `initialize()` with past expiry | Returns `Error(Contract, #5)` |
| `record_payment()` | Status changes to `PaymentReceived` |
| `sweep()` | Status changes to `Swept` |
| `expire()` past expiry ledger | Status changes to `Expired` |
| `get_info()` on existing account | All prior state intact |

Only proceed to mainnet after every row above passes.

---

## In-Flight Claims During an Upgrade

Understanding what happens to active ephemeral accounts during an upgrade is
critical for safe deployments.

### What "in-flight" means

An in-flight claim is any ephemeral account that has been `initialize()`d and
has not yet reached a terminal state (`Swept` or `Expired`). These accounts
have live state in instance storage.

### Effect of the upgrade on in-flight accounts

| Account state at upgrade time | Effect |
|---|---|
| `Active` (no payment yet) | No impact. State preserved. Next call uses new WASM logic. |
| `PaymentReceived` (awaiting sweep) | No impact. State preserved. Sweep will use new WASM logic. |
| `Swept` (terminal) | No impact. Already in terminal state. |
| `Expired` (terminal) | No impact. Already in terminal state. |

Because `update_current_contract_wasm()` is atomic and only replaces logic --
not storage -- all in-flight accounts continue with their existing state intact.
The only change is that subsequent calls to those contract instances execute the
new WASM.

### Risk: storage schema changes

If the new WASM reads a storage key that was written by the old WASM under a
different type, calls will panic. This is prevented by the pre-upgrade checklist
item requiring backward-compatible `DataKey` values and safe `unwrap_or`
defaults.

### Recommended upgrade window

Choose a window when the number of accounts in `PaymentReceived` state is
lowest. The SDK exposes account state via `get_status()` -- query active
instances before scheduling the window.

```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --network mainnet \
  --source <ANY_ACCOUNT> \
  -- get_status
```

---

## Performing the Upgrade

Follow this exact sequence for a mainnet upgrade.

```
1. Complete pre-upgrade checklist
2. Upload new WASM to mainnet
3. Record rollback hash
4. Invoke upgrade()
5. Verify version()
6. Run smoke tests
7. Update upgrade history log
```

### Upload WASM to mainnet

```bash
stellar contract upload \
  --wasm target/wasm32-unknown-unknown/release/ephemeral_account.wasm \
  --network mainnet \
  --source <DEPLOYER_SECRET_KEY>
```

### Invoke upgrade on mainnet

```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --network mainnet \
  --source <AUTHORIZED_CONTROLLER_SECRET_KEY> \
  -- upgrade \
  --new_wasm_hash <NEW_WASM_HASH>
```

### Verify

```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --network mainnet \
  --source <ANY_ACCOUNT> \
  -- version
```

Expected output: the new `CONTRACT_VERSION` value.

---

## Post-Upgrade Verification

After the upgrade transaction confirms, verify the following within 15 minutes:

- [ ] `version()` returns the new version number
- [ ] `get_status()` on a known active account returns the expected status
- [ ] SDK can call `initialize()` on a new contract instance without error
- [ ] No unexpected errors appear in the event stream
- [ ] Indexers and monitoring dashboards show normal activity

If any check fails, initiate the rollback procedure immediately.

---

## Rollback Procedure

A rollback re-uploads the previous WASM and calls `upgrade()` with its hash.
Because storage is untouched, accounts return to the previous logic without data
loss.

### When to roll back

- `version()` returns an unexpected value after upgrade
- Any smoke-test flow from the testnet verification section fails on mainnet
- Error rate on contract invocations spikes above baseline within 15 minutes
  of upgrade

### Rollback steps

**Step 1 -- Retrieve the previous WASM**

The previous WASM blob must be available locally or in your deployment artefacts.
If you do not have the file, retrieve it from the network using the previously
recorded WASM hash:

```bash
stellar contract fetch \
  --wasm-hash <PREVIOUS_WASM_HASH> \
  --network mainnet \
  --out previous_version.wasm
```

**Step 2 -- Re-upload the previous WASM**

```bash
stellar contract upload \
  --wasm previous_version.wasm \
  --network mainnet \
  --source <DEPLOYER_SECRET_KEY>
```

This returns a WASM hash. It should match `<PREVIOUS_WASM_HASH>` exactly,
since the blob is identical.

**Step 3 -- Invoke upgrade() with the previous hash**

```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --network mainnet \
  --source <AUTHORIZED_CONTROLLER_SECRET_KEY> \
  -- upgrade \
  --new_wasm_hash <PREVIOUS_WASM_HASH>
```

**Step 4 -- Verify rollback**

```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --network mainnet \
  --source <ANY_ACCOUNT> \
  -- version
```

Expected output: the previous `CONTRACT_VERSION` value.

**Step 5 -- Run smoke tests**

Repeat the smoke-test table from
[Testnet Verification](#testnet-verification-before-mainnet) against mainnet to
confirm all flows are healthy.

**Step 6 -- Document the rollback**

Add an entry to the [Upgrade History Log](#upgrade-history-log) recording the
reason for rollback, the WASM hashes involved, and the time of each transaction.

### Rollback limitations

- Rollback restores the previous **logic** only. Any storage written by the new
   WASM storage in the window between upgrade and rollback remains in storage. Ensure the
  old WASM handles any new keys gracefully (they will be absent or ignored).
- If the new WASM storage keys were deleted or migrated, those deletions cannot be
  automatically reversed. Assess on a case-by-case basis.

---

## Upgrade History Log

Maintain this table as part of every upgrade and rollback operation.

| Date | Network | Contract | Old Version | New Version | Old WASM Hash | New WASM Hash | Outcome | Engineer |
|---|---|---|---|---|---|---|---|---|
| _(first upgrade)_ | | | | | | | | |

---

*For questions about the upgrade process, see [architecture.md](architecture.md)
for contract design context and [testing.md](testing.md) for test procedures.*
