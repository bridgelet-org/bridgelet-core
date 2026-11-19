# Local Sandbox vs Stellar Testnet

## Why this page exists

The main README is explicit: there is **no local-sandbox deployment entrypoint**
in this repo today. `scripts/test-local.sh` does not exist, and `scripts/test.sh`
runs unit tests only — it does not stand up a local network or deploy anything.
Every testnet deployment is a manual run of `scripts/deploy-testnet.sh` against
the public Stellar testnet.

This page is for anyone tempted to stand up their own local Soroban sandbox
(for example via a Stellar Quickstart container) to test Bridgelet. It sets
expectations: a local sandbox is **not an official or supported entrypoint** for
this project. Nothing in the repo's scripts, CI, or config templates targets one.
If you choose to run one, you own the setup, the funding, and the differences
listed below.

For the supported path, use public testnet with the values in
[`network.toml`](network.toml) and the setup steps in
[`soroban-cli-identity-setup.md`](soroban-cli-identity-setup.md).

## Side-by-side comparison

| Setting | Public testnet (supported) | Local sandbox (unsupported, self-managed) |
|---|---|---|
| Network passphrase | `Test SDF Network ; September 2015` (exact, including spaces around `;`) | Whatever your local node is configured with; for the common Stellar Quickstart standalone image this is often `Standalone Network ; February 2017`. Confirm against your node — a mismatch breaks every signature. |
| Soroban RPC URL | `https://soroban-testnet.stellar.org` | A localhost endpoint you run yourself, e.g. `http://localhost:8000` (verify your container's published port). |
| Horizon URL | `https://horizon-testnet.stellar.org` | The Horizon endpoint exposed by your local stack, if any. |
| Friendbot | `https://friendbot.stellar.org` | Usually none unless you configure and run your own. Do not assume `stellar keys fund` will work against a custom network. |
| Funding mechanism | Friendbot sends free test XLM; `stellar keys fund <name> --network testnet` | You must fund accounts from a pre-funded root account or a faucet you provide. The CLI's built-in Friendbot flow does not apply to an arbitrary local network. |
| Faucet | Friendbot is the de facto faucet; alternative routes are documented in `testnet/faucet/` | No shared faucet; you are the faucet. Repeated funding is manual. |
| Ledger close time | Real time (roughly every few seconds), controlled by the public network | Controlled by your node config; may be faster or require manual close/time-travel. Do not assume wall-clock-based expiry tests behave the same. |
| Time travel | Not available; `expiry_ledger` advances with real ledgers | Local/dev tooling may let you advance ledger time. Useful for expiry testing, but it is a local-only behavior and not representative of testnet. |
| Contract IDs | Recorded in `deployments/testnet.json` and `deployment-artifacts/contract-ids.txt`; change on every redeploy/reset | Minted by your own deploys on your own node; never appear in the repo's deployment records. |
| Reset behaviour | Periodically reset by the SDF; all accounts and contracts are wiped, IDs change | State persists until you reset or recreate the container, which you control. No external reset schedule. |
| Deploy scripts | `scripts/deploy-testnet.sh` targets testnet | No repo script targets a local network. You must deploy by hand (or write your own script) and track IDs yourself. |
| Test entrypoint | Testnet integration testing is manual/CLI-driven | `scripts/test.sh` is unit tests only; there is no local-network test harness in this repo. |

## What `scripts/test.sh` actually does

Despite older references to a local sandbox entrypoint:

- `scripts/test.sh` runs the Rust unit tests for `ephemeral_account` only. It is
  a local, in-process check using the Soroban test environment — no network, no
  RPC, no deployed contracts.
- It does **not** run `sweep_controller`, `reserve_contract`, or
  `account_factory` tests, and it does **not** deploy to any network.
- There is **no** `scripts/test-local.sh` in this repo.

To run the other test suites locally (still unit/integration tests, not a
sandbox deployment):

```bash
for c in ephemeral_account sweep_controller reserve_contract account_factory; do
  (cd "contracts/$c" && cargo test)
done

# sweep_controller integration tests live under tests/
(cd contracts/sweep_controller && cargo test --test integration)
```

If you need end-to-end contract calls, the supported target is public testnet via
`stellar-cli`, not a local sandbox.

## Decision guide

**Use public testnet when:**

- You want the supported, documented path with known network values.
- You need Friendbot funding and pre-recorded contract IDs.
- You are testing interactions with real testnet state, resets, and other
  testers' deployments.
- You are doing manual QA of signing flows (see
  [`wallet-compatibility.md`](wallet-compatibility.md)).

**Consider a local sandbox only when:**

- You need deterministic ledger control / time travel for expiry-ledger testing.
- You are deliberately developing or debugging the deployment tooling itself
  and accept that nothing in this repo supports it.
- You can tolerate a self-managed network: your own funding, your own contract
  IDs, and no help from the repo's scripts when something breaks.

Even then, treat local results as indicative only. Before relying on behavior,
reproduce it on testnet with the exact passphrase, RPC URL, and contract IDs from
the repo's deployment records.

## Checklist when switching configuration

No file in `testnet/config/` points at a local sandbox. If you move a client or
test harness between the two, change **all** of the following together:

- [ ] Network identifier passed to `--network` (the built-in `testnet` vs your
      own registered local network name via `stellar network add`).
- [ ] Network passphrase: `Test SDF Network ; September 2015` on testnet; your
      local node's passphrase on the sandbox. This must be byte-exact.
- [ ] Soroban RPC URL: `https://soroban-testnet.stellar.org` vs your localhost
      RPC endpoint.
- [ ] Horizon URL: `https://horizon-testnet.stellar.org` vs your local Horizon.
- [ ] Friendbot URL: `https://friendbot.stellar.org` on testnet; a faucet/root
      account you control locally.
- [ ] Funding method: `stellar keys fund --network testnet` vs manual funding
      from your local root account.
- [ ] Contract IDs: current IDs from `deployments/testnet.json` /
      `deployment-artifacts/contract-ids.txt` vs the IDs your local deploy
      produced. Signatures are bound to a contract ID, so a signature made for
      one deployment will not verify on the other.
- [ ] The nonce for the target controller: re-read via `get_nonce()` after any
      switch or redeploy (see [`nonce-tracking.md`](nonce-tracking.md)).
- [ ] Any hardcoded addresses, asset contract IDs, or expiry ledgers. Testnet
      reset and local redeploy both invalidate them.
- [ ] Wallet/dapp network selection, if doing manual signing QA.

When switching back to testnet, restore values from [`network.toml`](network.toml)
rather than from memory, and re-verify the live contract IDs before relying on
them.

## Related documentation

- [`network.toml`](network.toml) - machine-readable testnet network values
- [`soroban-cli-identity-setup.md`](soroban-cli-identity-setup.md) - identities,
  funding and network registration on testnet
- [`nonce-tracking.md`](nonce-tracking.md) - nonce-safe signing pre-flight
- [`wallet-compatibility.md`](wallet-compatibility.md) - manual signing QA
- [`testnet/docs/overview.md`](../docs/overview.md) - which contracts are
  actually deployed on testnet
- [`README.md`](../../README.md) - build/test/deploy status and the note that no
  local-testing entrypoint exists
