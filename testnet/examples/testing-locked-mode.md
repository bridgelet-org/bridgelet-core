# Testing `SweepController` Locked Mode

`SweepController::initialize` takes an optional `authorized_destination`. If it
is `Some(addr)` the controller is in **locked mode**: `execute_sweep` and
`claim` accept only that destination. If it is `None` (flexible mode) any
destination is accepted.

> **Status.** The behaviour below is read from `contracts/sweep_controller/src/lib.rs`
> (`initialize`, `validate_destination`). The commands are a recipe, **not
> output captured from a live run**; per `testnet/registry/contract-status.md`
> the contracts must be (re)deployed and verified first. Replace every
> `<placeholder>` and verify IDs before running.

## Prerequisites

- Network settings from `testnet/docs/network-config.md`.
- A funded identity (`stellar keys fund <name> --network testnet`).
- A built `sweep_controller.wasm` and `ephemeral_account.wasm` (`scripts/build.sh`).
- Two destination addresses: `LOCKED_DEST` (will be authorized) and `OTHER_DEST`
  (must be rejected).

```bash
export NETWORK=testnet
export CREATOR="<funded-identity>"          # also needs to be able to sign for initialize
export LOCKED_DEST="<G... authorized destination>"
export OTHER_DEST="<G... a different destination>"
export SIGNER_PUBKEY="<64 hex chars: Ed25519 public key (never the seed)>"
```

## 1. Deploy and initialize in locked mode

```bash
SWEEP_CONTROLLER_ID=$(stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/sweep_controller.wasm \
  --source "$CREATOR" --network "$NETWORK")

stellar contract invoke --id "$SWEEP_CONTROLLER_ID" --source "$CREATOR" --network "$NETWORK" \
  -- initialize \
  --creator "$(stellar keys address "$CREATOR")" \
  --authorized_signer "$SIGNER_PUBKEY" \
  --authorized_destination "$LOCKED_DEST"
```

Expected: success, plus a `dest_auth` event carrying `DestinationAuthorized { destination }`.
A second `initialize` call fails with `AuthorizationFailed (3)`.

## 2. Confirm the lock exists

```bash
stellar contract events --id "$SWEEP_CONTROLLER_ID" --network "$NETWORK" \
  --start-ledger <deploy-ledger> --output pretty
```

Look for one `dest_auth` event with `LOCKED_DEST`.

## 3. Attempt a sweep to a different destination

Use an `EphemeralAccount` that has recorded a payment (see
`testnet/examples/signed-sweep-walkthrough.md`), and a signature produced for
`OTHER_DEST` and the controller's **current** nonce (`get_nonce`).

```bash
stellar contract invoke --id "$SWEEP_CONTROLLER_ID" --source "$CREATOR" --network "$NETWORK" \
  -- execute_sweep \
  --ephemeral_account "$EPHEMERAL_ACCOUNT_ID" \
  --destination "$OTHER_DEST" \
  --auth_signature "<128 hex chars signed for OTHER_DEST>"
```

**Expected: rejection with `UnauthorizedDestination (13)`.** `validate_destination`
runs **before** signature verification, so the rejection does not depend on the
signature being valid, and no state changes.

## 4. Confirm nothing changed

```bash
stellar contract invoke --id "$SWEEP_CONTROLLER_ID" --source "$CREATOR" --network "$NETWORK" \
  --send=no -- get_nonce            # unchanged
stellar contract invoke --id "$EPHEMERAL_ACCOUNT_ID" --source "$CREATOR" --network "$NETWORK" \
  --send=no -- get_status           # still PaymentReceived (1)
```

## 5. Positive control

Repeat step 3 with `--destination "$LOCKED_DEST"` and a signature for
`LOCKED_DEST`: the sweep should succeed, the nonce increments to 1, and `sweep`
(controller) and `swept_mul` (account) events appear.

The same lock applies to `claim`: `claim(recipient, ephemeral_account)` with a
`recipient` other than `LOCKED_DEST` is rejected with `UnauthorizedDestination (13)`.

## Notes and pitfalls

- Locked mode restricts **where** funds can go, not **who** may call. See
  `testnet/security/unverified-signature-path-warning.md`.
- The lock can be changed by the creator until the first sweep:
  see `testnet/examples/update-authorized-destination-flow.md`.
- Error code `13` is deliberate in the source; there is no code `12` in
  `sweep_controller/src/errors.rs`.
