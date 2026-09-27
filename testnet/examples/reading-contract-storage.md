# Reading Contract Storage Directly

When debugging an `EphemeralAccount`, it can be useful to inspect the contract's persistent storage directly through Soroban RPC instead of relying only on a contract read-only method.

This is useful when investigating a problem where the contract's own reporting method may itself be incorrect or affected by the issue being investigated.

## Why read storage directly?

`EphemeralAccount` stores its status in persistent instance storage under `DataKey::Status`:

```rust
env.storage().instance().set(&DataKey::Status, &status);
```

The contract also exposes `get_status()`, which reads the same storage value.

For normal application use, calling `get_status()` is simpler. For debugging, however, querying the ledger entry directly provides an independent view of the stored value without executing the contract's `get_status()` method.

## Prerequisites

You need:

* Stellar CLI installed
* Access to the target Stellar network
* An `EphemeralAccount` contract ID
* A contract instance that has been initialized and has a persistent `Status` entry

Set the contract ID:

```bash
export EPHEMERAL_ACCOUNT_ID="YOUR_EPHEMERAL_ACCOUNT_CONTRACT_ID"
```

## Read the Status storage entry

`DataKey::Status` is a unit variant of a `#[contracttype]` enum. Soroban encodes this as a one-element vector containing the `Status` symbol.

The base64-encoded SCVal for the storage key is:

```text
AAAAEAAAAAEAAAAPAAAABlN0YXR1cwAA
```

Fetch the persistent contract-data entry directly:

```bash
stellar ledger entry fetch contract-data \
  --contract "$EPHEMERAL_ACCOUNT_ID" \
  --key-xdr "AAAAEAAAAAEAAAAPAAAABlN0YXR1cwAA" \
  --durability persistent \
  --network testnet \
  --output json-formatted
```

This command reads the contract-data ledger entry through Stellar RPC rather than invoking `EphemeralAccount::get_status()`.

## Example response

A response will contain the ledger entry and its stored value. Depending on the CLI version and output format, the value will be represented as decoded XDR/JSON similar to:

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
      "vec": [
        {
          "symbol": "Active"
        }
      ]
    }
  }
}
```

The exact JSON shape may vary between Stellar CLI/RPC versions. The important fields are:

* the contract ID matches the `EphemeralAccount` being investigated;
* the key represents `DataKey::Status`;
* the durability is `persistent`;
* the stored value contains the current `AccountStatus`.

The current `AccountStatus` values used by `EphemeralAccount` include:

```text
Active
PaymentReceived
Swept
Expired
```

## Compare with the contract read-only method

The normal application-level read is:

```bash
stellar contract invoke \
  --id "$EPHEMERAL_ACCOUNT_ID" \
  --network testnet \
  -- \
  get_status
```

This executes the contract's `get_status()` function.

The direct storage query is different:

```bash
stellar ledger entry fetch contract-data \
  --contract "$EPHEMERAL_ACCOUNT_ID" \
  --key-xdr "AAAAEAAAAAEAAAAPAAAABlN0YXR1cwAA" \
  --durability persistent \
  --network testnet \
  --output json-formatted
```

The first command asks the contract to report its status.

The second asks the RPC service for the current ledger entry containing the stored status.

## Debugging workflow

When investigating a suspected status-reporting problem:

1. Record the `EphemeralAccount` contract ID.
2. Call `get_status()`.
3. Fetch the `Status` storage entry directly through RPC.
4. Compare the two results.
5. If they differ, investigate the contract code, deployed WASM version, or storage interpretation.
6. If they agree, continue investigating the surrounding application or RPC usage.

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

Direct storage inspection is a debugging technique, not a replacement for the contract API.

The storage key encoding depends on the contract's `DataKey` definition. If the contract changes its storage key type or layout, the XDR used to query the entry must also change.

Also, `getLedgerEntries` reads current live ledger state. It is not a historical state query. If the entry is archived or no longer available through the RPC state retained by the network, the query may not return the expected value.

For general RPC details, see Stellar's `getLedgerEntries` documentation.
