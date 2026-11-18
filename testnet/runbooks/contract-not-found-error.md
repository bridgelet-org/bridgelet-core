# Runbook: "Contract Not Found" Errors Against Testnet

## Symptom

A read or invoke against a Bridgelet contract fails with an RPC error saying the
contract, its instance, or its code cannot be found (wording varies by tool: for
example `contract not found`, a missing ledger entry for the contract instance,
or `HostError: Error(Storage, MissingValue)` during simulation).

## Likely causes (check in this order)

| # | Cause | How to confirm |
|---|---|---|
| 1 | **Wrong or stale contract ID** — an ID copied from an old document or artifact. `deployments/testnet.json`, `deployment-artifacts/contract-ids.txt` and `testnet/registry/contract-status.md` can disagree; the registry currently lists all four contracts as not deployed | Compare against `testnet/registry/contract-status.md` and `testnet/registry/last-verified.json`; run `testnet/registry/verify-contract-live.md`. See also `testnet/docs/changelog.md` and `testnet/policies/reset-policy.md` |
| 2 | **Wrong network** — a testnet ID queried on mainnet/futurenet, or the passphrase/RPC mismatch | `getNetwork` against your RPC; compare with `testnet/docs/network-config.md` |
| 3 | **Testnet was reset** — the SDF periodically resets testnet; every contract disappears | Ledger sequence went backwards or IDs that used to resolve no longer do; see `testnet/policies/reset-policy.md` |
| 4 | **Never deployed** — the contract was never deployed to this network; notably `account_factory` and `reserve_contract`, which the deploy script does not deploy (README, "Current Stub Inventory" and CI notes) | `testnet/registry/account-factory-status.md`, `testnet/registry/reserve-contract-status.md` |
| 5 | **Archived/expired ledger entry** — contract instance/code TTL elapsed and the entry was archived | Restore it (`stellar contract restore`) if the contract is expected to exist |
| 6 | **Child account not created yet** — you are using an ephemeral account ID that was never deployed/initialized | Confirm the creation transaction succeeded and the ID is the deployed child, not the factory or WASM hash |
| 7 | **Wrong kind of ID** — a `G...` account, a WASM hash, or a transaction hash used where a `C...` contract ID is required | Contract IDs start with `C` and are 56 characters |

## Triage steps

1. Confirm the network (`getNetwork`) and RPC are testnet.
2. Confirm the ID's format (`C...`, 56 chars).
3. Ask the RPC directly whether the instance exists:
   ```bash
   stellar contract info interface --id "<CONTRACT_ID>" --network testnet
   # or fetch the ledger entry for the contract instance via getLedgerEntries
   ```
4. Compare the ID with the registry and the changelog; find the most recent
   verification date.
5. If the ID exists in the registry but not on-chain, suspect a reset (cause 3) or
   a stale record (cause 1); follow `testnet/policies/reset-policy.md` and
   `testnet/runbooks/upgrade-verification.md`.
6. If the contract is `account_factory` or `reserve_contract`, treat "not found"
   as expected and use the workaround in `testnet/examples/batch-account-creation-mock.md`.

## Resolve

- Update your configuration to a verified ID; never "fix" by guessing.
- Record the stale source (document, artifact, env file) so it can be corrected.
- If a real deployment disappeared without a changelog entry, open an issue with
  the ID, network, time and the RPC response.

> Note: the old reference to a `testnet/registry/staleness-check.md` file does not
> exist; staleness is tracked via `testnet/registry/last-verified.json` and
> `testnet/registry/verify-contract-live.md`.
