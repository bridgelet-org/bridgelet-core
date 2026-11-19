# Updating the Authorized Destination (`update_authorized_destination`)

Worked example for `SweepController::update_authorized_destination(new_destination)`
and the `DestinationUpdated` event it emits.

> **Status.** Derived from `contracts/sweep_controller/src/lib.rs`. The commands
> are a recipe, **not output captured from a live testnet run** (the contracts
> are not currently verified as deployed; see `testnet/registry/contract-status.md`).

## Who is authorized

Only the **creator** — the `creator` address passed to `SweepController::initialize`.
The function loads the stored creator and calls `creator.require_auth()`.

| Caller | Result |
|---|---|
| Creator (signing the transaction) | succeeds if no sweep has happened |
| Any other address | auth failure (the transaction does not carry the creator's authorization) |
| Controller never initialized | `AuthorizationFailed (3)` (no stored creator) |

## When it is allowed

Only **before the first sweep**. The function reads the sweep nonce; if it is
greater than `0` it returns `AccountAlreadySwept (7)`. After any successful
`execute_sweep`/`claim` the destination is frozen.

Note it works in both modes: if the controller was initialized without an
authorized destination (flexible mode), calling this **turns locked mode on**,
and the event's `old_destination` is `None`.

## Walkthrough

```bash
export NETWORK=testnet
export CREATOR="<identity whose address was passed as creator to initialize>"
export SWEEP_CONTROLLER_ID="<verified sweep controller id>"
export NEW_DEST="<G... new destination>"

# 0. Pre-check: no sweep has happened yet
stellar contract invoke --id "$SWEEP_CONTROLLER_ID" --source "$CREATOR" --network "$NETWORK" \
  --send=no -- get_nonce            # must be 0

# 1. Update
stellar contract invoke --id "$SWEEP_CONTROLLER_ID" --source "$CREATOR" --network "$NETWORK" \
  -- update_authorized_destination --new_destination "$NEW_DEST"
```

## Verify the `DestinationUpdated` event

The event topic is `dest_upd` (see `testnet/registry/event-topics.md`). Payload:

```rust
DestinationUpdated {
    old_destination: Option<Address>,   // None if the controller was in flexible mode
    new_destination: Address,
}
```

```bash
stellar contract events --id "$SWEEP_CONTROLLER_ID" --network "$NETWORK" \
  --start-ledger <ledger-of-update> --output pretty
```

Check: exactly one `dest_upd` event, `new_destination == NEW_DEST`, and
`old_destination` equal to the previous destination (or `None`).

## Negative checks

1. **Wrong caller:** invoke with `--source <some-other-identity>`; expect failure
   and no `dest_upd` event.
2. **After a sweep:** once `get_nonce` > 0, the same call returns
   `AccountAlreadySwept (7)`.

## Caveats

- The creator controls the destination until the first sweep, so treat the
  creator key as sensitive (`testnet/security/admin-key-hygiene.md`).
- The signed sweep message binds the destination, so signatures prepared for the
  old destination stop being usable once the lock changes.
