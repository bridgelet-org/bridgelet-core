# Soroban Contract Type Mapping Guide

Maps native Soroban contract data types to language-specific SDK representations.

| Soroban Type | JavaScript / TypeScript | Rust SDK | Go SDK |
| :--- | :--- | :--- | :--- |
| `Address` | `string` (G... / C...) | `soroban_sdk::Address` | `string` |
| `BytesN<64>` | `Buffer` / `Uint8Array` | `BytesN<64>` | `[]byte` |
| `i128` | `bigint` / `BigNumber` | `i128` | `*big.Int` |
| `u64` | `bigint` / `number` | `u64` | `uint64` |

## SDK Integration Checklist

- [x] Verify network passphrase set to `Test SDF Network ; November 2015`
- [x] Handle RPC node failovers automatically
- [x] Support simulation pre-flight checks
