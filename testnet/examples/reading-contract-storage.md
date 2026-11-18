# Reading Contract Storage Directly

## Status

This is a **source-derived debugging example**, not a live-state observation.
The canonical [`testnet/registry/contract-status.md`](../registry/contract-status.md)
records all four Bridgelet contracts as undeployed, with `TBD`/`N/A` IDs and
`Last Verified: Never`. A contract ID from a deployment artifact is a claim to
verify on-chain, not proof of liveness.

Command availability was checked against `stellar` CLI 23.4.1, the version
recorded in [`../docs/faq.md`](../docs/faq.md). Both `stellar contract read` and
`stellar ledger entry fetch contract-data` are available in that version. No
live RPC query was made for this document.

For the contract-method pre-flight that should normally come first, see
[`checking-account-status.md`](checking-account-status.md). Direct storage
inspection is the independent fallback when the contract's own reporting may be
part of the problem.

## Why read storage directly?

`EphemeralAccount` keeps its status in **contract instance storage**:

```rust
env.storage().instance().set(&DataKey::Status, &status);
```

Instance storage is written into a contract-data ledger entry with
**persistent** durability; that does not mean the contract uses the
`env.storage().persistent()` API. The distinction matters when choosing CLI
flags:

| Item | Value for `DataKey::Status` |
|---|---|
| Storage API | `env.storage().instance()` |
| Ledger entry type | contract data |
| Durability flag | `persistent` |
| Key | `DataKey::Status` |
| Value | `AccountStatus` |

The contract also exposes `get_status()`, which reads the same key:

```rust
env.storage()
    .instance()
    .get(&DataKey::Status)
    .unwrap_or(AccountStatus::Active)
```

For normal application use, calling `get_status()` is simpler. For debugging, a
direct ledger read provides an independent view that does not execute the
contract's getter. The `unwrap_or` default is a reason to do both: **a missing
`Status` key is not the same as a stored `Active` value.** `initialize()` writes
`Active`; an uninitialized instance has no key, yet `get_status()` still returns
`Active`.

## Prerequisites

You need:

- Stellar CLI installed
- Access to the target Stellar network
- A contract ID that has been verified as the `EphemeralAccount` instance under
  investigation
- An instance that has been initialized and has a `Status` entry

Set the network, contract ID, and encoded key:

```bash
export NETWORK=testnet
export EPHEMERAL_ACCOUNT_ID="<verified-ephemeral-account-id>"
export STATUS_KEY_XDR="AAAAEAAAAAEAAAABAAAADwAAAAZTdGF0dXMAAA=="
```

## Verify the `DataKey::Status` key encoding

`DataKey::Status` is a unit variant of a `#[contracttype]` enum. Soroban encodes
that key as an `ScVal` vector containing one symbol, `Status`. Generate the
base64 XDR rather than hand-writing it:

```bash
stellar xdr encode --type ScVal --output single-base64 <<'JSON'
{ "vec": [ { "symbol": "Status" } ] }
JSON
```

The output is the `STATUS_KEY_XDR` value above. A truncated or hand-edited key
will not decode as an `ScVal`; regenerate it whenever the key type changes.

## Read the value with `stellar contract read`

`stellar contract read` prints the current value of a contract-data entry. It
does not invoke a contract method:

```bash
stellar contract read \
  --id "$EPHEMERAL_ACCOUNT_ID" \
  --key-xdr "$STATUS_KEY_XDR" \
  --durability persistent \
  --network "$NETWORK" \
  --output json
```

This is the shortest direct-storage check when you only need the decoded value.
If the key is absent, expect a not-found/empty result rather than a value. Do
not translate that into `Active` without checking `get_info()` as described in
[`checking-account-status.md`](checking-account-status.md).

## Fetch the full ledger entry

For lower-level debugging, fetch the whole contract-data ledger entry. This
shows the key, durability, and entry metadata alongside the value:

```bash
stellar ledger entry fetch contract-data \
  --contract "$EPHEMERAL_ACCOUNT_ID" \
  --key-xdr "$STATUS_KEY_XDR" \
  --durability persistent \
  --network "$NETWORK" \
  --output json-formatted
```

Both commands read the entry through Soroban RPC. Neither invokes
`EphemeralAccount::get_status()`.

## Example response

A full-entry response contains the ledger entry and its stored value. Depending
on the CLI version and output format, the value appears as decoded XDR/JSON
similar to:

```json
{
  "contractData": {
    "contract": "...",
    "key": {
      "vec": [
        {
          "symbol": "Status"
        }
      ]
    },
    "durability": "persistent",
    "val": {
      "u32": 0
    }
  }
}
```

The exact JSON shape may vary between Stellar CLI/RPC versions. The important
fields are:

- the contract ID matches the `EphemeralAccount` being investigated;
- the key represents `DataKey::Status`;
- the durability is `persistent`;
- the stored value contains the current `AccountStatus`.

The current `AccountStatus` values used by `EphemeralAccount` are:

| Discriminant | State |
|---:|---|
| `0` | `Active` |
| `1` | `PaymentReceived` |
| `2` | `Swept` |
| `3` | `Expired` |

## Compare with the contract read-only method

The normal application-level read is:

```bash
stellar contract invoke \
  --send no \
  --id "$EPHEMERAL_ACCOUNT_ID" \
  --network "$NETWORK" \
  --source "<funded-testnet-identity>" \
  -- \
  get_status
```

`--send no` simulates the invocation and returns the result without signing or
submitting a transaction. The call executes `get_status()` and applies its
`unwrap_or(AccountStatus::Active)` default. The direct reads do not:

| Method | Executes contract code? | What it answers |
|---|---|---|
| `get_status()` | Yes | "What does the contract report, including its default when the key is absent?" |
| `stellar contract read` | No | "What decoded value is stored at this key right now?" |
| `stellar ledger entry fetch contract-data` | No | "What whole ledger entry is stored at this key right now?" |

## Debugging workflow

When investigating a suspected status-reporting problem:

1. Record the `EphemeralAccount` contract ID and verify that it resolves.
2. Prove initialization with `get_info()` first; see
   [`checking-account-status.md`](checking-account-status.md).
3. Call `get_status()` with `--send no`.
4. Read the `Status` key directly with `stellar contract read`.
5. Fetch the full entry if the value, key, or durability is in question.
6. Compare the results:
   - no entry + `get_status() == Active` points to an uninitialized instance or
     a missing key;
   - an entry whose value differs from `get_status()` points to contract code,
     deployed WASM, or decoding;
   - agreement means the getter and storage agree, so continue with the
     surrounding application or RPC usage.

For example:

```text
get_status()
    ↓
PaymentReceived

Direct persistent storage
    ↓
PaymentReceived

Both agree
```

If the results disagree:

```text
get_status()
    ↓
Active

Direct persistent storage
    ↓
PaymentReceived

Investigate contract code/deployment or decoding
```

## Important limitations

Direct storage inspection is a debugging technique, not a replacement for the
contract API.

- **Storage layout is a contract implementation detail.** The key here is a
  symbol-based `Vec<ScVal>` derived from the unit variant name `Status`. If the
  variant is renamed, given fields, or replaced with a different key type, the
  XDR must be regenerated. Moving the value to another storage API may change
  durability and query flags even if the key bytes stay the same. Variant
  reordering alone does not change a symbol key, but do not rely on that without
  checking the deployed WASM.
- **Value decoding is also schema-dependent.** `AccountStatus` is a
  `#[repr(u32)]` integer enum, so the stored value is the numeric discriminant
  (`0` for `Active` in the sample above). Renaming a variant does not change the
  raw encoded value; changing the discriminant or enum representation can.
  Client output may still label the number using the current generated schema,
  so decode with the spec that matches the deployed WASM.
- **Durability and storage area are separate concepts.** This key lives in
  `instance()` storage and is read with `--durability persistent`. Changing
  either requires changing the query.
- **This is current state only.** `getLedgerEntries` does not provide historical
  state. If the entry is archived, evicted, or outside the RPC node's retained
  state, the query may not return the expected value.
- **Output shapes vary.** Treat the sample JSON as illustrative; verify the
  fields and decode with the CLI/RPC version your tooling uses.
- **A successful read proves state, not liveness of a deployment record.** It
  proves that an entry exists at the address you queried; it does not reconcile
  the conflicting IDs in deployment artifacts with the canonical registry.

## Related documentation

- [`checking-account-status.md`](checking-account-status.md) — initialization
  gate and read-only pre-flight probes
- [`README.md`](README.md) — example conventions and placeholders
- [`../registry/contract-status.md`](../registry/contract-status.md) — canonical
  deployment status
- [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md) —
  live-ID verification procedure
- [`../docs/soroban-cli-quickref.md`](../docs/soroban-cli-quickref.md) —
  `simulateTransaction` and CLI background

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-27 | Initial direct storage reading example |
| 1.1 | 2026-11-18 | Clarified CLI 23.4.1 invocation with `--send no`, corrected `Status` key XDR, added `contract read`, corrected `AccountStatus` XDR value, initialization caveat, layout caveats, and status cross-reference |
