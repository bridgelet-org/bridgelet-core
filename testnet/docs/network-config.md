# Testnet Network Configuration

Everything an SDK or CLI integrator needs to point at the Bridgelet testnet
deployment. These values were previously only discoverable inside
`scripts/deploy-testnet.sh`.

## Network endpoints

| Setting | Value |
|---|---|
| Network name (stellar-cli) | `testnet` |
| Network passphrase | `Test SDF Network ; September 2015` |
| Horizon URL | `https://horizon-testnet.stellar.org` |
| Soroban RPC URL | `https://soroban-testnet.stellar.org` |
| Friendbot (test XLM faucet) | `https://friendbot.stellar.org/?addr=<G...>` |

The passphrase must match **exactly**, including the spaces around `;`.
Transactions signed with any other passphrase are rejected by the network.

## Environment variables

```bash
export STELLAR_NETWORK=testnet
export STELLAR_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
export STELLAR_HORIZON_URL=https://horizon-testnet.stellar.org
export STELLAR_SOROBAN_RPC_URL=https://soroban-testnet.stellar.org
```

Or as a `.env` file for the SDK:

```dotenv
STELLAR_NETWORK=testnet
STELLAR_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
STELLAR_HORIZON_URL=https://horizon-testnet.stellar.org
STELLAR_SOROBAN_RPC_URL=https://soroban-testnet.stellar.org

STELLAR_CONTRACT_EPHEMERAL_ACCOUNT=CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3
STELLAR_CONTRACT_SWEEP_CONTROLLER=CBEU4X5MNGOECBSTNEUFMBALH2YI5YV4UIH7YXRNOLR2DNLZQD4Z5KWE
```

## Deployed contract IDs

Source of truth: [`deployment-artifacts/contract-ids.txt`](../../deployment-artifacts/contract-ids.txt).

| Contract | Contract ID |
|---|---|
| EphemeralAccount | `CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3` |
| SweepController | `CBEU4X5MNGOECBSTNEUFMBALH2YI5YV4UIH7YXRNOLR2DNLZQD4Z5KWE` |
| ReserveContract | `CACYMICFLHSPKVMCK336IEGCWAQB4XBC7P7HTJSQY3R7Y2YASE4AX3MS` |
| AccountFactory | `CARBMZY4466SWP3RTN3DR2F4JYNR5UCXD2ICFISV2YI4TS3AAZV2TQ24` |

Other useful testnet values:

| Item | Value |
|---|---|
| EphemeralAccount WASM hash (installed on testnet) | `5e667ea0687341bdccc81538492143b573777dd04b2450cdeb89b02cee62c58e` |
| Native XLM Stellar Asset Contract | `CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC` |

The native XLM contract ID is derived from the passphrase. You can regenerate
it with `stellar contract id asset --asset native --network testnet`, or with
`Asset.native().contractId(Networks.TESTNET)` in the JS SDK.

> **Note:** `deployments/testnet.json` still contains `null` placeholders.
> Use `deployment-artifacts/contract-ids.txt` until that file is repopulated.

## Configuring clients

### stellar-cli

`testnet` is built into stellar-cli, so `--network testnet` works without
any setup. To register it explicitly, or under another name:

```bash
stellar network add testnet \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015"
```

### JavaScript / TypeScript (`@stellar/stellar-sdk`)

```js
import { Horizon, rpc, Networks } from '@stellar/stellar-sdk';

const passphrase = Networks.TESTNET; // "Test SDF Network ; September 2015"
const horizon = new Horizon.Server('https://horizon-testnet.stellar.org');
const soroban = new rpc.Server('https://soroban-testnet.stellar.org');
```

### Rust (`soroban-sdk` tests / tooling)

```rust
const NETWORK_PASSPHRASE: &str = "Test SDF Network ; September 2015";
const SOROBAN_RPC_URL: &str = "https://soroban-testnet.stellar.org";
const HORIZON_URL: &str = "https://horizon-testnet.stellar.org";
```

`bridgelet_shared::passphrase::TESTNET_PASSPHRASE` exposes the same passphrase
constant.

## Which endpoint to use

- **Soroban RPC** is for everything that touches contracts: simulating,
  submitting and polling contract invocations, and reading contract storage
  and events.
- **Horizon** is for classic Stellar data: account balances, payment
  history, and trustlines. Use it to watch for inbound payments to an
  ephemeral account before calling `record_payment`.

## Testnet caveats

- The SDF resets testnet periodically. After a reset, every account and
  contract ID above is gone until `scripts/deploy-testnet.sh` is re-run and
  the IDs are updated.
- Ledgers close roughly every 5 seconds. Allow for this when you compute
  `expiry_ledger` values (10,000 ledgers ≈ 14 hours).
- Do not reuse these values on mainnet. The mainnet passphrase is
  `Public Global Stellar Network ; September 2015`, and it uses different
  endpoints and contract IDs.

## See also

- [getting-started.md](getting-started.md): first `initialize` and
  `record_payment` calls against testnet
- [docs/testnet-deployment.md](../../docs/testnet-deployment.md): how the
  contracts are deployed
- [docs/api-reference.md](../../docs/api-reference.md): full contract API
