# Runbook: Verify Expired-Account Recovery

## Purpose

Verify an expired `EphemeralAccount` transition on testnet by reconciling:

1. pre- and post-expiry contract state;
2. the `AccountExpired` event payload (`recovery_address`, `amount_returned` / `total_amount`, and `reserve_amount`);
3. reserve bookkeeping and `ReserveReclaimed` event data; and
4. actual on-chain SEP-41 token and native balance changes for every recorded asset.

An emitted event is an audit record. It is not automatically proof that funds reached
the recovery address. This runbook provides the formal verification procedure both for
**routine manual QA** and for **investigating a reported discrepancy**.

> **Operator Note:** This runbook describes verification steps. State-changing commands
> (`expire` or `recover`) are operator actions that should only be executed when testing
> or resolving an expired account.

---

## Core Invariants & Verification Principles

1. **Event vs. Balance Disconnect:** An `AccountExpired` event records the contract's intent
   and internal state change, but only an on-chain token balance query confirms that tokens
   arrived at the `recovery_address`.
2. **Current-Source Limitation:** In the checked-in Soroban implementation, `record_payment`
   records payment metadata but does not enforce inbound token custody. When `expire()` or
   `recover()` is called, the contract transitions status to `Expired` (3), computes the sum
   of recorded amounts, reclaims available reserve bookkeeping, and emits `AccountExpired`.
   It does not invoke external SEP-41 token transfer methods for recorded payment assets.
   Therefore, on current testnet builds, payment tokens may remain in the ephemeral account
   unless an external relayer/custom contract version handles the transfer.
3. **Multi-Asset Separation:** The event's aggregate amount sums raw integer amounts across
   potentially different token denominations. Always reconcile balances per asset contract ID.
4. **Reserve Distinction:** `reserve_amount` represents reclaimed contract base reserve (managed
   internally or via the reserve contract), distinct from token payment amounts.

---

## Event Schema & Field Breakdown

The Soroban `AccountExpired` event is defined in `contracts/ephemeral_account/src/events.rs`:

```rust
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountExpired {
    pub recovery_address: Address,
    pub amount_returned: i128,
    pub reserve_amount: i128,
}
```

The event is published under topic `expired`: `env.events().publish((symbol_short!("expired"),), event);`.

| Event Field | Type | Meaning & Verification Rule |
|---|---|---|
| `recovery_address` | `Address` | The fallback destination configured during account initialization (`get_info.recovery_address`). Must match post-expiry `swept_to`. |
| `amount_returned` (or `total_amount`) | `i128` | The integer sum of all recorded payment amounts. Local variable in contract is `total_amount`; serialized ABI field is `amount_returned`. Must match the sum of payments in `get_info.payments`. |
| `reserve_amount` | `i128` | The base reserve reclaimed during this expiry call. Must match the `amount` field in the concurrent `ReserveReclaimed` event. |

Concurrent `ReserveReclaimed` event (topic `reserve`):

```rust
pub struct ReserveReclaimed {
    pub destination: Address,
    pub amount: i128,
    pub sweep_id: u64,
    pub fully_reclaimed: bool,
    pub remaining_reserve: i128,
}
```

---

## Required Inputs

| Placeholder | Meaning |
|---|---|
| `<EA_CONTRACT_ID>` | Ephemeral account under test or investigation (`C...`) |
| `<RECOVERY_ADDRESS>` | Expected recovery address stored at initialization (`G...` or `C...`) |
| `<READER_IDENTITY>` | Funded testnet identity for read-only simulations |
| `<CALLER_IDENTITY>` | Creator or recovery identity if executing `recover()` |
| `<ASSET_CONTRACT_ID>` | SEP-41 token contract ID for each recorded payment |
| `<EXPIRY_LEDGER>` | Account's configured expiry ledger sequence |
| `<TX_LEDGER_BEFORE>` | Ledger immediately preceding the expiry transaction |
| `<EXPIRY_TX_HASH>` | Finalized expiry or recovery transaction hash |
| `<TX_LEDGER_AFTER>` | Ledger containing the expiry transaction |

---

## Workflow A: Step-by-Step Manual QA Verification

Use this workflow during routine testing of the expiry and recovery lifecycle on testnet.

### Step 1: Pre-Flight State & Balance Snapshot

Before triggering or verifying expiry, record initial state:

```bash
# 1. Query account info and status
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

# 2. Check current ledger vs expiry ledger
stellar contract invoke \
  --simulate-only \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  is_expired

stellar network current-ledger --rpc-url https://soroban-testnet.stellar.org

# 3. Snapshot reserve state
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
```

For every asset recorded in `get_info.payments`, snapshot the balance at both addresses:

```bash
# Ephemeral account balance
stellar contract invoke \
  --simulate-only \
  --id <ASSET_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  balance \
  --id <EA_CONTRACT_ID>

# Recovery address balance
stellar contract invoke \
  --simulate-only \
  --id <ASSET_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  balance \
  --id <RECOVERY_ADDRESS>
```

Fill out the pre-flight baseline table:

| Asset Contract | Recorded Payment | EA Balance Before | Recovery Balance Before |
|---|---:|---:|---:|
| `<ASSET_CONTRACT_ID>` | `<AMOUNT>` | `<EA_BEFORE>` | `<REC_BEFORE>` |

### Step 2: Trigger Expiry (If Testing Transition)

Once `current_ledger >= expiry_ledger` (`is_expired == true`), execute expiry:

**Path 1: Permissionless `expire`**
```bash
stellar contract invoke \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  expire
```

**Path 2: Authenticated `recover` (Creator or Recovery Address)**
```bash
stellar contract invoke \
  --id <EA_CONTRACT_ID> \
  --network testnet \
  --source <CALLER_IDENTITY> \
  -- \
  recover \
  --caller <RECOVERY_ADDRESS>
```

Record the transaction hash `<EXPIRY_TX_HASH>` and ledger sequence `<TX_LEDGER_AFTER>`.

### Step 3: Verify Post-Expiry Terminal State

```bash
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
  get_info
```

Expected assertions:
- `get_status == 3` (`AccountStatus::Expired`)
- `get_info.swept_to == Some(<RECOVERY_ADDRESS>)`
- `get_info.recovery_address == <RECOVERY_ADDRESS>` (unchanged)
- Subsequent calls to `expire()` or `recover()` fail with `InvalidStatus` (error code 3).

### Step 4: Extract and Verify Event Payload

Retrieve the events emitted during `<TX_LEDGER_AFTER>`:

```bash
stellar contract events \
  --contract-id <EA_CONTRACT_ID> \
  --start-ledger <TX_LEDGER_BEFORE> \
  --filter '{"topics":[["expired"]]}' \
  --network testnet

stellar contract events \
  --contract-id <EA_CONTRACT_ID> \
  --start-ledger <TX_LEDGER_BEFORE> \
  --filter '{"topics":[["reserve"]]}' \
  --network testnet
```

Match the event to `<EXPIRY_TX_HASH>` and verify:
1. `event.recovery_address == <RECOVERY_ADDRESS>`
2. `event.amount_returned == sum(payment.amount for payment in get_info.payments)`
3. `event.reserve_amount == reserve_event.amount`

### Step 5: Reconcile On-Chain Balances

Query post-expiry balances for each asset:

```bash
stellar contract invoke \
  --simulate-only \
  --id <ASSET_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  balance \
  --id <RECOVERY_ADDRESS>

stellar contract invoke \
  --simulate-only \
  --id <ASSET_CONTRACT_ID> \
  --network testnet \
  --source <READER_IDENTITY> \
  -- \
  balance \
  --id <EA_CONTRACT_ID>
```

Compute deltas:
```text
recovery_delta = recovery_balance_after - recovery_balance_before
ea_debit       = ea_balance_before - ea_balance_after
```

Reconciliation table:

| Asset | Expected Return | Actual Recovery Delta | Actual EA Debit | Reconciled? |
|---|---:|---:|---:|:---:|
| `<ASSET_CONTRACT_ID>` | `<RECORDED_AMOUNT>` | `<REC_DELTA>` | `<EA_DEBIT>` | `YES / NO` |

---

## Workflow B: Investigating a Reported Discrepancy

Use this workflow when an integrator reports:
- "The account expired, but tokens never arrived at the recovery address."
- "The `AccountExpired` event amounts do not match the expected refund."
- "The transaction succeeded, but the recovery address received nothing."

### Discrepancy Triage Matrix

| Reported Symptom | Root Cause | Investigative Check | Action |
|---|---|---|---|
| `expire()` fails with `NotExpired` (code 4) | Ledger has not reached `expiry_ledger` | Compare `stellar network current-ledger` with `get_info.expiry_ledger` | Wait until current ledger sequence exceeds expiry ledger; do not confuse Unix timestamps with ledger sequences. |
| `expire()` fails with `InvalidStatus` (code 3) | Account was already swept or previously expired | Check `get_status` and `get_info.swept_to` | If `status == 2` (`Swept`), funds were swept to destination. If `status == 3` (`Expired`), recovery already ran. |
| `AccountExpired` event emitted, but recovery token balance delta is `0` | Current-source limitation: contract records metadata but does not execute token transfers on expiry | Inspect `balance` of `<EA_CONTRACT_ID>` vs `<RECOVERY_ADDRESS>` | Verify if tokens remain in the ephemeral account. Reconcile whether testnet environment uses standard source or custom transfer wrapper. |
| `event.recovery_address` does not match integrator expectations | Wrong recovery address configured at `initialize()` | Query `get_info.recovery_address` and check deployment params | Address in contract storage is immutable. Funds/events reflect stored address, not caller's assumed address. |
| `amount_returned` differs from single payment expectation | Account received multiple payments across assets | Check `get_info.payments` and `get_info_paginated` | Sum recorded amounts across all payments. The event field is an aggregate integer sum. |
| `reserve_amount` is `0` | Reserve was already reclaimed or never funded | Query `get_reserve_remaining` and `is_reserve_reclaimed` | If `remaining_reserve == 0`, `reserve_amount: 0` is expected behavior. |
| Recovery address received less than total event amount | Aggregate event amount combines different decimals/tokens | Verify per-asset token balances separately | Never compare raw sum across multiple tokens directly against a single asset balance. |

### Diagnostic Decision Tree

```
[Discrepancy Reported]
        │
        ▼
Query get_status()
        ├── status == 0 or 1 (Active / PaymentReceived)
        │       └── Check is_expired():
        │             ├── false -> Not yet expired. Wait for expiry_ledger.
        │             └── true  -> Account eligible for expire(). Transaction not yet executed.
        │
        ├── status == 2 (Swept)
        │       └── Account was swept by SweepController, NOT expired.
        │             Inspect swept_to and controller sweep events.
        │
        └── status == 3 (Expired)
                │
                ▼
        Query events for contract ID around transaction ledger:
                ├── Event missing?
                │       └── Tx may have reverted or wrong contract ID queried. Re-verify tx hash.
                │
                └── Event present:
                        ├── Check recovery_address matches expected address.
                        ├── Check amount_returned == sum(payments).
                        ├── Check reserve_amount == matching ReserveReclaimed.amount.
                        └── Query SEP-41 token balances:
                                ├── Tokens at recovery address -> Recovery verified.
                                └── Tokens still at EA contract -> Current-source limitation confirmed.
```

---

## Classification of Verification Results

At the conclusion of either manual QA or discrepancy investigation, assign one of the following statuses:

| Status Code | Meaning | Criteria |
|---|---|---|
| `RECOVERY_BALANCE_VERIFIED` | Full End-to-End Success | `AccountExpired` event emitted, state is `Expired`, and on-chain SEP-41 balance delta at `recovery_address` equals recorded payment amounts. |
| `METADATA_AND_STATE_VERIFIED` | Current-Source Expected | Contract state is `Expired`, `swept_to` matches recovery address, event fields reconcile with internal payments and reserve, but no SEP-41 transfer was invoked on-chain. |
| `EVENT_FIELD_MISMATCH` | Event Inconsistency | Event `recovery_address`, `amount_returned`, or `reserve_amount` differs from stored contract values or reserve event. |
| `DESTINATION_MISMATCH` | Address Drift | Recovery address in event differs from the address intended by the integrator. |
| `UNEXPIRED_OR_STUCK` | Precondition Failure | Current ledger is below `expiry_ledger` or account status prevents transition. |
| `INSUFFICIENT_EVIDENCE` | Incomplete Audit | Missing transaction hash, unconfirmed event, or incomplete balance snapshot. |

---

## Evidence Record Template

Copy and attach this record to QA test reports or discrepancy tickets:

```text
=== EXPIRED ACCOUNT RECOVERY VERIFICATION RECORD ===
Investigation Date (UTC):    YYYY-MM-DD HH:MM:SS
Investigator Identity:       <PUBLIC_KEY_OR_HANDLE>
Classification:              <RECOVERY_BALANCE_VERIFIED | METADATA_AND_STATE_VERIFIED | ...>

CONTRACT METADATA:
- Ephemeral Account ID:      <EA_CONTRACT_ID>
- Stored Recovery Address:   <RECOVERY_ADDRESS>
- Configured Expiry Ledger:  <EXPIRY_LEDGER>
- Terminal Status:           <AccountStatus::Expired (3)>
- Post-Expiry swept_to:      <SWEPT_TO_ADDRESS>

TRANSACTION DETAILS:
- Expiry Tx Hash:            <EXPIRY_TX_HASH>
- Finalized Ledger:          <TX_LEDGER_AFTER>
- Invocation Method:         <expire | recover>
- Invoking Source:           <CALLER_PUBLIC_KEY>

EVENT AUDIT:
- Event Topic:               expired
- Event recovery_address:    <EVENT_RECOVERY_ADDRESS>
- Event amount_returned:     <EVENT_AMOUNT_RETURNED>
- Sum of Recorded Payments:  <SUM_PAYMENTS>
- Event reserve_amount:      <EVENT_RESERVE_AMOUNT>
- ReserveReclaimed amount:   <RESERVE_RECLAIMED_AMOUNT>

ON-CHAIN BALANCE RECONCILIATION:
Asset Contract ID            Recorded Amount    EA Delta        Recovery Delta    Matches?
------------------------------------------------------------------------------------------
<ASSET_1_ID>                 <AMOUNT_1>         <EA_DEBIT_1>    <REC_CREDIT_1>    <YES/NO>
<ASSET_2_ID>                 <AMOUNT_2>         <EA_DEBIT_2>    <REC_CREDIT_2>    <YES/NO>

RESERVE RECONCILIATION:
- Remaining Reserve Before:  <BEFORE_REMAINING>
- Remaining Reserve After:   <AFTER_REMAINING>
- Net Reserve Delta:         <BEFORE - AFTER>
- Matches reserve_amount?    <YES/NO>

CONCLUSION & NOTES:
<Summary of findings and next steps>
====================================================
```

---

## Escalation Path

Escalate to core maintainers if:
- `recovery_address` in the event does not match `get_info.recovery_address`;
- `amount_returned` does not equal the sum of payments in contract storage;
- `reserve_amount` differs from the `ReserveReclaimed` event or reserve storage delta;
- On-chain balances decreased from the ephemeral account without arriving at `recovery_address`;
- A deployed contract version claims to transfer tokens on expiry but balances did not change.

Attach the completed **Evidence Record** along with raw CLI outputs. Never include private keys, seed phrases, or authorization secrets.

---

## Related Documentation

- [`stuck-ephemeral-account.md`](stuck-ephemeral-account.md) — Triage for accounts that cannot sweep or expire
- [`nonce-desync.md`](nonce-desync.md) — Resolving sweep controller nonce drift
- [`../config/expiry-ledger-testing.md`](../config/expiry-ledger-testing.md) — Expiry ledger mechanics and testing patterns
- [`../registry/event-topics.md`](../registry/event-topics.md) — Canonical topic reference for Soroban events
- [`../monitoring/alert-thresholds.md`](../monitoring/alert-thresholds.md) — Monitoring ratios for `AccountExpired` vs `SweepCompleted`
