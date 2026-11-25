# Testnet Explorer Links

## Overview

This document curates pre-scoped, ready-made block explorer links for Bridgelet's deployed testnet contracts. Anyone investigating testnet activity, debugging transactions, or inspecting contract events has **one-click access** to human-readable history without constructing URLs by hand.

The contract IDs below reflect the canonical deployments recorded in `deployments/testnet.json` and `deployment-artifacts/contract-ids.txt`.

> **Note on Network Resets:** Stellar testnet is periodically reset by the Stellar Development Foundation (SDF), which clears ledger state and invalidates deployed contract IDs. Always verify that contracts are active using [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md).

---

## One-Click Contract Explorer Links

| Contract | Deployed Contract ID | Stellar Expert (Overview) | Stellar Expert (Transactions) | Stellar Expert (Events) | Lumenscan |
|---|---|---|---|---|---|
| **`SweepController`** | `CAJB4RRM5JRPXLITC5JUA7Q3LU2WQJDRPXVKMORXWHJJBPZRYWDJ5V4S` | [View Contract](https://stellar.expert/explorer/testnet/contract/CAJB4RRM5JRPXLITC5JUA7Q3LU2WQJDRPXVKMORXWHJJBPZRYWDJ5V4S) | [Transactions](https://stellar.expert/explorer/testnet/contract/CAJB4RRM5JRPXLITC5JUA7Q3LU2WQJDRPXVKMORXWHJJBPZRYWDJ5V4S?tab=txs) | [Events](https://stellar.expert/explorer/testnet/contract/CAJB4RRM5JRPXLITC5JUA7Q3LU2WQJDRPXVKMORXWHJJBPZRYWDJ5V4S?tab=events) | [Lumenscan](https://testnet.lumenscan.io/account/CAJB4RRM5JRPXLITC5JUA7Q3LU2WQJDRPXVKMORXWHJJBPZRYWDJ5V4S) |
| **`EphemeralAccount`** | `CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3` | [View Contract](https://stellar.expert/explorer/testnet/contract/CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3) | [Transactions](https://stellar.expert/explorer/testnet/contract/CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3?tab=txs) | [Events](https://stellar.expert/explorer/testnet/contract/CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3?tab=events) | [Lumenscan](https://testnet.lumenscan.io/account/CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3) |
| **`ReserveContract`** | `CBG3Y4GWFTL4D37JGV2XWAWC7EUQ7XGH4XM3SFLQYYSKPB2IYMAS54HY` | [View Contract](https://stellar.expert/explorer/testnet/contract/CBG3Y4GWFTL4D37JGV2XWAWC7EUQ7XGH4XM3SFLQYYSKPB2IYMAS54HY) | [Transactions](https://stellar.expert/explorer/testnet/contract/CBG3Y4GWFTL4D37JGV2XWAWC7EUQ7XGH4XM3SFLQYYSKPB2IYMAS54HY?tab=txs) | [Events](https://stellar.expert/explorer/testnet/contract/CBG3Y4GWFTL4D37JGV2XWAWC7EUQ7XGH4XM3SFLQYYSKPB2IYMAS54HY?tab=events) | [Lumenscan](https://testnet.lumenscan.io/account/CBG3Y4GWFTL4D37JGV2XWAWC7EUQ7XGH4XM3SFLQYYSKPB2IYMAS54HY) |
| **`AccountFactory`** | `CDBY4L3NTLW4OBZZTY73VL3NNWO2AGFFGBRJVDL3IMKDDLFF2W57WK6H` | [View Contract](https://stellar.expert/explorer/testnet/contract/CDBY4L3NTLW4OBZZTY73VL3NNWO2AGFFGBRJVDL3IMKDDLFF2W57WK6H) | [Transactions](https://stellar.expert/explorer/testnet/contract/CDBY4L3NTLW4OBZZTY73VL3NNWO2AGFFGBRJVDL3IMKDDLFF2W57WK6H?tab=txs) | [Events](https://stellar.expert/explorer/testnet/contract/CDBY4L3NTLW4OBZZTY73VL3NNWO2AGFFGBRJVDL3IMKDDLFF2W57WK6H?tab=events) | [Lumenscan](https://testnet.lumenscan.io/account/CDBY4L3NTLW4OBZZTY73VL3NNWO2AGFFGBRJVDL3IMKDDLFF2W57WK6H) |

---

## Detailed Contract Breakdowns

### 1. `SweepController`
- **Contract ID:** `CAJB4RRM5JRPXLITC5JUA7Q3LU2WQJDRPXVKMORXWHJJBPZRYWDJ5V4S`
- **Role:** Central gateway for verifying cryptographic signatures and sweeping ephemeral accounts.
- **Key Event Topics to Filter On:**
  - `sweep` — Emitted upon successful execution of `execute_sweep()` or `claim()`.
  - `dest_auth` — Emitted when a destination is authorized.
  - `dest_upd` — Emitted when an authorized destination is modified.
- **Copy-Ready URLs:**
  ```text
  https://stellar.expert/explorer/testnet/contract/CAJB4RRM5JRPXLITC5JUA7Q3LU2WQJDRPXVKMORXWHJJBPZRYWDJ5V4S
  https://stellar.expert/explorer/testnet/contract/CAJB4RRM5JRPXLITC5JUA7Q3LU2WQJDRPXVKMORXWHJJBPZRYWDJ5V4S?tab=txs
  https://stellar.expert/explorer/testnet/contract/CAJB4RRM5JRPXLITC5JUA7Q3LU2WQJDRPXVKMORXWHJJBPZRYWDJ5V4S?tab=events
  https://testnet.lumenscan.io/account/CAJB4RRM5JRPXLITC5JUA7Q3LU2WQJDRPXVKMORXWHJJBPZRYWDJ5V4S
  ```

### 2. `EphemeralAccount` (Reference Instance / Code Template)
- **Contract ID:** `CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3`
- **Role:** Base deployed template / reference instance for ephemeral accounts.
- **Key Event Topics to Filter On:**
  - `created` — Emitted when an ephemeral account is initialized with an admin and expiry ledger.
  - `payment` — Emitted when a single payment is recorded.
  - `multi_pay` — Emitted when multi-asset payments are received.
  - `swept_mul` — Emitted when all recorded asset balances are swept.
  - `expired` — Emitted when the account expires and funds fall back to the recovery address.
  - `reserve` — Emitted when contract base reserve is reclaimed.
- **Copy-Ready URLs:**
  ```text
  https://stellar.expert/explorer/testnet/contract/CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3
  https://stellar.expert/explorer/testnet/contract/CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3?tab=txs
  https://stellar.expert/explorer/testnet/contract/CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3?tab=events
  https://testnet.lumenscan.io/account/CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3
  ```

### 3. `ReserveContract`
- **Contract ID:** `CBG3Y4GWFTL4D37JGV2XWAWC7EUQ7XGH4XM3SFLQYYSKPB2IYMAS54HY`
- **Role:** Dedicated reserve management and rent-exemption handling (standby status on current testnet).
- **Key Event Topics to Filter On:**
  - `init` — Contract initialization.
  - `reserve` — Reserve allocation and reclaim events.
- **Copy-Ready URLs:**
  ```text
  https://stellar.expert/explorer/testnet/contract/CBG3Y4GWFTL4D37JGV2XWAWC7EUQ7XGH4XM3SFLQYYSKPB2IYMAS54HY
  https://stellar.expert/explorer/testnet/contract/CBG3Y4GWFTL4D37JGV2XWAWC7EUQ7XGH4XM3SFLQYYSKPB2IYMAS54HY?tab=txs
  https://testnet.lumenscan.io/account/CBG3Y4GWFTL4D37JGV2XWAWC7EUQ7XGH4XM3SFLQYYSKPB2IYMAS54HY
  ```

### 4. `AccountFactory`
- **Contract ID:** `CDBY4L3NTLW4OBZZTY73VL3NNWO2AGFFGBRJVDL3IMKDDLFF2W57WK6H`
- **Role:** Factory contract for deterministic batch deployment of ephemeral accounts.
- **Key Event Topics to Filter On:**
  - Individual child instances emit `created` events upon initialization.
- **Copy-Ready URLs:**
  ```text
  https://stellar.expert/explorer/testnet/contract/CDBY4L3NTLW4OBZZTY73VL3NNWO2AGFFGBRJVDL3IMKDDLFF2W57WK6H
  https://stellar.expert/explorer/testnet/contract/CDBY4L3NTLW4OBZZTY73VL3NNWO2AGFFGBRJVDL3IMKDDLFF2W57WK6H?tab=txs
  https://testnet.lumenscan.io/account/CDBY4L3NTLW4OBZZTY73VL3NNWO2AGFFGBRJVDL3IMKDDLFF2W57WK6H
  ```

---

## Dynamic Link Construction Guide

During live testing, new ephemeral child accounts and transaction hashes are created dynamically. Use the templates below to construct direct explorer links:

### Child `EphemeralAccount` Instances
When an ephemeral account is spawned (either directly or via `AccountFactory`), locate its address (`C...`) and use:

```text
Stellar Expert:
https://stellar.expert/explorer/testnet/contract/<EPHEMERAL_ACCOUNT_CONTRACT_ID>

Lumenscan:
https://testnet.lumenscan.io/account/<EPHEMERAL_ACCOUNT_CONTRACT_ID>
```

### Specific Transaction Hashes
When inspecting a submitted transaction result (e.g. from `execute_sweep`, `expire`, or `record_payment`):

```text
Stellar Expert:
https://stellar.expert/explorer/testnet/tx/<TRANSACTION_HASH>

Lumenscan:
https://testnet.lumenscan.io/tx/<TRANSACTION_HASH>
```

### Account / Wallet Addresses
For inspecting classic Stellar account balances (creator, relayer, or recovery address `G...`):

```text
Stellar Expert:
https://stellar.expert/explorer/testnet/account/<PUBLIC_G_ADDRESS>

Lumenscan:
https://testnet.lumenscan.io/account/<PUBLIC_G_ADDRESS>
```

---

## Stellar Testnet Infrastructure Endpoints

| Service | Public Endpoint URL | Purpose |
|---|---|---|
| **Stellar Expert Testnet** | <https://stellar.expert/explorer/testnet> | Primary Soroban-aware block explorer |
| **Lumenscan Testnet** | <https://testnet.lumenscan.io/> | Secondary Stellar account and transaction viewer |
| **Soroban RPC** | `https://soroban-testnet.stellar.org` | JSON-RPC for contract simulation and submission |
| **Horizon API** | `https://horizon-testnet.stellar.org/` | Classic ledger, payment, and account history API |
| **Friendbot** | `https://friendbot.stellar.org/` | Automated testnet XLM faucet |
| **Network Passphrase** | `Test SDF Network ; September 2015` | Required cryptographic domain separator |

---

## Verification & Troubleshooting

Before relying on an explorer page during an active incident:

1. **Verify Explorer Freshness:** Stellar Expert indexes Soroban transactions within 5–15 seconds of ledger close. If a transaction hash is not found immediately after submission, wait 2–3 ledgers (~10–15s) and refresh.
2. **Confirm Network Selector:** Ensure the explorer is set to **Testnet** (not Public/Mainnet). URLs must contain `/testnet/`.
3. **Check Contract WASM & Storage:**
   - In Stellar Expert, inspect the **Code** tab to confirm the uploaded WASM bytecode hash matches `deployments/testnet.json`.
   - Inspect the **Storage** tab to view persistent contract data (e.g., current controller nonce, initialized status).
4. **Compare RPC vs. Explorer:** If an explorer appears stale or unresponsive, verify directly against Soroban RPC:
   ```bash
   stellar contract invoke --simulate-only --id <CONTRACT_ID> --network testnet --source <SOURCE> -- get_info
   ```

---

## Related Documentation

- [`../registry/contract-status.md`](../registry/contract-status.md) — Canonical status and deployment dates
- [`../registry/event-topics.md`](../registry/event-topics.md) — Comprehensive Soroban event topic catalog
- [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md) — Read-only verification procedure for live contracts
- [`../runbooks/contract-not-found-error.md`](../runbooks/contract-not-found-error.md) — Diagnosing unindexed or missing contract errors
