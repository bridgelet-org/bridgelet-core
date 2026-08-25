<!--
Purpose: Explain why error code numbers overlap between contracts in Bridgelet Core and provide best practices for resolving them.
Owner: @chinweobtagaz
Status: Documentation-only. No contracts, docs, scripts, or tools changes are introduced by this file.
-->

# FAQ: Why Do Error Code Numbers Overlap Between Contracts?

| Field | Value |
| :--- | :--- |
| **Category** | Frequently Asked Questions (FAQ) |
| **Topic** | Error Enums & Discriminant Namespaces |
| **Owner / reviewer** | `@chinweobtagaz` |
| **Last reviewed** | `2026-08-25` |

---

## Quick Answer

In Soroban, each compiled smart contract is an isolated WebAssembly (WASM) module with its own independent error namespace defined via the `#[contracterror]` macro. By default, Soroban error enums are independently numbered starting near `1` (or within their own numeric discriminant ranges). 

Because numeric error codes are scoped to the contract instance that emitted them, **the same numeric integer (e.g., `1`, `2`, `1000`, `2001`) represents completely different failure conditions depending on which contract failed**. 

**Practical Takeaway:** Whenever you capture, log, or display an on-chain contract error, you **must always pair the numeric error code with its originating contract address / ID** `(contract_id, error_code)`. Never interpret a raw error number in isolation.

---

## Why Error Codes Are Scoped Per-Contract

### 1. Independent WASM Modules in Soroban
Soroban contracts do not share a global runtime error registry. When a contract function fails with a `#[contracterror]` enum, the Soroban host environment packages the error as an `Error(Contract, #code)` value where `#code` is a 32-bit unsigned integer (`u32`).

Each contract in `bridgelet-core` defines its own domain-specific `Error` enum:
- `EphemeralAccount`: Lifecycle and escrow errors (e.g. initialization, payment state, sweep validity, expiration).
- `SweepController`: Authorization, signature verification, destination lock, and token transfer routing errors.
- `AccountFactory`: Batch deployment and factory initialization errors.
- `ReserveContract`: Base reserve configuration, admin access, and amount ceiling errors.
- `SharedError`: Common failure variants across contracts.

Because each contract's Rust source compiles into a separate WASM artifact, each contract's error enum assigns discriminants independently.

### 2. Examples of Numeric Overlap

Consider how numeric error codes can overlap across contracts:

| Error Code (`u32`) | `AccountFactory` Variant | `SharedError` Variant | `EphemeralAccount` Variant | `SweepController` Variant | `ReserveContract` Variant |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **`1`** | `AlreadyInitialized` | `NotInitialized` | *(1-based legacy / unnamespaced)* | — | — |
| **`2`** | `NotInitialized` | `AlreadyInitialized` | — | — | — |
| **`3`** | — | `Unauthorized` | — | — | — |
| **`1000`** | — | — | `AlreadyInitialized` | — | — |
| **`2000`** | — | — | — | `InvalidAccount` | — |
| **`3000`** | — | — | — | — | `InvalidAmount` |

If a transaction reverts with raw error `#1`:
- If thrown by `AccountFactory`, it indicates `AlreadyInitialized` (double-initialization guard).
- If thrown by a helper returning `SharedError`, it indicates `NotInitialized`.
- If thrown by an external SEP-41 token contract, it may represent a standard token error (e.g. `InsufficientAllowance` or `BalanceMismatch`).

Without knowing the **contract ID**, a numeric code alone is ambiguous and cannot be reliably diagnosed.

---

## Practical Takeaways for Developers & Integrators

### 1. Always Pair `(contract_id, error_code)`
When building off-chain services, monitoring dashboards, indexing pipelines, or user interfaces:
- Never log bare numbers like `"Transaction failed with error 1"`.
- Always log the full context: `"Contract CADDR... failed with Error 1 (AlreadyInitialized)"`.
- Map the numeric error code against the ABI / client bindings of the specific contract identified by `contract_id`.

### 2. Use Generated Soroban SDK Client Bindings
The official Soroban TypeScript/Rust SDK bindings parse the contract's XDR error definitions automatically when invocations are made through the typed contract client. Using generated clients ensures errors are deserialized to typed enum variants (e.g., `SweepControllerError::InvalidSignature`) rather than opaque numeric codes.

### 3. Cross-Contract Invocation Inspection
When a top-level contract (such as `SweepController`) calls a sub-contract (such as `EphemeralAccount` or a token contract), Soroban propagates the sub-contract's error upward. Inspect the transaction execution trace and diagnostic events to determine which contract frame triggered the error.

---

## Related Documents & References

- [`error-enum-discriminant-overlap.md`](../glossary/error-enum-discriminant-overlap.md) / [Error Codes Namespace Reference](file:///c:/Users/HP/Desktop/bridgelet-core/contracts/shared/src/errors.rs) — Full technical breakdown of error discriminant allocations and namespace ranges across the workspace.
- [`contracts/ephemeral_account/src/errors.rs`](file:///c:/Users/HP/Desktop/bridgelet-core/contracts/ephemeral_account/src/errors.rs) — `EphemeralAccount` error enum definition.
- [`contracts/sweep_controller/src/errors.rs`](file:///c:/Users/HP/Desktop/bridgelet-core/contracts/sweep_controller/src/errors.rs) — `SweepController` error enum definition.
- [`contracts/account_factory/src/errors.rs`](file:///c:/Users/HP/Desktop/bridgelet-core/contracts/account_factory/src/errors.rs) — `AccountFactory` error enum definition.
- [`contracts/reserve_contract/src/errors.rs`](file:///c:/Users/HP/Desktop/bridgelet-core/contracts/reserve_contract/src/errors.rs) — `ReserveContract` error enum definition.
- [`contracts/shared/src/errors.rs`](file:///c:/Users/HP/Desktop/bridgelet-core/contracts/shared/src/errors.rs) — `SharedError` common error definitions and unit tests.
