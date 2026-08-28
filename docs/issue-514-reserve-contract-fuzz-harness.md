# Fuzz Testing Harness Plan — ReserveContract

Closes #514.

## Target entry points

`contracts/reserve_contract/src/lib.rs`:

- `set_base_reserve(env, amount: i128) -> Result<(), Error>` — the only
  state-mutating numeric entry point. Validation today is `0 < amount <=
  MAX_RESERVE_STROOPS` (`100_000_000_000`).
- `get_base_reserve` / `require_base_reserve` — read paths that must never
  panic regardless of what was previously stored.

## Adversarial input corpus

The fuzz harness (`contracts/reserve_contract/tests/fuzz.rs`, not added in
this doc-only PR) should generate `i128` values including:

- `0`, `-1`, `i128::MIN`, `i128::MAX`
- `MAX_RESERVE_STROOPS` and `MAX_RESERVE_STROOPS ± 1` (boundary)
- Values one stroop below/above zero and below/above the ceiling
- Randomly sampled values across the full `i128` range, biased toward the
  boundaries above (standard fuzz corpus seeding)

## Assertions per input

1. `set_base_reserve` returns `Err(InvalidAmount)` for every `amount <= 0`.
2. `set_base_reserve` returns `Err(AmountTooLarge)` for every `amount >
   MAX_RESERVE_STROOPS`.
3. For any accepted amount, `get_base_reserve` afterward returns exactly
   that amount (no truncation/overflow in the storage round-trip).
4. No input causes an unhandled panic — every rejection path returns a
   typed `Error`, never a raw abort.
5. Repeated `set_base_reserve` calls with alternating valid/invalid amounts
   never leave the previous valid value corrupted after a rejected call.

## Tooling

Use `proptest` (already idiomatic for Soroban contracts in this org) rather
than `cargo-fuzz`, since the calculation surface is pure and doesn't need
coverage-guided byte-level fuzzing. Recommended: 10,000+ cases per CI run,
gated to a scheduled (nightly) workflow rather than every PR given runtime
cost, per the issue's acceptance criteria.

## Follow-up process

Any panic or incorrect-but-non-panicking result discovered by the harness
gets filed as its own issue referencing the specific failing input,
reduced to a minimal reproducer via `proptest`'s built-in shrinking.
