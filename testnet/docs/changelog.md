# Testnet Changelog

This log lists changes that are visible on the **Bridgelet testnet
deployment**: new contract IDs, redeployments, changed function signatures,
new error codes, and testnet resets. If you integrate against testnet, this
is the only file you need to watch.

It is separate from any project-wide changelog. It does not record source
changes that haven't been deployed to testnet.

Current IDs and endpoints are listed in [network-config.md](network-config.md).

## How to update this file

Add an entry at the top every time `scripts/deploy-testnet.sh` is run
against testnet, every time a contract is upgraded in place, and every time
an SDF testnet reset takes the deployment down. Each entry should include:

- **Date** (UTC, `YYYY-MM-DD`) and the git commit that was deployed
- **Contract IDs.** List new IDs, or write "unchanged".
- **Breaking changes.** Anything that makes existing integrator code fail,
  such as changed signatures, removed functions or new required arguments.
- **Other changes.** New functions, new error codes, behavior fixes.
- **Action required.** What integrators need to do, or "none".

Also update `network-config.md` and `deployment-artifacts/contract-ids.txt`
in the same PR.

Entry template:

```markdown
## YYYY-MM-DD — <short title>

Deployed commit: `<sha>`

**Contract IDs:** unchanged | new (see below)
**Breaking changes:** none | …
**Other changes:** …
**Action required:** none | …
```

---

## Unreleased: not yet on testnet

These changes are on `main` but **not** in the deployed testnet contracts.
When they ship, they will move into a dated entry.

**Breaking changes (EphemeralAccount):**
- `initialize` gains an `authorized_signer: BytesN<32>` argument, between
  `authorized_controller` and `admin`, for a total of six arguments. Existing
  five-argument calls will fail after redeployment.
- `initialize` enforces a network-passphrase check. On `main` the expected
  value is currently the Standalone passphrase, which would reject testnet
  calls with `Error(Contract, #1007)`. This must be set to the Testnet
  passphrase before the next testnet deploy.

**Other changes (EphemeralAccount):**
- New read-only function `is_initialized() -> bool`.
- Error codes are namespaced per contract: EphemeralAccount uses
  1000–1999, SweepController 2000–2999, and ReserveContract 3000–3999.

**Action required when this ships:** update your `initialize` calls to pass
`authorized_signer`, and pick up any new contract IDs from
`network-config.md`.

---

## 2026-07-12: Initial testnet deployment

Deployed commit: `741aec2`

**Contract IDs:**

| Contract | Contract ID |
|---|---|
| EphemeralAccount | `CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3` |
| SweepController | `CBEU4X5MNGOECBSTNEUFMBALH2YI5YV4UIH7YXRNOLR2DNLZQD4Z5KWE` |
| ReserveContract | `CACYMICFLHSPKVMCK336IEGCWAQB4XBC7P7HTJSQY3R7Y2YASE4AX3MS` |
| AccountFactory | `CARBMZY4466SWP3RTN3DR2F4JYNR5UCXD2ICFISV2YI4TS3AAZV2TQ24` |

EphemeralAccount WASM hash: `5e667ea0687341bdccc81538492143b573777dd04b2450cdeb89b02cee62c58e`

**Interface as deployed:**
- `EphemeralAccount::initialize(creator, expiry_ledger, recovery_address, authorized_controller, admin)`
- `EphemeralAccount::record_payment(amount: i128, asset: Address)`
- `SweepController::initialize(creator, authorized_signer: BytesN<32>, authorized_destination: Option<Address>)`
- `SweepController::execute_sweep(ephemeral_account, destination, auth_signature: BytesN<64>)`

**Notes:**
- The shared EphemeralAccount instance is already initialized, and its
  expiry ledger has passed. To test, deploy your own instance from the WASM
  hash; see [getting-started.md](getting-started.md).

**Action required:** none (first release).
