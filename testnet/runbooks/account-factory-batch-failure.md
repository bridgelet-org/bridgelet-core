# Runbook: Investigating `AccountFactory::batch_initialize` Partial Failure

## Overview

When `AccountFactory` is (or becomes) deployed on testnet,
`batch_initialize` may report that one or more accounts failed **without**
returning the underlying per-account error. This runbook is the investigative
path for that situation: given only a **failed address** (and the batch
transaction hash), diagnose *why* by checking each `initialize()` precondition
manually.

Write this procedure now so it is ready the moment factory deploys land on
testnet—even if the factory build still discards detailed per-item errors.

## Symptom

- Batch invoke returns success with a partial-failure summary, **or** the
  transaction fails after some accounts were created.
- Operator has: failed account address(es), factory contract id, batch tx hash.
- Operator does **not** have a structured per-account error code from the factory.

## Prerequisites

- Testnet identity with deploy/invoke rights for the factory path
- Factory contract id (see `testnet/registry/account-factory-status.md` when present)
- Ability to run read-only `stellar contract invoke` against candidate accounts
- Current ledger awareness (`stellar network current-ledger` or RPC
  `getLatestLedger`)

## Investigative sequence

Work **one failed address at a time**. Do not re-run the full batch until each
address is classified.

### 1. Confirm the address and deployment

```bash
FAILED_ACCOUNT=<address from batch result>
stellar contract info \
  --id "$FAILED_ACCOUNT" \
  --network testnet
```

| Observation | Likely meaning |
|---|---|
| Contract not found / no WASM | Deploy step never committed; treat as deploy failure |
| WASM present | Deploy may have succeeded; `initialize` likely failed or never ran |

### 2. Read status if the contract exists

```bash
stellar contract invoke \
  --id "$FAILED_ACCOUNT" \
  --network testnet \
  --source testnet-investigator \
  -- \
  get_status
```

| Status / result | Next check |
|---|---|
| Already non-default / Active / initialized | `AlreadyInitialized` — duplicate salt or address reuse |
| Uninitialized / error on get_status | Continue precondition checks below |
| Invoke fails with host/RPC errors | See [rpc-outage.md](rpc-outage.md) |

### 3. Check each `initialize()` precondition manually

Mirror the parameters the batch would have passed (creator, expiry ledger,
recovery address, authorized controller, admin).

1. **Caller authorization** — Was the batch submitted with the expected source?
   Re-simulate as that identity.
2. **Expiry ledger** — Must be strictly greater than the ledger at
   initialization time. Compare batch-time ledger (from Horizon/RPC for the
   factory tx) to the `expiry_ledger` in the request.
3. **Address / salt uniqueness** — Derive expected address from factory salt
   and index; confirm no earlier successful deploy owns that id.
4. **Admin / controller / recovery** — Non-zero, well-formed addresses; recovery
   not accidentally equal to a forbidden sentinel if the interface forbids it.
5. **Resource budget** — Large batches can fail mid-way; prefer
   `batch_initialize_paginated` with small pages when diagnosing.

Dry-run a single initialize when the interface allows simulation:

```bash
stellar contract invoke \
  --id "$FAILED_ACCOUNT" \
  --network testnet \
  --source testnet-investigator \
  -- \
  initialize \
  --creator <CALLER> \
  --expiry_ledger <FUTURE_LEDGER> \
  --recovery_address <RECOVERY> \
  --authorized_controller <CALLER> \
  --admin <CALLER>
```

Capture the **first** explicit error string/code from this single-account path;
that is the missing detail the batch discarded.

### 4. Classify and act

| Classification | Operator action |
|---|---|
| `AlreadyInitialized` | Skip address; use a new salt/index for retries |
| `InvalidExpiry` | Recompute expiry from *current* ledger + buffer; resubmit only failed items |
| WASM / deploy missing | Redeploy WASM or fix factory deploy config; do not only retry initialize |
| Auth / identity mismatch | Fix source account or auth entries; resubmit failed subset |
| Resource / gas | Split batch; use paginated API |
| Unknown | File incident with evidence pack below |

### 5. Safe retry policy

- Retry **only** the failed addresses, not the entire original batch, once
  successful accounts are confirmed initialized.
- Prefer paginated initialize to isolate the next failure.
- Record the new transaction hash next to the original factory tx.

## Evidence pack for escalation

1. Factory contract id and network passphrase  
2. Batch transaction hash and UTC time  
3. Failed account address(es)  
4. Request parameters (salt/index, expiry, roles)  
5. Results of `get_status` / single `initialize` simulation  
6. Ledger sequence at failure time  

## Related documentation

- `testnet/registry/account-factory-status.md` — deployment status  
- `testnet/examples/batch-account-creation-mock.md` — batch shapes  
- `testnet/config/expiry-ledger-testing.md` — expiry edge cases  
- [rpc-outage.md](rpc-outage.md) — infrastructure vs contract failure  
