# Soroban CLI Quick Reference

Command shapes for the four Bridgelet contracts on Stellar testnet using
`stellar` 23.4.1. The flag and subcommand names below were checked against the
installed CLI's local `--help` output; the commands themselves were **not**
run against a live network while writing this page.

## Read this before copying a command

> The canonical [contract status registry](../registry/contract-status.md)
> records two TBD and two N/A IDs (EphemeralAccount and SweepController: `TBD`;
> ReserveContract and AccountFactory: `N/A`), all Last Verified `Never`. That
> table is stale relative to the current `scripts/deploy-testnet.sh`, which
> builds and deploys all four contracts. This page deliberately uses
> placeholders. Resolve and verify an ID with
> [verify-contract-live.md](../registry/verify-contract-live.md) before using
> it, and never treat a copied ID as proof of liveness.

`scripts/deploy-testnet.sh` currently calls `scripts/build.sh`, which builds the
whole workspace including `account_factory`, then deploys and initializes all
four contracts. Do not assume ReserveContract or AccountFactory are absent;
verify the ID and deployed interface on-chain before using the reference
shapes below.

Network values, set once per shell:

```bash
export NETWORK=testnet
export RPC_URL=https://soroban-testnet.stellar.org
export PASSPHRASE="Test SDF Network ; September 2015"
export READER=<FUNDED_TESTNET_IDENTITY>
export EA_ID=<VERIFIED_EPHEMERAL_ACCOUNT_ID>
export SC_ID=<VERIFIED_SWEEP_CONTROLLER_ID>
export RESERVE_ID=<VERIFIED_RESERVE_CONTRACT_ID>
export FACTORY_ID=<VERIFIED_ACCOUNT_FACTORY_ID>
```

The passphrase is exactly `Test SDF Network ; September 2015`, including the
spaces around `;`.

## Command shape and read-only posture

`stellar contract invoke` builds an implicit CLI from the deployed contract's
schema. Everything after `--` is a contract function and its named arguments.

```bash
stellar contract invoke \
  --id "$EA_ID" \
  --network "$NETWORK" \
  --source "$READER" \
  --send no \
  -- \
  get_info
```

| Form | Meaning |
|---|---|
| `--send no` | Simulate and return the result; do not sign or submit. Use for every read. `--is-view` is the deprecated alias. |
| no `--send` | Default: submit only when simulation shows ledger writes, events, or required auth. Use deliberately. |
| `--send yes` | Always submit, even for a pure read. Usually a waste of fees. |
| `-- --help` | Print the function list and argument names generated from that contract's schema. Use it when a deployed interface differs from this page. |
| `-- <fn> --<arg> <value>` | Pass a named argument after the function name. Argument names below come from the checked-in contract source. |

`--simulate-only` is **not** accepted by `stellar` 23.4.1; older docs in this
tree use it. Use `--send no` instead.

`stellar contract read` is a different tool: it reads a raw contract-data ledger
entry by storage key rather than calling a function. Prefer the contract's
getter functions; use `read` only for debugging.

## Function names by contract

These are the public functions in `contracts/*/src/lib.rs` in this checkout.
A deployed WASM may differ; confirm with
`stellar contract info interface --contract-id "$EA_ID" --network "$NETWORK"`
or `stellar contract invoke ... -- --help`.

| Contract | Read-only functions | State-changing functions |
|---|---|---|
| EphemeralAccount | `is_expired`, `get_status`, `get_info`, `get_info_paginated`, `get_payment_count`, `get_reserve_remaining`, `get_reserve_available`, `is_reserve_reclaimed`, `get_last_reserve_event`, `get_reserve_reclaim_event_count`, `simulate_sweep`, `simulate_sweep_paginated` | `initialize`, `record_payment`, `sweep`, `sweep_claim`, `expire`, `reclaim_reserve`, `recover`, `upgrade` |
| SweepController | `can_sweep`, `get_nonce` | `initialize`, `execute_sweep`, `claim`, `update_authorized_destination` |
| ReserveContract | `get_base_reserve`, `require_base_reserve`, `has_base_reserve`, `get_admin` | `initialize`, `set_base_reserve` |
| AccountFactory | `batch_initialize_count` | `initialize`, `batch_initialize`, `batch_initialize_paginated` |

Function names here are the exported names from the Rust source. For example,
the controller calls an abbreviated internal auth symbol for the claim path;
the contract function itself is `sweep_claim`, not `swp_claim`.

## EphemeralAccount

### Read-only

```bash
# Full state. Errors with NotInitialized on a fresh, uninitialized instance.
stellar contract invoke --id "$EA_ID" --network "$NETWORK" --source "$READER" \
  --send no -- get_info

# Cheaper probes. get_status returns Active (0) for an UNINITIALIZED instance,
# so it is not an initialization check; get_info is.
stellar contract invoke --id "$EA_ID" --network "$NETWORK" --source "$READER" \
  --send no -- get_status
stellar contract invoke --id "$EA_ID" --network "$NETWORK" --source "$READER" \
  --send no -- is_expired
stellar contract invoke --id "$EA_ID" --network "$NETWORK" --source "$READER" \
  --send no -- get_payment_count

# NO_CURSOR is 4294967295; the first page starts there.
stellar contract invoke --id "$EA_ID" --network "$NETWORK" --source "$READER" \
  --send no -- get_info_paginated \
  --params '{"limit": 10, "cursor_index": 4294967295}'

# Dry-run sweep check: returns the payments and an error code, changes nothing.
stellar contract invoke --id "$EA_ID" --network "$NETWORK" --source "$READER" \
  --send no -- simulate_sweep --destination "$DESTINATION"

# Reserve lifecycle reads.
stellar contract invoke --id "$EA_ID" --network "$NETWORK" --source "$READER" \
  --send no -- get_reserve_remaining
stellar contract invoke --id "$EA_ID" --network "$NETWORK" --source "$READER" \
  --send no -- get_reserve_available
stellar contract invoke --id "$EA_ID" --network "$NETWORK" --source "$READER" \
  --send no -- is_reserve_reclaimed
stellar contract invoke --id "$EA_ID" --network "$NETWORK" --source "$READER" \
  --send no -- get_last_reserve_event
stellar contract invoke --id "$EA_ID" --network "$NETWORK" --source "$READER" \
  --send no -- get_reserve_reclaim_event_count
```

### State-changing

```bash
# Five arguments in the current source. The changelog describes a future
# six-argument initialize with authorized_signer; verify the deployed schema
# before submitting. initialize can only be called once per instance.
stellar contract invoke --id "$EA_ID" --network "$NETWORK" --source "$CREATOR" \
  -- initialize \
  --creator "$CREATOR" \
  --expiry_ledger "$FUTURE_LEDGER" \
  --recovery_address "$RECOVERY" \
  --authorized_controller "$SC_ID" \
  --admin "$ADMIN"

# Records metadata only. It does not move tokens; amount is in base units.
stellar contract invoke --id "$EA_ID" --network "$NETWORK" --source "$CALLER" \
  -- record_payment --amount "$AMOUNT_BASE_UNITS" --asset "$ASSET_ID"

# Terminal and reserve paths. Each is irreversible for this instance.
stellar contract invoke --id "$EA_ID" --network "$NETWORK" --source "$CALLER" \
  -- expire
stellar contract invoke --id "$EA_ID" --network "$NETWORK" --source "$CALLER" \
  -- recover --caller "$CALLER"
stellar contract invoke --id "$EA_ID" --network "$NETWORK" --source "$CALLER" \
  -- reclaim_reserve

# Admin-gated code change. See runbooks/upgrade-verification.md first.
stellar contract invoke --id "$EA_ID" --network "$NETWORK" --source "$ADMIN" \
  -- upgrade --new_wasm_hash "$NEW_WASM_HASH_HEX"
```

Do **not** call `sweep` or `sweep_claim` directly in an integration.
`EphemeralAccount::sweep` accepts a signature but does not verify it
cryptographically, and both paths depend on the controller's authorization. Use
`SweepController::execute_sweep` or `::claim` instead; see
[unverified-signature-path-warning.md](../security/unverified-signature-path-warning.md)
and the [FAQ](faq.md).

## SweepController

### Read-only

```bash
# get_nonce is the authority for signing; never use a local counter as a
# substitute. A successful 0 can also mean an uninitialized controller.
stellar contract invoke --id "$SC_ID" --network "$NETWORK" --source "$READER" \
  --send no -- get_nonce

# Readiness probe for one account instance.
stellar contract invoke --id "$SC_ID" --network "$NETWORK" --source "$READER" \
  --send no -- can_sweep --ephemeral_account "$EA_ID"
```

### State-changing

```bash
# Omit --authorized_destination for unlocked mode. Pass a G... address to
# restrict every sweep to that destination.
stellar contract invoke --id "$SC_ID" --network "$NETWORK" --source "$CREATOR" \
  -- initialize \
  --creator "$CREATOR" \
  --authorized_signer "$SIGNER_PUBLIC_KEY_HEX"

# Signature is 64 bytes/128 hex characters, built from the CURRENT get_nonce,
# destination, and this controller ID. See SIGNATURE_FORMAT.md.
stellar contract invoke --id "$SC_ID" --network "$NETWORK" --source "$RELAYER" \
  -- execute_sweep \
  --ephemeral_account "$EA_ID" \
  --destination "$DESTINATION" \
  --auth_signature "$SIGNATURE_HEX"

# Gas-free claim: the recipient must supply Soroban auth; most integrators
# submit this path through an SDK rather than a plain CLI invocation.
stellar contract invoke --id "$SC_ID" --network "$NETWORK" --source "$RELAYER" \
  -- claim --recipient "$RECIPIENT" --ephemeral_account "$EA_ID"

# Creator-only, and only while nonce == 0.
stellar contract invoke --id "$SC_ID" --network "$NETWORK" --source "$CREATOR" \
  -- update_authorized_destination --new_destination "$DESTINATION"
```

## ReserveContract

> The canonical registry still records ReserveContract with an `N/A` ID and
> Last Verified `Never`; it is stale relative to the current deploy script,
> which builds, deploys, and initializes it. It is not wired into
> `EphemeralAccount`; it is a standalone admin-set store. Verify the ID
> on-chain, then use these shapes.

```bash
# Read-only.
stellar contract invoke --id "$RESERVE_ID" --network "$NETWORK" \
  --source "$READER" --send no -- get_base_reserve
stellar contract invoke --id "$RESERVE_ID" --network "$NETWORK" \
  --source "$READER" --send no -- require_base_reserve
stellar contract invoke --id "$RESERVE_ID" --network "$NETWORK" \
  --source "$READER" --send no -- has_base_reserve
stellar contract invoke --id "$RESERVE_ID" --network "$NETWORK" \
  --source "$READER" --send no -- get_admin

# Admin-only state changes.
stellar contract invoke --id "$RESERVE_ID" --network "$NETWORK" \
  --source "$ADMIN" -- initialize --admin "$ADMIN"
stellar contract invoke --id "$RESERVE_ID" --network "$NETWORK" \
  --source "$ADMIN" -- set_base_reserve --amount "$AMOUNT_STROOPS"
```

## AccountFactory

> The canonical registry still records AccountFactory with an `N/A` ID and
> Last Verified `Never`; it is stale relative to the current deploy script,
> which builds, deploys, and initializes it with the EphemeralAccount WASM
> hash. `batch_initialize` also discards per-account error detail
> (`error: None`). See
> [account-factory-batch-failure.md](../runbooks/account-factory-batch-failure.md).

```bash
# Counting is read-only: it does not create accounts.
stellar contract invoke --id "$FACTORY_ID" --network "$NETWORK" \
  --source "$READER" --send no -- batch_initialize_count \
  --requests '[{"expiry_ledger": <FUTURE_LEDGER>, "recovery_address": "<RECOVERY_ADDRESS>"}]'

# State-changing. Requests contain only expiry_ledger and recovery_address.
stellar contract invoke --id "$FACTORY_ID" --network "$NETWORK" \
  --source "$CREATOR" -- initialize \
  --ephemeral_account_wasm_hash "$EPHEMERAL_WASM_HASH_HEX"

stellar contract invoke --id "$FACTORY_ID" --network "$NETWORK" \
  --source "$CREATOR" -- batch_initialize \
  --creator "$CREATOR" \
  --requests '[{"expiry_ledger": <FUTURE_LEDGER>, "recovery_address": "<RECOVERY_ADDRESS>"}]'

# Prefer paginated batches. Resume with the returned next_cursor_index;
# 4294967295 means no more pages.
stellar contract invoke --id "$FACTORY_ID" --network "$NETWORK" \
  --source "$CREATOR" -- batch_initialize_paginated \
  --creator "$CREATOR" \
  --requests '[{"expiry_ledger": <FUTURE_LEDGER>, "recovery_address": "<RECOVERY_ADDRESS>"}]' \
  --params '{"limit": 10, "cursor_index": 4294967295}'
```

## Raw storage read

`stellar contract read` reads a ledger entry, not a function result. The key is
base64-encoded XDR for `DataKey::Status` in the current EphemeralAccount
storage layout; see [reading-contract-storage.md](../examples/reading-contract-storage.md)
for how that key is derived and its limitations.

```bash
stellar contract read \
  --id "$EA_ID" \
  --key-xdr "AAAAEAAAAAEAAAAPAAAABlN0YXR1cwAA" \
  --durability persistent \
  --network "$NETWORK" \
  --output json
```

If the deployed storage layout changed, this key is wrong. Compare with
`get_status` and investigate the deployed version rather than guessing a new
key.

## See also

- [getting-started.md](getting-started.md): first `initialize` and
  `record_payment` walkthrough
- [network-config.md](network-config.md): endpoints and network settings
- [troubleshooting-index.md](troubleshooting-index.md): symptom-based runbook
  routing
- [troubleshooting table](getting-started.md#troubleshooting): common error
  codes
- [SIGNATURE_FORMAT.md](../../docs/SIGNATURE_FORMAT.md): sweep signature bytes
- [event-topics.md](../registry/event-topics.md): event symbols for indexers

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-11-18 | Replaced the thin stub with per-contract read-only and state-changing command shapes for stellar 23.4.1 |
| 1.1 | 2026-11-18 | Corrected deploy-script coverage and stale-registry wording for all four contracts |
