# Bridgelet Core Testnet FAQ

Quick answers for people using bridgelet-core on Stellar **testnet**. Every
answer comes from the [main README](../../README.md) and the repo's existing
deployment files. For contract-by-contract deployment status, see the
[Testnet Overview](./overview.md).

---

## Contracts and deployment

### Which contracts can I actually call on testnet?

Only **`ephemeral_account`** and **`sweep_controller`**. They are the only two
contracts that `scripts/deploy-testnet.sh` deploys, and `sweep_controller` is
the only one it initializes. `reserve_contract` and `account_factory` are not
part of the supported testnet deployment. See the [overview table](./overview.md#tldr).

### Is `reserve_contract` live on testnet?

**Not as part of the supported deployment.** The deploy script never deploys
it. The script does echo a `RESERVE_CONTRACT_ID` variable, but it never sets
it, and the README lists this as a fix needed before the script is
production-usable. `deployment-artifacts/contract-ids.txt` does list a
reserve contract ID, but that file was committed by hand and the current
tooling can't reproduce it. Check that ID on-chain before you use it.

Even where it is deployed, `reserve_contract` is **not wired into
`ephemeral_account`** yet. It is a standalone store for an admin-set
base-reserve amount (max 10,000 XLM).

### Is `account_factory` live on testnet?

Same answer as `reserve_contract`: the script doesn't deploy it, and the only
ID you'll find is the unverified hand-committed one in `contract-ids.txt`. It
also needs a stored `ephemeral_account` WASM hash, and the tooling doesn't set
that up.

### Where do I find the contract IDs?

- **`deployments/testnet.json`**: written by `deploy-testnet.sh`. It has
  `ephemeralAccount` and `sweepController`. The committed copy is a
  placeholder with `null` values, so you get real IDs after you run the script.
- **`deployment-artifacts/contract-ids.txt`**: four IDs committed by hand.
  Treat them as hints, not guarantees.

Testnet is reset from time to time, so check any ID before you rely on it
(see [overview → Where contract IDs live](./overview.md#where-contract-ids-live-and-how-far-to-trust-them)).

### `docs/testnet-deployment.md` says the script deploys ReserveContract and AccountFactory. Which is right?

The **script** is right. `scripts/deploy-testnet.sh` only deploys
`ephemeral_account` and `sweep_controller`. That guide describes where the
tooling is meant to end up, not what it does today.

---

## Sweeps and signatures

### Why does `sweep()` accept a signature parameter it doesn't check?

`EphemeralAccount::sweep(destination, auth_signature)` is a **partial stub**
(tracked in #86). The signature `BytesN<64>` stays in the interface so the
function signature won't change when real verification arrives. Today,
authorization uses Soroban's `require_auth()` on the authorized controller,
**not** `env.crypto().ed25519_verify()`. So whatever you pass as
`auth_signature` is accepted but never cryptographically verified.

The production fix is to verify an Ed25519 signature over
destination + nonce + contract_id against the stored `authorized_signer`,
the way `SweepController` already does.

### So how should I sweep funds on testnet?

**Always go through `SweepController`:**

- `SweepController::execute_sweep(ephemeral_account, destination, auth_signature)`
  does real Ed25519 verification with nonce replay protection, then runs
  atomic multi-asset SEP-41 transfers.
- `SweepController::claim(recipient, ephemeral_account)` is the experimental
  gas-free path. The recipient signs a Soroban auth entry, and a relayer
  submits the transaction and pays the fees.

Don't call `EphemeralAccount::sweep()` directly until the stub is replaced.

### What's the difference between `sweep` and `sweep_claim` on `ephemeral_account`?

`sweep_claim(destination)` has no signature parameter. It is the downstream
call that `SweepController::claim()` makes, using
`authorize_as_current_contract()` to satisfy the account's
`authorized_controller.require_auth()`. Integrators shouldn't call either
function directly.

### Is the deployed `sweep_controller` locked to a single destination?

No. The deploy script initializes it with `authorized_destination = null`,
which means **unlocked** mode: any destination is accepted if the Ed25519
signature from `authorized_signer` is valid. The creator can change this later
with `update_authorized_destination`.

### How do I format the signature for `execute_sweep`?

See [`docs/SIGNATURE_FORMAT.md`](../../docs/SIGNATURE_FORMAT.md).

---

## CI and stability

### Why is CI disabled? Does that affect testnet stability?

The README says CI/CD is "currently disabled". Today, the part that matters
for testnet is **`.github/workflows/deploy-testnet.yml`**: its deploy steps
are commented out, so **nothing deploys to testnet automatically on merge**.
(`.github/workflows/test.yml` has since been re-enabled and runs on pushes and
PRs to `main`.)

What this means for testnet users:

- **No surprise redeploys.** A merge to `main` doesn't replace the contracts
  you're integrating against. In that narrow sense, IDs are *more* stable.
- **No guarantee testnet matches `main`.** Deployments are manual runs of
  `scripts/deploy-testnet.sh`, so the code on testnet may be older than
  `main`, and nothing records which commit was deployed.
- **Every manual run creates new contract IDs.** If someone redeploys, or
  testnet is reset, your stored IDs stop working. Keep your IDs in config, not
  in code.
- The README's older "Automated Testnet Deployment" section (deploy on merge,
  90-day artifacts, deployment summary) is **aspirational**, not current
  behavior.

### Does `scripts/test.sh` test everything?

No. It only runs `cargo test` for `ephemeral_account`. To cover all four core
contracts:

```bash
for c in ephemeral_account sweep_controller reserve_contract account_factory; do
  (cd contracts/$c && cargo test)
done
(cd contracts/sweep_controller && cargo test --test integration)
```

---

## Other

### Is there a local sandbox or `test-local.sh` flow?

The README says the supported local entrypoint is `scripts/test.sh`, which
runs unit tests only and deploys nothing to a local sandbox. Use testnet for
anything that needs real deployment.

### What versions should I use?

Soroban SDK 22.0.0 and `stellar-cli` 23.4.1, with the
`wasm32-unknown-unknown` target. Binaryen (v100+) is optional and optimizes
the WASM.

### Where do I report a problem with the testnet deployment?

Open a GitHub issue on this repo. Include the contract ID, the method you
called, and the transaction hash. There is no `CONTRIBUTING.md` yet.
