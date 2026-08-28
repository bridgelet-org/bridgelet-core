# Issue #506: SweepController Cross-Contract Call Safety Audit

Scope: every `SweepController` function that both mutates storage and calls
another contract (`contracts/sweep_controller/src/lib.rs`).

## Functions reviewed

| Function | External call | Ordering |
| --- | --- | --- |
| `execute_sweep` -> `sweep_account` | `EphemeralAccountClient::sweep()` | Nonce incremented (`authorization::increment_nonce`) **before** the cross-contract call, per the doc comment on `increment_nonce()`: "Must be called *after* `verify_sweep_auth()` succeeds but *before* any external contract calls, so that a re-entrant call within the same transaction would see the incremented nonce and fail." Token transfers (`transfers::execute_transfers`) happen only *after* the external `sweep()` call returns and `info.payment_received`/`amount == 0` are checked. |
| `claim` | `EphemeralAccountClient::get_info()`, `sweep_claim()` | Nonce is incremented first (line ~150, comment references issue #410), then `authorize_claim` invokes the account contract. |
| `can_sweep` | `get_info()` | Read-only, no state mutation before or after the call. |
| `get_reserve_info` | 3 read-only calls | Read-only, no state mutation. |
| `fee_estimate` | `is_initialized()`, `get_info()` | Read-only, no state mutation. |

## Findings

1. **Checks-effects-interactions is followed.** The replay-prevention nonce
   (the only piece of controller state an external call could exploit) is
   always written before the cross-contract invocation in both `sweep_account`
   and `claim`. A hypothetical re-entrant call back into `execute_sweep`
   during `account_client.sweep()` would see the already-incremented nonce
   and fail signature verification in `authorization::verify_sweep_auth`.
2. **Post-call validation is present.** `sweep_account` checks
   `info.payment_received` and `amount == 0` *after* the cross-contract call
   returns, before executing token transfers, so a misbehaving/no-op
   `EphemeralAccount` cannot cause funds to move without a validated payment
   record.
3. **No exploitable ordering issue found.** Soroban's single-threaded WASM
   execution (no callbacks/fallbacks) further limits reentrancy exposure
   beyond what EVM-style CEI analysis alone would show; see
   `docs/reentrancy-analysis.md` for the underlying runtime guarantees.
4. **Gap:** `update_authorized_destination` does not call any external
   contract, so it is out of scope, but note it does not re-check the nonce
   read at call time against a fresh read after `require_auth()` — not an
   issue today since `require_auth()` itself has no reentrancy surface.

## Conclusion

No exploitable reentrancy/ordering issue identified in `SweepController`.
Existing nonce-before-call ordering already satisfies the acceptance
criteria's "no state left inconsistent" requirement. No contract changes
required as a result of this audit; recommend keeping the ordering comment
on `increment_nonce()` as the canonical rationale for future reviewers.
