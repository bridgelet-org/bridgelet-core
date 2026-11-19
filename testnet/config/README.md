# Testnet Config

Configuration templates for **consuming** the Bridgelet contracts already deployed to Stellar testnet.

## What this directory is

Files here are **integrator-facing consumption config**: non-secret values needed to point client code, SDKs, or test harnesses at the deployed testnet contracts — network endpoints, network passphrase, contract IDs, and similar read-side settings.

- **Read-only.** Nothing here triggers or configures a deployment. These are inputs to read from, not state to mutate. Treat the committed templates as read-only: copy one, rename it, and fill it in — do not edit templates in place.
- **No secrets, ever.** Never put a secret key (`S...`), seed phrase, or any private signing material in this directory or in files derived from these templates. If a consumer needs deployment-side credentials, source them from the root `.env` flow instead of duplicating them here.

## Relationship to the root `.env.example`

The repo-root `.env.example` is **deployment configuration**, not consumption config. It collects the deployment-time secrets and parameters that `scripts/deploy-testnet.sh` requires:

| Root `.env.example` | Role |
|---|---|
| `SIGNER_SECRET_KEY` | Secret key that pays fees and signs contract `initialize` calls |
| `AUTHORIZED_SIGNER_PUBLIC_KEY` | Public key `SweepController` verifies sweep signatures against |
| `RECOVERY_ADDRESS`, `CREATOR_ADDRESS` | Addresses used at initialization time |
| `*_CONTRACT_ID` | Written by the deploy script after a successful deploy |

The two serve different audiences and must not be confused:

- **`.env.example` → deployer persona.** Secrets, admin keys, deploy flow. Lives at repo root.
- **`testnet/config/` → integrator persona.** Public, non-secret consumption values for building against the already-deployed contracts. Lives here.

Note the distinction is **deployment-time vs consumption-time**, not merely secret vs public: `.env.example` also carries public network values (e.g. `STELLAR_HORIZON_URL`, `STELLAR_SOROBAN_RPC_URL`, `STELLAR_NETWORK_PASSPHRASE`) that integrators need too. Overlap on public values is fine; secret values belong only to the root `.env` flow.

## Where the contract IDs come from

Contract IDs consumed by integrators are produced by the deploy flow and recorded in:

- `deployments/testnet.json` — structured record of the latest testnet deployment
- `deployment-artifacts/contract-ids.txt` — flat `KEY=VALUE` list, also used as a CI artifact

Those are the source of truth. Templates in this directory may reference or mirror those IDs, but they do not mint or override them.

## Files in this directory

| File | Purpose |
|---|---|
| [`network.toml`](network.toml) | Machine-readable testnet network values (passphrase, Horizon URL, Soroban RPC URL, Friendbot URL). Load this file instead of copying values out of the docs. |
| [`soroban-cli-identity-setup.md`](soroban-cli-identity-setup.md) | Testnet setup with `stellar keys generate`, `stellar keys fund` and `stellar network add`. |

## Loading `network.toml`

`network.toml` deliberately leaves out contract IDs. Read those from the files
named under `[sources]` in `network.toml`. Those point at the deploy outputs
listed above.

Python 3.11+ (standard library):

```python
import tomllib
with open("testnet/config/network.toml", "rb") as f:
    net = tomllib.load(f)["network"]
rpc_url, passphrase = net["soroban_rpc_url"], net["passphrase"]
```

Node / TypeScript (`npm i smol-toml`):

```ts
import { readFileSync } from "node:fs";
import { parse } from "smol-toml";
const { network } = parse(readFileSync("testnet/config/network.toml", "utf8")) as any;
// network.horizon_url, network.soroban_rpc_url, network.passphrase
```

Rust (`toml` + `serde` as dev-dependencies, e.g. in an integration test):

```rust
#[derive(serde::Deserialize)]
struct Network { passphrase: String, horizon_url: String, soroban_rpc_url: String }
#[derive(serde::Deserialize)]
struct Config { network: Network }

// include_str! paths are relative to the current source file; adjust as needed.
let cfg: Config = toml::from_str(include_str!("../../testnet/config/network.toml")).unwrap();
```

Exporting the values as environment variables (bash, using Python to parse):

```bash
eval "$(python3 - <<'PY'
import tomllib, shlex
c = tomllib.load(open("testnet/config/network.toml", "rb"))
for key, var in c["env_var_names"].items():
    print(f"export {var}={shlex.quote(c['network'][key])}")
PY
)"
```

When an endpoint or the passphrase changes, update `network.toml`,
`.env.example`, and `testnet/docs/network-config.md` in the same change.
