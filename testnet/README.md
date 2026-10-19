# Bridgelet Core on Testnet

## Purpose

This directory is the home for everything an integrator or new contributor needs
to **use** the Bridgelet testnet deployment: the network settings to point an
SDK at, the status of each contract, worked examples, runbooks for when
something breaks, and the policies that govern a shared environment.

It is documentation only. It does not deploy anything. The deployment itself
lives in `scripts/deploy-testnet.sh`, `deployments/testnet.json` and
`.github/workflows/deploy-testnet.yml`, and nothing in `testnet/` changes them.

## Status

**Read this first.** No Bridgelet contract is currently recorded as deployed to
Stellar testnet.

| Contract | Deployed | Source |
|---|---|---|
| `EphemeralAccount` | No (ID `TBD`) | [`registry/contract-status.md`](registry/contract-status.md) |
| `SweepController` | No (ID `TBD`) | [`registry/contract-status.md`](registry/contract-status.md) |
| `ReserveContract` | No | [`registry/contract-status.md`](registry/contract-status.md) |
| `AccountFactory` | No | [`registry/contract-status.md`](registry/contract-status.md) |

`deployments/testnet.json` and `deployment-artifacts/contract-ids.txt` contain
contract IDs, but the status registry does not corroborate them. **Treat those
IDs as unverified** until someone confirms them with
[`registry/verify-contract-live.md`](registry/verify-contract-live.md).

Most documents here therefore describe what the code **specifies**, not what was
observed on a live deployment, and say so at the top. The deployment-status
documents are the source of truth for that; this page does not repeat them.

## Start here

| I want to... | Go to |
|---|---|
| Understand which contracts I can call | [`docs/overview.md`](docs/overview.md) |
| Point an SDK or CLI at testnet | [`docs/network-config.md`](docs/network-config.md) |
| Make my first call | [`docs/getting-started.md`](docs/getting-started.md) |
| Look up a contract ID | [`registry/contract-status.md`](registry/contract-status.md) |
| Fund a test account | [`faucet/README.md`](faucet/README.md) |
| Fix something that is failing | [`runbooks/README.md`](runbooks/README.md) |
| Find out what changed recently | [`docs/changelog.md`](docs/changelog.md) |
| Ask a question or report a problem | [`docs/support.md`](docs/support.md) |
| Answer a common question quickly | [`docs/faq.md`](docs/faq.md) |

## What is in each directory

| Directory | What lives there | Index |
|---|---|---|
| [`docs/`](docs/) | Network configuration, getting started, changelog, FAQ, support, ownership | [`overview.md`](docs/overview.md) |
| [`config/`](config/) | Read-only consumption config and setup guides, with a no-secrets rule | [`README.md`](config/README.md) |
| [`registry/`](registry/) | Contract status, IDs, WASM hashes, event topics, upgrade log, verification | [`README.md`](registry/README.md) |
| [`examples/`](examples/) | Worked integration walkthroughs: create, pay, sweep, recover | [`README.md`](examples/README.md) |
| [`integration/`](integration/) | Mapping between Soroban types and SDK types | none |
| [`faucet/`](faucet/) | Funding testnet accounts and handling rate limits | [`README.md`](faucet/README.md) |
| [`runbooks/`](runbooks/) | Step-by-step diagnostics for specific failure symptoms | [`README.md`](runbooks/README.md) |
| [`monitoring/`](monitoring/) | Monitoring **plans** and procedures (not running monitoring) | [`README.md`](monitoring/README.md) |
| [`security/`](security/) | Operational security observations of the shared deployment | [`README.md`](security/README.md) |
| [`policies/`](policies/) | Versioning, reset, data retention, breaking-change notice, contribution | [`README.md`](policies/README.md) |
| [`reset/`](reset/) | The announcement template for a redeploy or reset | none |
| [`tooling/`](tooling/) | Specifications for consumer-side helper tools (not runnable code) | [`README.md`](tooling/README.md) |

Directories marked "none" have no index of their own; the table above is the
index.

## Three things worth knowing before you build

1. **Sweep through `SweepController`, never `EphemeralAccount::sweep()`
   directly.** The account's `sweep()` does not verify its signature argument.
   See [`security/unverified-signature-path-warning.md`](security/unverified-signature-path-warning.md).
2. **Testnet is shared and gets reset.** Contract IDs and all on-chain state can
   disappear, either from a Bridgelet redeploy or an SDF testnet reset. Do not
   store anything you would miss. See [`policies/reset-policy.md`](policies/reset-policy.md).
3. **The sweep nonce is global.** Other integrators' sweeps advance it, which is
   the usual reason a correct signature is rejected. See
   [`config/nonce-tracking.md`](config/nonce-tracking.md).

## Conventions

- Every document states whether it is derived from source or observed, and does
  not claim an on-chain observation it did not make.
- No secret key, seed or credential belongs anywhere in this tree. See
  [`policies/data-retention.md`](policies/data-retention.md).
- To add or correct a document, read
  [`policies/contribution-guide.md`](policies/contribution-guide.md) first.

## Related Documentation

- [`../README.md`](../README.md): the project README, including the MVP stub
  inventory
- [`../docs/api-reference.md`](../docs/api-reference.md): the contract interface
- [`../docs/security.md`](../docs/security.md): the design-level security model

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-26 | Initial entry point for the `testnet/` documentation tree |
