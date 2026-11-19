# Bridgelet Testnet: Start Here

The single entry point for **using** the existing Bridgelet testnet deployment:
which contract IDs are live, what network configuration to point an SDK at, and
where to go when something breaks. This directory is documentation only; it does
not contain or change deployment tooling (`scripts/deploy-testnet.sh`,
`deployments/testnet.json`, `.github/workflows/deploy-testnet.yml`).

> Bridgelet Core is an **MVP and unaudited** (main [README](../README.md)).
> Testnet is best effort, is reset periodically by the Stellar Development
> Foundation, and holds no real value. Verify every contract ID before use.

## 1. Network configuration

| Setting | Value |
|---|---|
| Network passphrase | `Test SDF Network ; September 2015` |
| Horizon | `https://horizon-testnet.stellar.org` |
| Soroban RPC | `https://soroban-testnet.stellar.org` |
| Friendbot | `https://friendbot.stellar.org/?addr=<G...>` |

Full details and `.env` snippets: [docs/network-config.md](docs/network-config.md).

## 2. Which contract IDs are live

IDs recorded in `deployments/testnet.json` (`deployedAt` 2026-10-10T04:38:14Z):

| Contract | ID |
|---|---|
| `ephemeral_account` | `CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3` |
| `sweep_controller` | `CAJB4RRM5JRPXLITC5JUA7Q3LU2WQJDRPXVKMORXWHJJBPZRYWDJ5V4S` |
| `reserve_contract` | `CBG3Y4GWFTL4D37JGV2XWAWC7EUQ7XGH4XM3SFLQYYSKPB2IYMAS54HY` |
| `account_factory` | `CDBY4L3NTLW4OBZZTY73VL3NNWO2AGFFGBRJVDL3IMKDDLFF2W57WK6H` |

On **2026-11-18** all four were found on testnet by a read-only fetch of their
code, and the `ephemeral_account` WASM hash matched `deployments/testnet.json`
(details in [registry/testnet-launch-notes.md](registry/testnet-launch-notes.md),
if merged). **That can change at any time**: a redeploy or a testnet reset
invalidates these IDs. Re-verify before relying on them
([registry/verify-contract-live.md](registry/verify-contract-live.md)).

Sources disagree about status. `registry/contract-status.md` (dated 2026-09-24)
still says "not deployed", and [docs/overview.md](docs/overview.md) says only two
contracts are in the supported deployment. Treat the live on-chain check as the
authority, and see [docs/overview.md](docs/overview.md) for which contracts are
meant to be integrated against.

The shared `ephemeral_account` instance is already initialized; deploy your own
instance from the installed WASM instead of reusing it
([docs/getting-started.md](docs/getting-started.md)).

## 3. Where to go

| I want to... | Read |
|---|---|
| Make my first calls | [docs/getting-started.md](docs/getting-started.md) |
| Know what is deployed and callable | [docs/overview.md](docs/overview.md) |
| Know what is stubbed or limited | [docs/known-limitations.md](docs/known-limitations.md) |
| Set up the CLI identity and commands | [config/soroban-cli-identity-setup.md](config/soroban-cli-identity-setup.md), [docs/soroban-cli-quickref.md](docs/soroban-cli-quickref.md) |
| Fund a testnet account | [faucet/README.md](faucet/README.md) |
| Run an end-to-end example | [examples/README.md](examples/README.md) |
| Look up event topics | [registry/event-topics.md](registry/event-topics.md) |
| Troubleshoot | [runbooks/README.md](runbooks/README.md), [docs/faq.md](docs/faq.md) |
| Understand reset/redeploy behaviour | [policies/reset-policy.md](policies/reset-policy.md), [docs/changelog.md](docs/changelog.md) |
| Review security caveats | [security/README.md](security/README.md), [security/unverified-signature-path-warning.md](security/unverified-signature-path-warning.md) |
| Get support or find owners | [docs/support.md](docs/support.md), [docs/contact-and-ownership.md](docs/contact-and-ownership.md) |

## 4. Directory map

| Directory | Contents |
|---|---|
| `docs/` | Overview, network config, getting started, FAQ, limitations, changelog |
| `config/` | Per-topic test configuration (CLI identity, nonce, expiry, gas-free claim) |
| `examples/` | Worked integration examples |
| `faucet/` | Funding testnet accounts |
| `integration/` | Integration guidance for consuming applications |
| `monitoring/` | Proposed monitoring and thresholds (nothing automated yet) |
| `policies/` | Reset, versioning, contribution and related policies |
| `registry/` | Contract status, IDs, hashes, event topics, test accounts |
| `reset/` | Reset communication and procedure material |
| `runbooks/` | Troubleshooting procedures |
| `security/` | Security notes and warnings |
| `tooling/` | Specs for proposed tooling |

## 5. Ground rules

- Never put a secret key or signing seed in the repository, an issue, a log or a URL.
- Never reuse a key intended for mainnet on testnet.
- Sweep through `SweepController::execute_sweep` or `::claim`, not by calling
  `EphemeralAccount::sweep()` directly.
