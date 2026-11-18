# Test Suites and Mid-Run Contract Upgrades

`EphemeralAccount::upgrade(new_wasm_hash)` lets the admin change the code behind
an instance. If it happens while your integration suite is running, you get
confusing failures that look like regressions in your code. This page shows how
to fail fast with a clear message instead.

> Recommendation based on `testnet/runbooks/upgrade-verification.md` and the
> contract source. `upgrade` emits no event, so upgrades are visible only through
> the code identity and the changelog. Commands are unexecuted recipes.

## Principle

At **suite startup**, record the code identity of every contract you test, compare
it with an **expected** value you pinned, and abort with an explicit message on
mismatch. Optionally re-check at the end to detect a mid-run change.

## What to pin

| Item | Where the expected value comes from |
|---|---|
| Contract ID | `testnet/registry/contract-status.md` / your config |
| WASM hash | `testnet/registry/wasm-hash-reference.md`, `deployments/testnet.json` (`ephemeralAccountWasmHash` for the child code), or your own build |
| Commit/version | `testnet/docs/changelog.md` |

Keep them in a small file checked into your repo, e.g. `testnet-pins.json`:

```json
{
  "network": "testnet",
  "contracts": {
    "sweepController": { "id": "<C...>", "wasmHash": "<64 hex>" },
    "ephemeralAccountTemplate": { "wasmHash": "<64 hex>" }
  }
}
```

## Reading the current hash

A WASM hash is the SHA-256 of the contract code. Two ways:

```bash
# 1) Fetch the deployed code and hash it
stellar contract fetch --id "<CONTRACT_ID>" --network testnet --out-file /tmp/current.wasm
sha256sum /tmp/current.wasm        # compare with the pinned wasmHash

# 2) Read the contract instance ledger entry through Soroban RPC getLedgerEntries
#    and extract the executable's wasm_hash.
```

For **child** `EphemeralAccount` instances created from a template, check each
instance you use (an `upgrade` applies to one instance, not to the template).

## Fail fast

```text
before_all():
    for contract in pins:
        actual = read_wasm_hash(contract.id)
        if actual != contract.wasmHash:
            abort("Contract <id> code changed.\n expected <pinned>\n actual   <actual>\n"
                  "Check testnet/docs/changelog.md; update pins or investigate an unexpected upgrade.")
after_all():
    re-read hashes; if any changed -> mark the whole run INVALID (do not trust failures)
```

Make the abort message name the contract, the expected and actual hash, and point
to the changelog. A failing test caused by an upgrade should never be reported as
a product bug.

## Handling an expected upgrade

1. Read the changelog entry; update the pinned hash in the same change that
   updates any tests for changed behaviour.
2. Re-run `testnet/runbooks/upgrade-verification.md` steps for state integrity
   (status, payments, expiry unchanged).
3. Consider running the suite against both old and new behaviour only if you
   support both.

## Parallel runs

If several suites share one deployment, an upgrade mid-run affects all of them.
Serialize runs around announced upgrades, or give each run its own deployment
(`testnet/integration/local-mock-vs-live-testnet-tradeoffs.md`).

## Also pin

The `SweepController` nonce is **not** a code identity, but a reset/redeploy
resets it to `0`; if your tests assume a nonce, read it fresh at startup
(`testnet/runbooks/nonce-desync.md`).
