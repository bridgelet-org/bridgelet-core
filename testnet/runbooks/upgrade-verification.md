# Runbook: Verify an EphemeralAccount Upgrade

## Purpose

Verify that an admin-authorized `EphemeralAccount::upgrade(new_wasm_hash)` call
completed on testnet, that the target contract instance points to the intended
WASM hash, that **the new contract logic is functionally active**, and that the
upgrade did not silently corrupt or mutate stored account state.

Because `EphemeralAccount::upgrade()` emits no contract event, a successful
transaction submission and an updated code hash on the ledger entry are
necessary but not sufficient:
1. An updated code pointer proves the ledger entry was written, but does not
   prove the runtime module executes correctly without unexpected host traps or
   storage deserialization failures.
2. In-place upgrades modify contract behavior on the existing instance without
   changing its contract ID. Integrators and automated test suites need verified
   behavioral confirmation that the change took effect.
3. This runbook directly pairs with [`testnet/registry/upgrade-history.md`](../registry/upgrade-history.md):
   every upgrade must be verified through the behavioral checks detailed below
   before being logged in the upgrade registry.

> **Operational Note**: This document is an actionable operational runbook.
> Replace placeholder values (`<...>`) with live testnet values. Do not treat
> example commands as executed history.

---

## Current contract behavior & upgrade mechanics

The `EphemeralAccount` smart contract (`contracts/ephemeral_account/src/lib.rs`):

1. **Initialization check**: Rejects uninitialized instances with `Error::NotInitialized`.
2. **Admin authorization**: Reads the admin address stored during `initialize()`,
   requiring that admin's signature:
   ```rust
   let admin = storage::get_admin(&env).ok_or(Error::NotUpgradeAdmin)?;
   admin.require_auth();
   ```
3. **In-place code replacement**: Invokes the Soroban host deployer:
   ```rust
   env.deployer().update_current_contract_wasm(new_wasm_hash);
   ```
4. **No event emitted**: Soroban does not emit an automatic event on WASM
   replacement, and `EphemeralAccount` does not emit a custom event.
5. **Instance isolation**: An upgrade only updates the single target contract
   instance. It does not upgrade other deployed `EphemeralAccount` instances or
   any factory deployment templates.
6. **Storage continuity**: Contract instance storage (creator, status, expiry,
   recovery address, payment records, reserves) is retained across the upgrade.
   However, Soroban enforces no automatic schema migration; if the new WASM
   expects an incompatible storage layout, subsequent invocations will trap.

---

## Required evidence & identifiers

| Placeholder | Meaning | How to obtain |
|---|---|---|
| `<EA_CONTRACT_ID>` | Target `EphemeralAccount` contract ID (e.g. `C...`) | [`testnet/registry/contract-status.md`](../registry/contract-status.md) or test fixture |
| `<ADMIN_IDENTITY>` | Local identity for the stored contract admin | Configured in `stellar keys` with testnet funds |
| `<READER_IDENTITY>` | Funded identity for read-only simulations | `stellar keys generate --global testnet-reader` |
| `<OLD_WASM_HASH>` | 64-hex WASM hash immediately before upgrade | On-chain query or baseline in [`../registry/upgrade-history.md`](../registry/upgrade-history.md) |
| `<NEW_WASM_HASH>` | 64-hex hash of candidate WASM uploaded to testnet | Output of `stellar contract upload` |
| `<UPGRADE_TX_HASH>` | Transaction hash returned by `upgrade()` call | CLI invocation output / Soroban RPC |
| `<UPGRADE_LEDGER>` | Ledger sequence number including the upgrade tx | RPC `getTransaction` response |
| `<CHANGED_METHOD>` | Method whose logic/output changed in the release | Release notes / changelog (e.g. `simulate_sweep`, `get_info_paginated`) |

Baseline reference: The initial testnet deployment recorded in
`deployments/testnet.json` is `5e667ea0687341bdccc81538492143b573777dd04b2450cdeb89b02cee62c58e`.
Use this as `<OLD_WASM_HASH>` if no prior upgrade has been logged.

> [!CAUTION]
> Never include private keys, secret seeds, auth signatures, or sensitive
> environment tokens in verification reports, incident logs, or registry entries.

---

## Stop conditions

**DO NOT** submit `upgrade()` or declare an upgrade verified if any of the following apply:

- The target contract ID or network passphrase does not match testnet (`Test SDF Network ; September 2015`).
- Pre-upgrade simulation of `get_status` or `get_info` fails, indicating the instance is uninitialized or corrupted.
- The stored admin address cannot be authorized by the executing identity.
- The candidate WASM hash has not been pre-uploaded to testnet or differs from the release build SHA-256.
- The release introduces an undocumented breaking change to storage keys or existing public method signatures.
- Upgrades are attempted directly on live production-mirror accounts without dry-running on a disposable test account first.

---

## Verification Procedure

### 1. Capture pre-upgrade state baseline

Before calling `upgrade()`, record a complete snapshot of the contract's stored
state using `--simulate-only`. This establishes the baseline for proving state
preservation post-upgrade:

```bash
# 1. Capture primary account info (creator, expiry, recovery, payment count)
stellar contract invoke \
  --simulate-only \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  get_info

# 2. Capture lifecycle status (0 = Active, 1 = Swept, 2 = Expired)
stellar contract invoke \
  --simulate-only \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  get_status

# 3. Capture reserve state
stellar contract invoke \
  --simulate-only \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  get_reserve_remaining

stellar contract invoke \
  --simulate-only \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  is_reserve_reclaimed
```

Save the exact JSON outputs, current ledger sequence, and timestamp.

---

### 2. Record the pre-upgrade code identity

Confirm and record `<OLD_WASM_HASH>` directly from the network:

**Option A: Using Stellar CLI (preferred)**
```bash
stellar contract info hash \
  --contract-id <EA_CONTRACT_ID> \
  --network testnet
```

**Option B: Fetching bytecode directly**
```bash
stellar contract fetch \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --out-file /tmp/pre_upgrade.wasm

sha256sum /tmp/pre_upgrade.wasm | awk '{print $1}'
```

**Option C: Querying Soroban RPC `getLedgerEntries`**
Query the contract instance key and extract the `executable.wasm_hash` field.
Verify that the recorded hash matches the entry in [`testnet/registry/upgrade-history.md`](../registry/upgrade-history.md).

Also export the existing interface schema for later comparison:
```bash
stellar contract info interface \
  --contract-id <EA_CONTRACT_ID> \
  --network testnet > /tmp/interface_pre.json
```

---

### 3. Verify the candidate WASM

Before invocation, verify that the new WASM is properly installed on testnet:

1. Confirm candidate hash format: exactly 64 hexadecimal characters (32 bytes).
2. Confirm candidate WASM is uploaded to testnet:
   ```bash
   # Uploads bytecode and returns the candidate WASM hash
   NEW_WASM_HASH=$(stellar contract upload \
     --wasm target/wasm32-unknown-unknown/release/ephemeral_account.wasm \
     --network testnet \
     --source <ADMIN_IDENTITY>)
   echo "Candidate WASM hash: $NEW_WASM_HASH"
   ```
3. Check the candidate interface against the approved release pull request:
   ```bash
   stellar contract inspect \
     --wasm target/wasm32-unknown-unknown/release/ephemeral_account.wasm
   ```

---

### 4. Submit the upgrade transaction

Execute the upgrade using the stored admin identity:

```bash
stellar contract invoke \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <ADMIN_IDENTITY> \
  -- \
  upgrade \
  --new_wasm_hash <NEW_WASM_HASH>
```

Capture the standard output and record:
- Transaction hash: `<UPGRADE_TX_HASH>`
- Finalized ledger sequence: `<UPGRADE_LEDGER>`
- Submission timestamp (UTC)

If the transaction fails:
- Error `NotUpgradeAdmin` (code 15): Caller is not the registered admin.
- Error `NotInitialized` (code 1): Contract has no admin initialized.
- Host authorization failure: Admin signature was omitted or invalid.
- Do not proceed to verification if submission failed.

---

### 5. Verify transaction finality

Confirm the transaction has been included in a finalized ledger via Soroban RPC:

```bash
curl -fsS "https://soroban-testnet.stellar.org" \
  -H 'Content-Type: application/json' \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getTransaction",
    "params": {
      "hash": "<UPGRADE_TX_HASH>"
    }
  }'
```

Verify:
- `"status": "SUCCESS"`
- `"latestLedger"` is greater than or equal to `<UPGRADE_LEDGER>`.
- No contract execution traps occurred.

---

### 6. Verify the on-chain code pointer

Confirm that the contract instance executable on the ledger now references
`<NEW_WASM_HASH>`:

```bash
ACTUAL_HASH=$(stellar contract info hash \
  --contract-id <EA_CONTRACT_ID> \
  --network testnet)

echo "Expected: <NEW_WASM_HASH>"
echo "Actual:   $ACTUAL_HASH"
```

Alternatively, fetch the live WASM and compute its checksum:
```bash
stellar contract fetch \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --out-file /tmp/post_upgrade.wasm

sha256sum /tmp/post_upgrade.wasm
```

The resulting hash **must equal `<NEW_WASM_HASH>` exactly**. If the hash still
equals `<OLD_WASM_HASH>`, the upgrade transaction either reverted or targeted
a different contract instance.

---

### 7. Verify the deployed interface

Inspect the updated interface and verify that any new or modified methods
match the release specification:

```bash
stellar contract info interface \
  --contract-id <EA_CONTRACT_ID> \
  --network testnet > /tmp/interface_post.json

diff -u /tmp/interface_pre.json /tmp/interface_post.json
```

Verify that expected public methods are present and no required functions were
inadvertently dropped.

---

### 8. Run a behavior probe (Confirm new logic took effect)

> [!IMPORTANT]
> **Why this step is critical**: Code-hash equality proves that the ledger entry
> was updated, but only an execution probe proves that:
> 1. The new WASM binary is active in the Soroban VM runtime and RPC caches.
> 2. The contract logic changes (bug fixes, calculation updates, error codes)
>    execute as intended on live ledger state.
> 3. Existing contract instance storage successfully deserializes under the new
>    code without runtime panic.

Select and execute the appropriate behavioral probe depending on what changed in
the upgrade:

#### Scenario A: Modified computation / bugfix verification

When the upgrade fixes an internal calculation, rounding issue, or simulation
behavior (e.g. reserve calculations or `simulate_sweep` fee formulas):

Run a read-only simulation comparing the new output with the pre-upgrade output:

```bash
stellar contract invoke \
  --simulate-only \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  simulate_sweep \
  --destination <PROBE_DESTINATION_ADDRESS>
```

**Verification checklist:**
- Inspect the returned tuple `(payments, error_code)`.
- Confirm that the calculation (e.g. remaining balance, fee deduction, or
  return struct) matches the **new formula** documented in the release notes.
- Confirm that the return value diverges from the pre-upgrade baseline in the
  exact manner intended by the fix.

#### Scenario B: Newly introduced method or query endpoint

When the upgrade introduces a new method (such as paginated inspection
`get_info_paginated` or a version query):

1. **Pre-upgrade baseline check**: Calling this method on the old code returned
   an RPC / host error (`HostError` or method symbol not found).
2. **Post-upgrade verification**: Invoke the new method:
   ```bash
   stellar contract invoke \
     --simulate-only \
     --id <EA_CONTRACT_ID> \
     --network testnet \
     --source <READER_IDENTITY> \
     -- \
     get_info_paginated \
     --params '{"limit": 5, "cursor_index": 4294967295}'
   ```
3. **Acceptance criteria**:
   - The call succeeds (status 0).
   - The returned payload conforms to the new data structure (e.g., records list
     with pagination cursor).
   - This definitively proves the new binary's function dispatch table is active.

#### Scenario C: Updated validation rules or error codes

When the upgrade introduces namespaced error codes (e.g., `DuplicateAsset = 13`
or `TooManyPayments = 14` in `record_payment`):

Invoke a simulation designed to trigger the boundary condition:

```bash
# Example: Simulating duplicate asset payment
stellar contract invoke \
  --simulate-only \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  record_payment \
  --amount 100000000 \
  --asset <ALREADY_RECORDED_ASSET_ID>
```

**Acceptance criteria:**
- The transaction simulation rejects with the specific newly introduced error
  code (e.g., `Error(Contract, #13)`) instead of the legacy generic error.

#### Scenario D: State-changing verification on a disposable test account

For upgrades that alter stateful write flows (such as `record_payment`,
`claim`, or initialization options):

> [!WARNING]
> Do not execute state-modifying probe transactions on shared or production-mirror
> instances. Use a dedicated disposable test account.

1. Locate or deploy a disposable `EphemeralAccount` on testnet (refer to
   [`testnet/registry/known-test-accounts.md`](../registry/known-test-accounts.md)).
2. Upgrade the disposable instance to `<NEW_WASM_HASH>`.
3. Execute the stateful write operation:
   ```bash
   stellar contract invoke \
     --id <DISPOSABLE_EA_CONTRACT_ID> \
     --network testnet \
     --source <TEST_SIGNER_IDENTITY> \
     -- \
     <STATE_CHANGING_METHOD> \
     --arg1 <VALUE1>
   ```
4. Query the instance with `get_info` to confirm that the state transition
   committed correctly under the new logic.

---

### 9. Verify state preservation & invariants

Re-query the target contract instance using the read methods executed in Step 1.
Unless the upgrade intentionally included an approved data migration, verify
that every pre-existing field is unchanged:

| Field | Pre-Upgrade Value | Post-Upgrade Value | Status |
|---|---|---|---|
| `creator` | (from Step 1) | `stellar ... get_info` | Must match exactly |
| `expiry_ledger` | (from Step 1) | `stellar ... get_info` | Must match exactly |
| `recovery_address` | (from Step 1) | `stellar ... get_info` | Must match exactly |
| `status` | (from Step 1) | `stellar ... get_status` | Must match exactly |
| Payment count & records | (from Step 1) | `stellar ... get_info` | Must match exactly |
| `swept_to` | (from Step 1) | `stellar ... get_info` | Must match exactly |
| `get_reserve_remaining` | (from Step 1) | `stellar ... get_reserve_remaining` | Must match exactly |
| `is_reserve_reclaimed` | (from Step 1) | `stellar ... is_reserve_reclaimed` | Must match exactly |

> [!CRITICAL]
> If any stored field resets to default or fails to deserialize, the new WASM
> has broken storage schema compatibility. **Halt operations immediately** and
> initiate the rollback procedure below.

---

### 10. Record the verified result (Pair with `upgrade-history.md`)

Once all steps pass (transaction finalized, code hash matches, behavior probe
confirms new logic, and state invariants hold), the upgrade is verified.

Log the upgrade in [`testnet/registry/upgrade-history.md`](../registry/upgrade-history.md):

#### 1. Append to the EphemeralAccount Upgrades table:
```markdown
| <UTC_DATE> | <EA_CONTRACT_ID> | <OLD_WASM_HASH> | <NEW_WASM_HASH> | <REASON> | <ADMIN_PUBLIC_ADDRESS> | <UPGRADE_TX_HASH> |
```

#### 2. Append to the Upgrade Event Log Template:
```markdown
| <UTC_DATE> | EphemeralAccount | <EA_CONTRACT_ID> | <OLD_WASM_HASH> | <NEW_WASM_HASH> | <REASON> | <COMMIT_SHA> | <OPERATOR_LOGIN> |
```

#### 3. Update companion registries:
- [`testnet/registry/wasm-hash-reference.md`](../registry/wasm-hash-reference.md):
  Update the current WASM hash and commit SHA for `EphemeralAccount`.
- [`testnet/registry/contract-status.md`](../registry/contract-status.md):
  Update the WASM hash column and timestamp.
- [`testnet/registry/last-verified.json`](../registry/last-verified.json):
  Update `"ephemeral_account"` to the current ISO 8601 UTC timestamp.
- [`testnet/docs/changelog.md`](../docs/changelog.md):
  Note the upgrade event under the active release section.

> **Integrator Correlation**: Integrators encountering test failures or unexpected
> contract responses on testnet should inspect `testnet/registry/upgrade-history.md`
> first. By keeping this log updated with verified entries, integrators can
> immediately distinguish an intentional contract upgrade from a regressive bug.

---

## Verification Classification

| Evidence Observed | Classification | Required Action |
|---|---|---|
| Tx succeeded, hash equals `<NEW_WASM_HASH>`, behavior probe confirms new logic, state preserved | **Verified** | Log in `upgrade-history.md` and update registries. |
| Tx succeeded, but hash remains `<OLD_WASM_HASH>` | **Failed / Unresolved** | Check transaction arguments; confirm target contract ID. Do not record as upgraded. |
| Hash matches, but behavior probe returns legacy output | **Stale Cache / Incomplete** | Query alternate RPC endpoint; check if local node has stale simulation cache. |
| Hash matches, but behavior probe fails with `HostError` | **Regression** | New code logic contains a runtime defect. Trigger rollback. |
| Hash matches, but stored state changed unexpectedly | **Compatibility Failure** | Storage layout mismatch. Trigger immediate rollback. |
| Tx failed with `NotUpgradeAdmin` | **Auth Failure** | Verify signer matches the stored admin address. |

---

## Rollback Procedure

If the upgraded contract exhibits regressions, runtime traps, or storage
incompatibility:

1. **Pause dependent traffic**: Notify team and pause automated integration
   pipelines interacting with `<EA_CONTRACT_ID>`.
2. **Preserve evidence**: Save all transaction outputs, error logs, simulation
   results, and current storage snapshots for the postmortem.
3. **Execute rollback upgrade**:
   Provided the new WASM still exposes a functioning admin `upgrade()` method,
   invoke `upgrade` pointing back to `<OLD_WASM_HASH>`:
   ```bash
   stellar contract invoke \
     --id <EA_CONTRACT_ID> \
     --network testnet \
     --source <ADMIN_IDENTITY> \
     -- \
     upgrade \
     --new_wasm_hash <OLD_WASM_HASH>
   ```
4. **Re-verify rollback**: Re-run Steps 5, 6, and 9 of this runbook to confirm
   the contract has returned to `<OLD_WASM_HASH>` and that original behavior is
   restored.
5. **Log rollback**: Record the rollback transaction in
   [`testnet/registry/upgrade-history.md`](../registry/upgrade-history.md) with
   reason `"Emergency rollback to <OLD_WASM_HASH> due to <ISSUE>"`.

---

## Related Documentation

- [`testnet/registry/upgrade-history.md`](../registry/upgrade-history.md) — Upgrade log and template
- [`testnet/registry/wasm-hash-reference.md`](../registry/wasm-hash-reference.md) — Canonical WASM hash reference
- [`testnet/registry/contract-status.md`](../registry/contract-status.md) — Testnet deployment status
- [`testnet/registry/known-test-accounts.md`](../registry/known-test-accounts.md) — Disposable test accounts
- [`testnet/registry/verify-contract-live.md`](../registry/verify-contract-live.md) — General contract verification
- [`testnet/integration/testing-against-upgraded-contracts.md`](../integration/testing-against-upgraded-contracts.md) — Test suite pinning guidance
- [`testnet/runbooks/stuck-ephemeral-account.md`](stuck-ephemeral-account.md) — Triage for stuck accounts
