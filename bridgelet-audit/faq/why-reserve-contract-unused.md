<!--
Purpose: Explain why ReserveContract exists in Bridgelet Core despite not currently being called by other contracts, summarizing the architectural disconnect, current hardcoded constants, and practical takeaways.
Owner: @ohamamarachi474-del
Status: Documentation-only. No contracts, docs, scripts, or tools changes are introduced by this file.
-->

# FAQ: Why Does `ReserveContract` Exist If Nothing Calls It?

| Field | Value |
| :--- | :--- |
| **Category** | Frequently Asked Questions (FAQ) |
| **Topic** | Contract Architecture & Reserve Configuration |
| **Owner / reviewer** | `@ohamamarachi474-del` |
| **Last reviewed** | `2026-08-26` |

---

## Quick Answer

`ReserveContract` was designed as a central, on-chain configuration module to store, update, and expose the Stellar network's base reserve parameter under authorized administrator control.

However, in the current implementation of Bridgelet Core, **no downstream contract calls `ReserveContract`**. Instead, `EphemeralAccount` initializes its reserve accounting using a compile-time constant:
```rust
const BASE_RESERVE_STROOPS: i128 = 1_000_000_000; // 100 XLM
```

**The contract itself is not buggy or defective**—its code is fully functional, thoroughly unit-tested, and secure. It is simply **not yet wired into the operational execution paths** of `EphemeralAccount` and `AccountFactory`.

**The Practical Takeaway Today:** Invoking `ReserveContract::set_base_reserve()` has **zero effect** on how ephemeral accounts track, sweep, or reclaim base reserves. To alter the base reserve used by newly deployed ephemeral accounts today, developers must modify the Rust constant in `contracts/ephemeral_account/src/lib.rs` and recompile/redeploy the contract.

---

## The Disconnect: Orphaned Contract vs. Hardcoded Constant

The system currently exhibits an architectural disconnect between two different representations of the base reserve:

```
┌───────────────────────────────────────────────────────────────────────────────┐
│                           The Base Reserve Disconnect                         │
├──────────────────────────────────────┬────────────────────────────────────────┤
│ 1. Standalone ReserveContract        │ 2. EphemeralAccount Constant           │
├──────────────────────────────────────┼────────────────────────────────────────┤
│ • File: contracts/reserve_contract/  │ • File: contracts/ephemeral_account/   │
│ • State: Stored in instance storage  │ • State: Rust const BASE_RESERVE_STROOPS│
│ • Mutability: Admin-updatable on-chain│ • Mutability: Compile-time constant    │
│ • Value: Configurable (e.g. 100 XLM) │ • Value: Fixed at 1,000,000,000 stroops│
│ • Status: Orphaned (Uncalled)        │ • Status: Active across all accounts   │
└──────────────────────────────────────┴────────────────────────────────────────┘
```

### Why Was This Architecture Built?

1. **Initial Protocol Design**: `ReserveContract` was implemented to provide a single, dynamic on-chain source of truth for the Stellar network's base reserve requirement ($0.5\text{ XLM}$ per subentry, with a $1.0\text{ XLM}$ minimum account balance). This design aimed to allow protocol administrators to adjust the system-wide reserve requirement without requiring full contract redeployments when network-wide parameters shift.
2. **Cross-Contract Decoupling During Development**: When building and testing the core `EphemeralAccount` state machine and `AccountFactory` batch deployer, `EphemeralAccount` was given a self-contained constant (`BASE_RESERVE_STROOPS`) to minimize cross-contract call complexity and reduce gas costs.
3. **Pending Cross-Contract Integration**: The cross-contract integration linking `EphemeralAccount::initialize()` or `AccountFactory::batch_initialize()` to `ReserveContract::require_base_reserve()` was deferred, leaving `ReserveContract` in an "orphaned" but complete state.

---

## Is `ReserveContract` Buggy?

**No.** `ReserveContract` is cleanly implemented and passes all verification standards:

- **Strict Access Control**: Enforces one-time admin initialization (`initialize`) and mandates `admin.require_auth()` for all updates via `set_base_reserve()`.
- **Input & Bounds Validation**: Guards against invalid amounts ($0 < \text{amount} \le \text{MAX\_RESERVE\_STROOPS}$ where ceiling is $10{,}000\text{ XLM}$ = $100{,}000{,}000{,}000\text{ stroops}$) to catch common operator errors such as unit confusion (XLM vs. stroops).
- **Event Auditability**: Emits `BaseReserveUpdated` and `ContractInitialized` events on every state transition.
- **Instance Storage & TTL Management**: Properly extends instance storage TTL on all invocations (`storage::extend_instance_ttl(&env)`).
- **Comprehensive Unit Testing**: Covered by unit tests validating authorization, amount boundaries, and query helpers (`get_base_reserve`, `require_base_reserve`, `has_base_reserve`).

The contract functions exactly as designed; it is simply not invoked by the rest of the system at runtime.

---

## Side-by-Side Comparison

| Feature / Attribute | `ReserveContract` | `EphemeralAccount` (`BASE_RESERVE_STROOPS`) |
| :--- | :--- | :--- |
| **Source Location** | [`contracts/reserve_contract/src/lib.rs`](../../contracts/reserve_contract/src/lib.rs) | [`contracts/ephemeral_account/src/lib.rs`](../../contracts/ephemeral_account/src/lib.rs) |
| **Storage Type** | On-chain contract instance storage (`DataKey::BaseReserve`) | Inlined WebAssembly compile-time constant |
| **Current Default** | `None` until configured via `set_base_reserve` | `1_000_000_000` stroops ($100\text{ XLM}$) |
| **How to Update** | Call `set_base_reserve(env, new_amount)` as admin | Edit source code, recompile WASM, and redeploy |
| **Active System Impact** | **None** (no contracts read from it) | **Full** (governs all ephemeral account reserves) |
| **Cross-Contract Calls** | None required | None required |

---

## Practical Takeaways for Operators & Integrators

1. **Do Not Expect On-Chain Config Changes to Take Effect**:
   If an operator deploys `ReserveContract` and calls `set_base_reserve(10_000_000)` (1 XLM), `EphemeralAccount` instances will **continue** to use `1_000_000_000` stroops (100 XLM) because they do not query `ReserveContract`.
2. **Manual Reconciliation (Operational Runbook)**:
   If `ReserveContract` is deployed to act as an off-chain or on-chain reference, operators must manually cross-check that the value stored in `ReserveContract` agrees with the `BASE_RESERVE_STROOPS` constant in deployed ephemeral accounts. See the [Cross-Check Runbook](../runbooks/cross-check-reserve-contract-vs-hardcoded-reserve.md).
3. **Deployment Strategy**:
   Operators deploying Bridgelet Core to new networks (e.g. testnet or mainnet) can choose whether to deploy `ReserveContract`. Omitting it does not break `AccountFactory`, `SweepController`, or `EphemeralAccount`.
4. **Future Migration Path**:
   When cross-contract integration is implemented, `EphemeralAccount::initialize()` will query `ReserveContract::require_base_reserve()` or accept a factory-injected reserve parameter, transforming `ReserveContract` into an active on-chain dependency.

---

## Deeper Technical References

- [`bridgelet-audit/runbooks/cross-check-reserve-contract-vs-hardcoded-reserve.md`](../runbooks/cross-check-reserve-contract-vs-hardcoded-reserve.md) — Operational runbook for manually verifying that `ReserveContract` aligns with the compiled constant.
- [`bridgelet-audit/threat-models/reserve-contract-config-flow.md`](../threat-models/reserve-contract-config-flow.md) — Threat model analyzing the trust assumptions and downstream impacts if `ReserveContract` is integrated.
- [`bridgelet-audit/faq/what-is-base-reserve.md`](what-is-base-reserve.md) — Overview of the Stellar base reserve and why ephemeral accounts track it separately.
- [`bridgelet-audit/glossary/reserve-reclaim.md`](../glossary/reserve-reclaim.md) — Specification of the reserve tracking variables (`BaseReserveRemaining`, `AvailableReserve`, `ReserveReclaimed`).
- [`contracts/reserve_contract/src/lib.rs`](../../contracts/reserve_contract/src/lib.rs) — Full source code for the `ReserveContract` implementation.
- [`contracts/ephemeral_account/src/lib.rs`](../../contracts/ephemeral_account/src/lib.rs) — Full source code for `EphemeralAccount` and the hardcoded `BASE_RESERVE_STROOPS` constant.
