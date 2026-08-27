# Event Emission Completeness Audit (#510)

## Method
Reviewed `env.events().publish(...)` call sites in `contracts/ephemeral_account/src/events.rs`,
`contracts/sweep_controller/src/lib.rs`, and `contracts/reserve_contract/src/lib.rs` against every
state-changing public function in the same modules.

## Findings

### ephemeral_account (events.rs)
State transitions covered: `AccountCreated` (initialize), `PaymentReceived` /
`MultiPaymentReceived` (deposit handling), `SweepExecutedMulti` (sweep), `AccountExpired`
(expiry reclaim), `ReserveReclaimed` (reserve return). All five mutating paths that change
`AccountStatus` or move funds emit a matching event — **no gap found** in this contract.

### sweep_controller (lib.rs)
`emit_sweep_completed`, `emit_sweep_executed_multi`, `emit_destination_authorized`,
`emit_destination_updated` cover the sweep and destination-config paths. `update_authorized_signer`
(the timelocked signer-rotation function, lib.rs:417) was checked for an emitted event at both the
"initiate" and "take-effect" points — only the initiation path was confirmed to emit; the
take-effect ledger boundary should be re-verified against the full diff, since a silent rotation
completion would be invisible to the off-chain indexer.

### reserve_contract (lib.rs)
`emit_initialized` and `emit_base_reserve_updated` cover both mutating functions
(`initialize`, `set_base_reserve`). `set_base_reserve` includes the old and new value in its event
payload, which is good practice for indexer diffing — **no gap found**.

## Cross-reference with bridgelet-sdk
The `contract_events` table / webhook consumer schema was not directly accessible from this repo;
this audit could confirm emission from the contract side but not final field-name parity with the
SDK's indexer. Flagging as a follow-up dependency rather than closing that half of the acceptance
criteria silently.

## Recommendation
1. Confirm event emission on the timelock take-effect branch of `update_authorized_signer`.
2. Do a joint schema diff against `bridgelet-sdk`'s webhook event list in a follow-up PR once both
   repos can be checked out together.
