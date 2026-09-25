# Runbook: Verify an EphemeralAccount Upgrade

## Purpose

Verify that an admin-authorized `EphemeralAccount::upgrade(new_wasm_hash)` call
completed, that the target instance now points to the intended WASM, and that
the upgrade did not silently change stored account state.

A successful transaction submission is not enough. The function emits no
contract event, so the upgrade transaction and the current code identity are
the primary evidence.

> This is a procedure, not a record that an upgrade occurred. Replace every
> placeholder and do not treat example commands as executed.

## Current contract behavior

The checked-in `EphemeralAccount` implementation:

1. rejects an uninitialized instance with `NotInitialized`;
2. reads the admin stored during `initialize`;
3. requires that admin's authorization;
4. calls `update_current_contract_wasm(new_wasm_hash)`; and
5. emits no upgrade event.

Consequences:

- `new_wasm_hash` is a 32-byte hash for WASM already uploaded to testnet, not
  a contract ID or local file path.
- An upgrade changes only the selected contract instance. It does not update
  other `EphemeralAccount` instances or a factory template automatically.
- The contract has no compatibility check for storage layout or interface.
  Instance storage is expected to remain available, but the runbook must prove
  that the selected state survived.
- `NotUpgradeAdmin` is returned when no admin is stored. A different signer
  normally fails Soroban authorization before the function completes.

## Required evidence

| Placeholder | Meaning |
|---|---|
| `<EA_CONTRACT_ID>` | initialized account instance being upgraded |
| `<ADMIN_IDENTITY>` | local identity for the stored admin |
| `<READER_IDENTITY>` | funded identity for read-only checks |
| `<OLD_WASM_HASH>` | current hash immediately before the change, if known |
| `<NEW_WASM_HASH>` | 64-character candidate hash already on testnet |
| `<UPGRADE_TX_HASH>` | transaction returned by the upgrade submission |
| `<UPGRADE_LEDGER>` | finalized ledger containing the transaction |
| `<CHANGED_READ_METHOD>` | release-specific read method, if available |

Never include a secret key, seed phrase, auth signature, or environment file
in this runbook or the incident record.

## Stop conditions

Do not submit `upgrade()` until:

- the target contract ID and network are verified;
- `get_info` proves the instance is initialized;
- the intended admin address is confirmed from initialization/deployment
  evidence;
- the candidate WASM is available on testnet;
- the current and candidate interfaces have been reviewed;
- a disposable or explicitly approved test instance is selected; and
- the deployment/upgrade registry does not still contain unresolved deployment
  placeholders.

An in-place upgrade is not an endpoint or documentation fix. Do not combine it
with an incident workaround unless the upgrade is separately approved.

## 1. Capture pre-upgrade state

Use read-only simulations where the installed CLI supports them:

```bash
stellar contract invoke \
  --simulate-only \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  get_info

stellar contract invoke \
  --simulate-only \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  get_status

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
  get_reserve_available

stellar contract invoke \
  --simulate-only \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  is_reserve_reclaimed
```

Retain the complete `get_info` result, including creator, status,
`expiry_ledger`, `recovery_address`, payments, and `swept_to`. Record the
current ledger and UTC time.

## 2. Record the current code identity

On CLI versions that provide the command:

```bash
stellar contract info hash \
  --contract-id <EA_CONTRACT_ID> \
  --network testnet
```

Record the normalized result as `<OLD_WASM_HASH>`. If the installed CLI does
not support `contract info hash`, use its contract-instance ledger lookup or
the Stellar SDK/RPC contract executable lookup. Do not guess a ledger key or
treat a local file hash as the deployed hash.

Also capture the deployed interface when supported:

```bash
stellar contract info interface \
  --contract-id <EA_CONTRACT_ID> \
  --network testnet
```

Compare the interface with the approved release. An interface change is
expected only if the release intentionally includes one.

## 3. Verify the candidate

- Confirm `<NEW_WASM_HASH>` is exactly 32 bytes/64 hexadecimal characters.
- Confirm the WASM is installed on the same testnet network.
- Record the release identifier and intended behavior change.
- Compare the candidate interface and error-code scheme.
- Keep the previous known-good hash available for a possible rollback.

## 4. Submit the upgrade

This is the only state-changing step in the runbook:

```bash
stellar contract invoke \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <ADMIN_IDENTITY> \
  -- \
  upgrade \
  --new_wasm_hash <NEW_WASM_HASH>
```

Record:

- returned transaction hash: `<UPGRADE_TX_HASH>`;
- submission and finalization time;
- final success/failure result;
- finalized ledger: `<UPGRADE_LEDGER>`; and
- complete diagnostic events on failure.

Do not retry blindly. A failed transaction did not complete the upgrade.

## 5. Verify transaction finality

Query the exact transaction hash through the submitter or Soroban RPC:

```bash
curl -fsS "$STELLAR_SOROBAN_RPC_URL" \
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

Require a successful final transaction result. A pending or unknown result is
not a completed upgrade.

## 6. Verify the current code pointer

After finalization, repeat the supported code-hash query:

```bash
stellar contract info hash \
  --contract-id <EA_CONTRACT_ID> \
  --network testnet
```

The result must equal `<NEW_WASM_HASH>` exactly.

| Observation | Result |
|---|---|
| Transaction succeeded and hash equals candidate | Continue to state/probe checks |
| Hash still equals old value | Upgrade failed or unresolved |
| Hash differs from both values | Configuration or integrity failure |
| Transaction failed | Upgrade did not complete |

## 7. Verify the new interface

Inspect the deployed interface again and compare it with the release manifest.
Check that expected public methods are present and removed methods are not
silently retained by an older build.

Interface equality is not behavioral proof, but an interface mismatch is a
release failure even if the code-pointer query passed.

## 8. Run a behavior probe

Code-hash equality proves the intended code is active. Add a behavior probe
when the release changes an observable result.

Prefer a read-only method, for example:

```bash
stellar contract invoke \
  --simulate-only \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  simulate_sweep \
  --destination <PROBE_DESTINATION>
```

Or use the release-specific method:

```bash
stellar contract invoke \
  --simulate-only \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  <CHANGED_READ_METHOD>
```

Record the expected value from the approved release notes or test plan and
compare it with the post-upgrade result. If changed behavior is state-changing,
use a separately approved disposable instance rather than a live-value
account.

## 9. Verify state preservation

Repeat the pre-upgrade reads. Unless the release intentionally changes a
field, these values must remain the same:

- creator;
- `expiry_ledger`;
- `recovery_address`;
- payment count and payment records;
- lifecycle status;
- `swept_to`;
- reserve remaining and available;
- reserve-reclaimed flag; and
- last reserve event/count when exposed.

A state change without an intentional migration is a compatibility failure.
Do not hide it by repeating the upgrade.

## 10. Record the verified result

After all checks pass, append a row to the `EphemeralAccount Upgrades` table
in [`../registry/upgrade-history.md`](../registry/upgrade-history.md):

```markdown
| <UTC_DATE> | <EA_CONTRACT_ID> | <OLD_WASM_HASH> | <NEW_WASM_HASH> | <REASON> | <ADMIN_PUBLIC_ADDRESS> | <UPGRADE_TX_HASH> |
```

Also update, when applicable:

- [`../registry/wasm-hash-reference.md`](../registry/wasm-hash-reference.md)
- [`../registry/contract-status.md`](../registry/contract-status.md)
- [`../registry/last-verified.json`](../registry/last-verified.json)
- [`../docs/changelog.md`](../docs/changelog.md)

If the old hash cannot be established, record `UNKNOWN` and explain the
evidence gap. Do not fill a placeholder with a guessed value.

## Classification

| Evidence | Classification |
|---|---|
| Successful transaction, candidate hash, expected behavior, preserved state | **Verified** |
| Successful transaction but old/unknown/different hash | **Failed or unresolved** |
| Candidate hash active but release behavior probe fails | **Regression** |
| Behavior passes but stored state changed unexpectedly | **Compatibility failure** |
| No upgrade event | Expected; not a failure |

## Rollback

If the new code is incompatible:

1. pause dependent traffic;
2. preserve the transaction, hash, interface, state snapshots, and probe output;
3. keep the known-good hash available;
4. consider `upgrade(<OLD_WASM_HASH>)` only after incident approval and only if
   the new build still exposes a functioning admin-gated `upgrade()` method;
5. repeat every check in this runbook after rollback; and
6. record the rollback as a separate upgrade-history entry.

Do not assume an in-contract rollback path exists if the new code removed or
broke `upgrade()`.

## Related documents

- [`../registry/upgrade-history.md`](../registry/upgrade-history.md)
- [`../registry/wasm-hash-reference.md`](../registry/wasm-hash-reference.md)
- [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md)
- [`../registry/admin-addresses.md`](../registry/admin-addresses.md)
- [`../monitoring/README.md`](../monitoring/README.md)
