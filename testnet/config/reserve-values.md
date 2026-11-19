# Which Base Reserve Value Is in Effect on Testnet

**Short answer:** for reserve-reclaim behavior on testnet today, the only value
that matters is the constant compiled into **EphemeralAccount**:

```rust
// contracts/ephemeral_account/src/lib.rs
const BASE_RESERVE_STROOPS: i128 = 1_000_000_000; // 100 XLM
```

**`ReserveContract::set_base_reserve` has no effect on EphemeralAccount.**
EphemeralAccount never calls or reads ReserveContract. Changing the stored
value in ReserveContract won't change what an ephemeral account tracks,
reclaims, or reports in its events.

This page explains exactly what happens, so that reserve-reclaim tests assert
the right numbers.

---

## The two values

| | EphemeralAccount `BASE_RESERVE_STROOPS` | ReserveContract base reserve |
|---|---|---|
| Where | Compile-time constant in `contracts/ephemeral_account/src/lib.rs` | Instance storage of the deployed ReserveContract |
| Value | `1_000_000_000` stroops (100 XLM) | Whatever the admin last set; `None` until set |
| How it changes | Only by deploying or upgrading EphemeralAccount with a new WASM | `set_base_reserve(amount)`, admin auth, `0 < amount ≤ 100,000,000,000` |
| Read by | EphemeralAccount itself, once, in `initialize` | Nothing on-chain. Only off-chain callers of `get_base_reserve` / `has_base_reserve` read it. |
| **Affects reserve reclaim?** | **Yes** | **No** |

The repository [README](../../README.md) lists this duplication as a known
issue ("Reserve tracking is duplicated across two contracts"). Nothing in the
current source wires ReserveContract into EphemeralAccount or AccountFactory;
neither contract references `reserve_contract` at all.

---

## What EphemeralAccount actually does

1. **`initialize`** calls `init_reserve_tracking(BASE_RESERVE_STROOPS)`, which
   sets:
   - `get_reserve_remaining` → `1000000000`
   - `get_reserve_available` → `1000000000`
   - `is_reserve_reclaimed` → `false`
   - `get_reserve_reclaim_event_count` → `0`
2. **The first sweep or expiry**, through `sweep` (via
   `SweepController::execute_sweep`), `sweep_claim` (via
   `SweepController::claim`), or `expire`, reclaims `min(available, remaining)`.
   With nothing else changing those values, that is the full `1000000000`.
   Afterwards:
   - `get_reserve_remaining` → `0`
   - `get_reserve_available` → `0`
   - `is_reserve_reclaimed` → `true`
   - one `ReserveReclaimed` event with `amount = 1000000000`,
     `fully_reclaimed = true`, `remaining_reserve = 0`
   - for `expire`, the `AccountExpired` event's `reserve_amount` is also
     `1000000000`
3. **Later `reclaim_reserve` calls** succeed and return `0`. Each one still
   emits a `ReserveReclaimed` event with `amount = 0` and increments
   `get_reserve_reclaim_event_count`.

### Reclaim is bookkeeping, not a transfer

`reclaim_reserve_to` updates the contract's own counters and emits events. It
does **not** move XLM or any other token. On Stellar, the real reserve sits on
the ephemeral account's classic `G...` address, so getting it back is up to the
SDK or off-chain operator (for example with a classic `account_merge` or
payment). The `1000000000` in the events is the contract's *bookkeeping*
figure.

It also doesn't match the network's real minimum balance. Testnet's base
reserve is 0.5 XLM per ledger entry, and a basic account needs 1 XLM. Don't
reconcile the event amount against actual balance changes on the `G...`
address; they are separate numbers.

---

## Testing reserve reclaim on testnet

Assert against the EphemeralAccount getters, not ReserveContract:

```bash
EA=<EPHEMERAL_ACCOUNT_CONTRACT_ID>
SRC=testnet-identity

# Right after initialize
stellar contract invoke --id "$EA" --network testnet --source "$SRC" --send no -- get_reserve_remaining   # 1000000000
stellar contract invoke --id "$EA" --network testnet --source "$SRC" --send no -- is_reserve_reclaimed    # false

# After a sweep, claim or expire
stellar contract invoke --id "$EA" --network testnet --source "$SRC" --send no -- get_reserve_remaining   # 0
stellar contract invoke --id "$EA" --network testnet --source "$SRC" --send no -- is_reserve_reclaimed    # true
stellar contract invoke --id "$EA" --network testnet --source "$SRC" --send no -- get_last_reserve_event
```

### Pitfalls

- **Don't** call `ReserveContract::set_base_reserve` in your test setup and
  expect `get_reserve_remaining` or the `ReserveReclaimed.amount` to follow it.
  They won't.
- **Don't** treat `ReserveContract::get_base_reserve` returning `None` (or any
  other value) as a sign that EphemeralAccount has no reserve. EphemeralAccount
  always starts at `1000000000`.
- If you need a different reserve figure for a test, that means changing the
  constant and deploying or upgrading EphemeralAccount. On testnet that is a
  deployer operation, not something an integrator can configure.

---

## Confirming this against the deployed WASM

This page describes the source on `main`. The value is compiled in, so a
testnet deployment built from a different commit could differ. To confirm the
live behavior, initialize a fresh ephemeral account on testnet and check that
`get_reserve_remaining` returns `1000000000` before any sweep. If it returns
something else, the deployed WASM differs from `main`; see
[`../registry/wasm-hash-reference.md`](../registry/wasm-hash-reference.md).

ReserveContract's stored value can be read at any time. It is informational
only:

```bash
stellar contract invoke --id <RESERVE_CONTRACT_ID> --network testnet --source testnet-identity --send no -- get_base_reserve
```

---

## If this changes

If a future change makes EphemeralAccount (or AccountFactory) read
ReserveContract, for example at `initialize`, update this page, the README's
known-issues entry, and [`../docs/changelog.md`](../docs/changelog.md) in the
same change. The [reserve contract status](../registry/reserve-contract-status.md)
page tracks the ReserveContract deployment itself.
