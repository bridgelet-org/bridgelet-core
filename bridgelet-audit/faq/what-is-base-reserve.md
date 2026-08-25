<!--
Purpose: Explain what the Stellar base reserve is and why Bridgelet Ephemeral Accounts track it separately from user payment funds.
Owner: @chinweobtagaz
Status: Documentation-only. No contracts, docs, scripts, or tools changes are introduced by this file.
-->

# FAQ: What Is the 'Base Reserve' This System Tracks, and Why Does It Matter?

| Field | Value |
| :--- | :--- |
| **Category** | Frequently Asked Questions (FAQ) |
| **Topic** | Stellar Protocol Base Reserve & Account Accounting |
| **Owner / reviewer** | `@chinweobtagaz` |
| **Last reviewed** | `2026-08-25` |

---

## Quick Answer

On the Stellar network, every on-chain account must maintain a minimum balance of lumens (XLM) known as the **base reserve** (currently 0.5 XLM per subentry, with a minimum account balance of 2 base reserves = 1 XLM). This reserve is an anti-spam and state-bloat prevention mechanism enforced by the Stellar protocol.

When Bridgelet spins up a temporary **Ephemeral Account** to receive a cross-chain transfer or user deposit, the creator/relayer must fund that account with sufficient XLM to satisfy the network's base reserve. 

Bridgelet tracks this base reserve separately from user payment funds so that:
1. **User payments are cleanly swept** to the recipient without underpaying or overpaying them.
2. **The creator's operational XLM deposit is not lost** and can be reclaimed back to the creator or recovery address when the account's lifecycle ends.

---

## Plain-Language Stellar Network Explanation

### What is the Stellar Base Reserve?
Every account on the Stellar ledger occupies storage space on every validator node. To prevent malicious actors from spamming the network with billions of empty accounts or trustlines, the Stellar protocol mandates that accounts hold a non-spendable deposit:

- **Base Reserve Unit**: $0.5\text{ XLM}$ ($5{,}000{,}000\text{ stroops}$, where $1\text{ XLM} = 10{,}000{,}000\text{ stroops}$).
- **Minimum Account Balance**: $2 \times \text{Base Reserve} = 1.0\text{ XLM}$ ($10{,}000{,}000\text{ stroops}$).
- **Per-Subentry Reserve**: Each trustline, data entry, signer, or offer requires an additional $0.5\text{ XLM}$ reserve.

These reserved lumens cannot be transferred via normal payment operations while the account or its subentries are active. They can only be released when subentries are removed or when the account is merged / closed.

---

## Why Ephemeral Accounts Must Track Reserve Separately

In Bridgelet Core, an `EphemeralAccount` acts as a single-use, temporary bridge escrow. When an account is active, it holds two distinct categories of funds:

```
┌────────────────────────────────────────────────────────────────────────┐
│                      EphemeralAccount Balances                         │
├──────────────────────────────────┬─────────────────────────────────────┤
│      User Payment Funds          │      Operational Base Reserve       │
│  (e.g., 100 USDC, 50 XLM)        │       (e.g., 1 XLM base deposit)    │
│                                  │                                     │
│  • Deposited by bridge/payer     │  • Deposited by relayer/creator     │
│  • Belongs to end user           │  • Belongs to protocol operator     │
│  • Destination: User Wallet      │  • Destination: Relayer / Recovery  │
└──────────────────────────────────┴─────────────────────────────────────┘
```

### The Problems With Commingled Accounting:

1. **Preventing User Overpayment / Operator Loss**:
   If an account receives an inbound payment of $50\text{ XLM}$, and the contract holds a total balance of $51\text{ XLM}$ (including the $1\text{ XLM}$ creator reserve), sweeping the entire balance to the user would leak protocol operator capital ($1\text{ XLM}$) on every bridge transaction.

2. **Preventing User Underpayment**:
   If the base reserve were deducted indiscriminately from user deposit balances, users would receive less than the exact amount they bridged.

3. **Preventing Trapped / Stranded Capital**:
   Without explicit reserve accounting, the operational XLM used to create thousands of ephemeral accounts would remain stranded on-chain as dead capital after accounts are swept.

---

## How Bridgelet Solves This

The `EphemeralAccount` contract tracks reserve state on-chain via dedicated storage entries:

- **`BaseReserveRemaining`**: The total unreclaimed reserve liability still owed back to the operator.
- **`AvailableReserve`**: The portion of reserve funds currently liquid and ready for immediate transfer.
- **`ReserveReclaimed`**: Boolean flag set to `true` once the entire reserve liability has been refunded.

### Lifecycle of the Base Reserve in Bridgelet:

1. **Initialization**: When `AccountFactory` or an operator initializes an `EphemeralAccount`, the reserve liability is registered (e.g. `BASE_RESERVE_STROOPS`).
2. **Inbound Payments**: Inbound token/XLM payments are recorded in `payments` storage, strictly isolated from the reserve tracking variables.
3. **During Sweep (`execute_sweep` / `claim`)**:
   - The user's payment assets are transferred directly to the user's destination address.
   - The contract calls `reclaim_reserve_to(destination)` to atomically transfer the available base reserve back to the authorized destination/relayer.
4. **During Expiry (`expire` / `recover`)**:
   - If the account expires without being swept, both the uncollected user funds and the base reserve are returned safely to the designated `recovery_address`.
5. **Idempotent Follow-Up (`reclaim_reserve`)**:
   - If a portion of the reserve was temporarily locked by active subentries during the primary sweep, the operator can invoke `reclaim_reserve()` at a later time to collect the remaining stroops once subentries clear.

---

## Deeper Technical References

For detailed formulas, event structures, and code walkthroughs:

- [`docs/base-reserve-handling.md`](file:///c:/Users/HP/Desktop/bridgelet-core/docs/base-reserve-handling.md) (or [`base-reserve.md`](../../docs/base-reserve-handling.md)) — Complete technical architecture for Stellar base reserve handling during sweep and expiry operations.
- [`bridgelet-audit/glossary/reserve-reclaim.md`](file:///c:/Users/HP/Desktop/bridgelet-core/bridgelet-audit/glossary/reserve-reclaim.md) — Glossary breakdown of the `ReserveReclaimed` event schema, state variables, and worked partial-reclaim examples.
- [`bridgelet-audit/threat-models/reserve-contract-config-flow.md`](file:///c:/Users/HP/Desktop/bridgelet-core/bridgelet-audit/threat-models/reserve-contract-config-flow.md) — Threat model analyzing the configuration of system-wide base reserve parameters.
