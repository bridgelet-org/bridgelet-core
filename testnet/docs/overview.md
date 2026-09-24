# Bridgelet Core on Testnet: Deployment Overview

This page is for anyone integrating against **bridgelet-core on Stellar testnet**.
It answers one question: *which of the four core contracts can I actually call
there?*

It summarizes what the [main README](../../README.md) and the existing
deployment tooling already say. It does **not** change or replace the deploy
tooling. To deploy, see [`docs/testnet-deployment.md`](../../docs/testnet-deployment.md)
and `scripts/deploy-testnet.sh`.

> Network: Stellar testnet (`Test SDF Network ; September 2015`)
> Soroban RPC: `https://soroban-testnet.stellar.org`

## TL;DR

| Contract | Deployed by `scripts/deploy-testnet.sh`? | Initialized by the script? | Safe to integrate against on testnet? |
|---|---|---|---|
| `ephemeral_account` | ✅ Yes | ❌ No (each account is initialized by its creator) | ✅ Yes, but route sweeps through `sweep_controller` |
| `sweep_controller` | ✅ Yes | ✅ Yes (`authorized_destination = null`) | ✅ Yes (the recommended entry point) |
| `reserve_contract` | ❌ No | ❌ No | ⚠️ Not part of the supported deployment |
| `account_factory` | ❌ No | ❌ No | ⚠️ Not part of the supported deployment |

**Bottom line:** only `ephemeral_account` and `sweep_controller` are part of the
supported testnet deployment. Don't build on `reserve_contract` or
`account_factory` on testnet until the deploy tooling covers them.

## Per-contract status

### 1. `ephemeral_account`: deployed

- The deploy script builds and deploys it, and writes its ID into
  `deployments/testnet.json` as `contracts.ephemeralAccount`.
- The script does **not** call `initialize`. Each ephemeral account is
  initialized by its creator with `creator`, `expiry_ledger`,
  `recovery_address`, `authorized_controller` and `admin`.
- **Caveat:** `sweep()` accepts an `auth_signature: BytesN<64>` but **does not
  verify it cryptographically**. It relies on `require_auth()` (stub tracked in
  #86). Don't call `EphemeralAccount::sweep()` directly. Use
  `SweepController::execute_sweep` or `SweepController::claim`.

### 2. `sweep_controller`: deployed and initialized

- The script deploys it and then calls `initialize` with:
  - `creator = $CREATOR_ADDRESS`
  - `authorized_signer = $AUTHORIZED_SIGNER_PUBLIC_KEY`
  - `authorized_destination = null` (unlocked mode: any destination is
    allowed if the signature is valid)
- Its ID is written into `deployments/testnet.json` as
  `contracts.sweepController`.
- This is the contract that does **real Ed25519 signature verification** with
  nonce replay protection. It also runs atomic multi-asset SEP-41 transfers
  and the experimental gas-free `claim()` path.

### 3. `reserve_contract`: not deployed by the tooling

- `scripts/deploy-testnet.sh` never deploys it.
- The script still *echoes* `RESERVE_CONTRACT_ID` at the end. That variable is
  never set by the script. The README flags this as a
  "fix needed before this script is production-usable".
- Even where it is deployed, the README notes it has **no integration wiring
  into `ephemeral_account` yet**. It is a standalone admin-set base-reserve
  store (`init` / `get` / `set` / `has`).

### 4. `account_factory`: not deployed by the tooling

- `scripts/deploy-testnet.sh` never deploys it.
- It needs a stored `ephemeral_account` WASM hash to batch-deploy accounts, and
  the tooling doesn't set that up either.
- Known limitation: `batch_initialize` reports *which* account failed to
  initialize but discards *why*.

## Where contract IDs live (and how far to trust them)

| File | What it contains | Trust level |
|---|---|---|
| `deployments/testnet.json` | Written by `deploy-testnet.sh`. Only has keys for `ephemeralAccount` and `sweepController`. The committed copy has `null` values and a note to run the script. | Authoritative **after you run the script yourself** |
| `deployment-artifacts/contract-ids.txt` | Four IDs (ephemeral account, sweep controller, reserve, factory), committed by hand in commit `741aec2`. | ⚠️ **Not reproducible** from the current script, which only deploys two contracts. Check each ID on-chain before you rely on it. Testnet resets also wipe contracts. |
| `docs/testnet-deployment.md` | Says the script deploys ReserveContract and AccountFactory too (step 3). | ⚠️ Out of date compared with the script. The script only deploys two contracts. |

To check that an ID is live, invoke a read-only method (for example,
`sweep_controller`'s `can_sweep`) against it:

```bash
stellar contract invoke --id <CONTRACT_ID> --network testnet \
  --source <ANY_FUNDED_TESTNET_KEY> -- can_sweep --ephemeral_account <ADDR>
```

If the ID does not resolve, the contract isn't live, for example because of a
testnet reset. Redeploy with `scripts/deploy-testnet.sh`.

## Automated deployment

There is **no automated testnet deployment** right now. The deploy steps in
`.github/workflows/deploy-testnet.yml` are commented out. Every testnet
deployment is a manual run of `scripts/deploy-testnet.sh`. The IDs change
every time someone runs it. See the [FAQ](./faq.md) for what this means for
stability.

## See also

- [Testnet FAQ](./faq.md)
- [Main README](../../README.md): MVP status, stub inventory, CI/CD notes
- [API Reference](../../docs/api-reference.md)
- [Security Model](../../docs/security.md)
- [Signature Format](../../docs/SIGNATURE_FORMAT.md)
