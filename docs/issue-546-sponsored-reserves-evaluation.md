# Sponsored Reserves for Ephemeral Account Creation — Feasibility

Closes #546.

## Current cost model

`EphemeralAccountContract::initialize()` tracks a base reserve via
`storage::init_reserve_tracking(&env, base_reserve)`
(`ephemeral_account/src/lib.rs:86`), sourced from `BASE_RESERVE_STROOPS`
or `ReserveContract::get_base_reserve()` (`resolve_base_reserve()`,
line 521). This is contract-storage accounting only — the real XLM lock
happens at the classic-account level when the underlying Stellar account
is created/funded, outside these Soroban contracts.

## Feasibility of Stellar sponsored reserves

`BeginSponsoringFutureReserves` / `EndSponsoringFutureReserves` are
**classic transaction-envelope operations**, not Soroban host functions.
No `env.` API in `soroban-sdk` (as used throughout
`contracts/ephemeral_account` and `contracts/account_factory`) invokes
sponsorship — it operates at the classic ledger/operation level, prior to
and independent of any Soroban contract invocation.

**Constraint**: a Soroban contract cannot submit or wrap classic
sponsorship operations on another account's behalf. That must happen in
the transaction envelope constructed off-chain, sandwiching the classic
`CreateAccount`/`ChangeTrust` ops for the ephemeral account between
sponsorship begin/end ops, signed by both sponsor and new account.

## Conclusion: feasible, but at the SDK layer, not contracts/*

- Feasible: yes, at the transaction-construction layer in
  `bridgelet-sdk`; no `contracts/*` change is required or possible, since
  these contracts only observe/track the reserve figure and never
  construct classic operations.
- Before/after: today the funding account's XLM is locked (full base
  reserve) until `reclaim_reserve_to()` returns it post-sweep. With
  sponsorship, the funding account's balance is never debited for the
  reserve — the sponsor's liability drops to zero via
  `EndSponsoringFutureReserves` without a separate on-chain reclaim.

## Acceptance criteria status

- [x] Feasibility evaluated: yes, at the SDK/transaction-envelope layer.
- [x] Constraint documented: sponsorship is classic-only, not callable
      from a Soroban contract's `env`.
- [ ] Implementation belongs in `bridgelet-sdk`, out of scope for
      `contracts/*`.
