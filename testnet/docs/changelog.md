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

**Breaking changes:** none.

**Other changes (EphemeralAccount):** the installed WASM
(`5e667ea0…c58e`) was built at `741aec2`. Since then `main` has added three
read-only functions that are **not** callable on testnet yet:
- `get_info_paginated(params: PaginationParams)`
- `get_payment_count() -> u32`
- `simulate_sweep_paginated(...)`

`initialize`, `record_payment` and the error codes (1–15) are unchanged.

**Action required when this ships:** none, unless you want to use the new
functions.

---

## 2026-10-10: SweepController, ReserveContract and AccountFactory redeployed

Deployed commit: `e7efd55`

**Contract IDs:** three new IDs. EphemeralAccount was **not** redeployed;
its ID and WASM hash are unchanged.

| Contract | Old ID (no longer used) | New ID |
|---|---|---|
| EphemeralAccount | — | `CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3` (unchanged) |
| SweepController | `CBEU4X5MNGOECBSTNEUFMBALH2YI5YV4UIH7YXRNOLR2DNLZQD4Z5KWE` | `CAJB4RRM5JRPXLITC5JUA7Q3LU2WQJDRPXVKMORXWHJJBPZRYWDJ5V4S` |
| ReserveContract | `CACYMICFLHSPKVMCK336IEGCWAQB4XBC7P7HTJSQY3R7Y2YASE4AX3MS` | `CBG3Y4GWFTL4D37JGV2XWAWC7EUQ7XGH4XM3SFLQYYSKPB2IYMAS54HY` |
| AccountFactory | `CARBMZY4466SWP3RTN3DR2F4JYNR5UCXD2ICFISV2YI4TS3AAZV2TQ24` | `CDBY4L3NTLW4OBZZTY73VL3NNWO2AGFFGBRJVDL3IMKDDLFF2W57WK6H` |

**Breaking changes:** the old SweepController, ReserveContract and
AccountFactory IDs are no longer the Bridgelet deployment. Any code or
config that hard-codes them must be updated. The public signatures of
`SweepController::initialize`, `execute_sweep` and `claim` are unchanged.

**Other changes:**
- AccountFactory: new `batch_initialize_paginated(...)` and
  `batch_initialize_count(requests) -> u32`.
- SweepController: the public `execute_transfers` helper was removed;
  transfers now happen inside `execute_sweep` / `claim`.

**Action required:** replace the three old IDs with the new ones listed in
[network-config.md](network-config.md). If you set an EphemeralAccount's
`authorized_controller` to the old SweepController ID, deploy a new
instance pointing at the new ID.

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
