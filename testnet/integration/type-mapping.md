# Soroban Contract Type Mapping Guide

## Overview

Maps the types used by Bridgelet's public contract API to typical representations in client SDKs. The signatures and field names come from `docs/api-reference.md`; that document is the source of truth for the contract interface and is not edited here.

> **Deployment status.** `testnet/registry/contract-status.md` records all four contracts as undeployed with IDs `TBD`. This is a reference for writing wrappers, not a claim that anything is live.

The JS/TS and Python columns describe values passed to generated or high-level SDK clients. When you build raw JSON-RPC/XDR yourself you will see `ScVal` / `xdr.ScVal` wrappers instead; the underlying value types are the same.

---

## Network settings

Use these exactly. The passphrase includes spaces around the semicolon; an older revision of this guide incorrectly used `November 2015` instead of `September 2015`.

| Setting | Value |
|---|---|
| Network passphrase | `Test SDF Network ; September 2015` |
| Soroban RPC | `https://soroban-testnet.stellar.org` |
| Horizon | `https://horizon-testnet.stellar.org` |

See `testnet/docs/network-config.md` for the full configuration and `testnet/integration/sdk-checklist.md` for the wrapper-level requirements.

---

## Type mapping

| Soroban / Rust contract type | JavaScript / TypeScript (`@stellar/stellar-sdk`) | Python (`stellar-sdk`) | Go | Rust SDK (`soroban-sdk`) |
|---|---|---|---|---|
| `Address` | `string` (`G...` or `C...`) | `str` | `string` (`xdr.ScAddress` for raw XDR) | `soroban_sdk::Address` |
| `BytesN<32>` | `Uint8Array` / `Buffer`, or hex `string` | `bytes` (or hex `str`) | `[32]byte` / `[]byte` | `soroban_sdk::BytesN<32>` |
| `BytesN<64>` | `Uint8Array` / `Buffer`, or hex `string` | `bytes` (or hex `str`) | `[64]byte` / `[]byte` | `soroban_sdk::BytesN<64>` |
| `i128` | `bigint` | `int` | `*big.Int` | `i128` |
| `u32` | `number` | `int` | `uint32` | `u32` |
| `u64` | `bigint` | `int` | `uint64` | `u64` |
| `Option<T>` | `T`, `null`, or `undefined` | `T` or `None` | `*T` / `nil` | `Option<T>` |
| `Vec<T>` | `T[]` | `list[T]` | `[]T` | `soroban_sdk::Vec<T>` |
| Struct / `contracttype` | Object with the ABI field names | `dict` or dataclass | struct with XDR field tags | `#[contracttype]` struct |
| Enum / discriminant | `number` or symbol string | `int` or `enum.IntEnum` | `uint32` constant | enum with explicit `#[repr(u32)]` where defined |

Examples of the last two rows in this API: `AccountStatus` (`Active = 0`, `PaymentReceived = 1`, `Swept = 2`, `Expired = 3`), `AccountInfo`, `Payment`, `PaginationParams`, and `DestinationUpdated { old_destination: Option<Address>, new_destination: Address }`.

---

## Marshalling caveats

### `bigint` vs `number`

`i128` and `u64` must not travel through a JavaScript `number`. Amounts are in base units (stroops for XLM); a `u64` above `2^53 - 1` cannot be represented exactly as a double.

```typescript
const amount = 100_000_000n;
const nonce = 42n;
const expiryLedger = 123456; // u32 is safe as a number
```

`JSON.stringify` throws on a `BigInt`. Convert to a decimal string at a JSON boundary and parse back to `bigint` in the SDK layer. In Go, use `*big.Int` and check its sign; in Python, `int` is arbitrary precision but still needs explicit base-unit conversion.

### Hex vs base64 vs raw bytes

`BytesN<32>` is an Ed25519 public key; `BytesN<64>` is a signature. They are fixed-size raw byte arrays — 32 and 64 bytes respectively — and the container type controls how they are rendered:

| Form | `BytesN<32>` | `BytesN<64>` |
|---|---|---|
| Raw bytes | 32 bytes | 64 bytes |
| Hex string | 64 hex characters | 128 hex characters |

Do not add a `0x` prefix unless the specific SDK constructor expects one, and do not base64-encode a hex string. On the wire, XDR binary fields are base64; SDK helpers perform that conversion. Double-encoding is a common cause of signatures that never verify.

### XDR `contracttype` structs

A struct payload is encoded on the wire as an `ScVal` map from symbol field names to `ScVal` values. Generated clients hide this; raw callers must construct it themselves. Field names must match the ABI exactly — `expiry_ledger`, not `expiryLedger`, and `ephemeral_account`, not `ephemeralAccount`. Unknown fields added by a future WASM must not be silently dropped by your decoder. See `testnet/tooling/event-tail-spec.md` for the same rule applied to event decoding.

### Unsigned vs signed

- `i128` is signed. Contract callers should still reject non-positive `amount` values (`InvalidAmount`, code `4`), but do not reject the type itself as negative-capable.
- `u32` and `u64` are unsigned. Reject negative inputs client-side before encoding; an attempted negative `u64` is a marshalling error, not a contract error.
- `expiry_ledger` is `u32`; controller `nonce` and event `timestamp` / `sweep_id` are `u64`. Amounts and stroop values are `i128`.

### `Option` and null

`authorized_destination: Option<Address>` is the important case: `None` means flexible mode and `Some(addr)` means locked mode. Represent absence as `null` / `None` / `nil` as appropriate to the language. An empty string, a zero address, or an omitted field is **not** the same as `None`, and `get_last_reserve_event(): Option<ReserveReclaimed>` uses the same distinction. See `testnet/config/nonce-tracking.md` for the locked-destination failure mode.

### Enum discriminants and error codes

Discriminants are contract-scoped, not global. Do not decode a bare integer without knowing which contract was invoked.

- `AccountStatus` values are `0`–`3` as listed above.
- Error codes in `docs/api-reference.md` are per contract: code `2` is `NotInitialized` on `EphemeralAccount` and `TransferFailed` on `SweepController`.
- `SweepController` skips code `12` (`InvalidNonce = 11`, `UnauthorizedDestination = 13`). An index-based `from_code` will silently mis-map it.
- The `1000`–`2999` namespaced scheme described in `testnet/docs/changelog.md` is not implemented in the current source.

Use an explicit `match` on literal discriminants and leave unknown codes unmapped. Full coverage is in `testnet/examples/error-handling.md`.

---

## SDK Integration Checklist

- [ ] Configure the passphrase exactly as `Test SDF Network ; September 2015`, with `https://soroban-testnet.stellar.org` for contracts and `https://horizon-testnet.stellar.org` for classic data.
- [ ] Resolve every contract ID from configuration and re-verify it before a run; do not hard-code an ID from an older artifact. `testnet/registry/contract-status.md` currently records all four as undeployed.
- [ ] Use `bigint` for `i128` and `u64` in JS/TS, and decimal strings at JSON boundaries; keep ledger `u32` values as numbers.
- [ ] Encode `BytesN<32>` / `BytesN<64>` as 32 / 64 raw bytes (or the exact hex form the SDK expects), with no double base64/hex conversion.
- [ ] Build and decode `contracttype` structs with exact ABI field names; surface unknown fields rather than dropping them.
- [ ] Represent `Option<T>` as `null` / `None` / `nil`; never substitute an empty string or zero address for `None`.
- [ ] Decode enum discriminants per contract with an explicit match, preserving the contract identity and leaving unknown codes unmapped.
- [ ] Simulate before every state-changing submission (`simulateTransaction` or `--send no`); see `testnet/integration/simulate-before-submit.md`.
- [ ] Read the controller nonce immediately before signing, serialize per controller, and bound retries; see `testnet/config/nonce-tracking.md`.
- [ ] Route sweeps through `SweepController::execute_sweep()` or `claim()`, and surface the stub warning in `testnet/security/unverified-signature-path-warning.md` wherever `sweep()` is exposed.
- [ ] Support local event fixtures and idempotent webhook handling per `testnet/integration/webhook-simulation.md`.
- [ ] Map errors contract-by-contract and distinguish traps and auth failures from `#[contracterror]` variants.
- [ ] Never log secret keys, signing seeds, or full signature material; see `testnet/config/README.md`.

---

## Related Documentation

- `docs/api-reference.md` — authoritative signatures, structs, and error tables
- `testnet/docs/network-config.md` — passphrase, endpoints, and configuration
- `testnet/integration/simulate-before-submit.md` — pre-flight simulation workflow
- `testnet/integration/webhook-simulation.md` — local event fixtures for consumers
- `testnet/integration/sdk-checklist.md` — full wrapper acceptance checklist
- `testnet/examples/error-handling.md` — error discriminants, reachability, and traps
- `testnet/config/nonce-tracking.md` — nonce semantics and locked-destination pitfalls
- `testnet/registry/contract-status.md` — canonical deployment status
- `testnet/tooling/event-tail-spec.md` — XDR event decoding rules

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-24 | Initial type mapping for JS/TS, Rust, and Go |
| 2.0 | 2026-11-18 | Added Python, `Address`, `BytesN<32>`, `i128`, `u32`, `Option`, `Vec`, structs and enums; corrected network passphrase to `Test SDF Network ; September 2015`; made checklist actionable |
