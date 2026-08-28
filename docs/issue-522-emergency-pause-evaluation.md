# ADR: Emergency Pause Mechanism (Issue #522)

## Status
Decided (evaluation only — no code change in this document).

## Context
None of `reserve_contract`, `ephemeral_account`, or `sweep_controller` currently
expose a pause/circuit-breaker path. A `CircuitBreaker`-style contract has been
proposed separately but not implemented. This ADR decides *where* pause state
should live before that contract is built.

## Options Considered
1. **In-contract pause flag** — each contract stores its own `paused: bool`,
   checked at the top of state-changing entry points (`set_base_reserve`,
   `record_payment`, `sweep`, `sweep_claim`, `expire`, `recover`).
2. **External circuit-breaker contract** — every guarded call performs a
   cross-contract read (`env.invoke_contract`) against a shared
   `CircuitBreaker` contract before proceeding.
3. **Both** — a global breaker for platform-wide halts, plus local flags for
   per-contract incident response.

## Decision
Adopt **option 3, in-contract flag first, external breaker layered on later**:
- Each contract gets its own `paused` flag, gated by the same admin address
  already required by `set_admin`/`initialize` flows (no new trust surface).
- The external `CircuitBreaker` contract, when implemented, becomes an
  *additional* pre-check that contracts may opt into for platform-wide halts
  (e.g. a detected exploit in a shared dependency), not a replacement.

## Tradeoffs
- **In-contract only**: cheapest (one storage read, no cross-contract call),
  but requires N separate admin transactions to pause N contracts during an
  incident.
- **External breaker only**: one transaction pauses everything, but adds a
  cross-contract call (`invoke_contract`) — and its associated CPU/resource-fee
  cost — to *every* guarded call, even when nothing is paused.
- **Both**: local flag covers the common case cheaply; the external check is
  reserved for contracts where platform-wide blast radius matters
  (`reserve_contract`, `sweep_controller`), keeping the fee overhead scoped
  rather than applied everywhere by default.

## Consequence
This decision should drive the `CircuitBreaker` contract's interface (a simple
`is_paused(caller: Address) -> bool` read function callable cross-contract),
not the other way around, per the issue's acceptance criteria.
