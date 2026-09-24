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
