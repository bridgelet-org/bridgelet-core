<!--
Purpose: Checklist for ensuring test coverage is sufficient for all public entry points and error paths across Bridgelet Core contracts.
Owner: ameeribro4-sudo (closes #303).
Status: Documentation-only. No contracts, docs, scripts, or tools changes are introduced by this file.
-->

# Test-Coverage Completeness Checklist

> **Purpose.** Provide a structured checklist for verifying that every public entry point
> in Bridgelet Core has adequate test coverage — happy path, error paths, and edge cases —
> before signing off on a release.

| Field | Value |
| :--- | :--- |
| **Related issue** | [#303](https://github.com/bridgelet-org/bridgelet-core/issues/303) |
| **Owner / reviewer** | `_operator-name_` |
| **Last reviewed** | `_ISO-8601 date_` |

## Table of Contents

1. [SweepController Test Coverage](#sweepcontroller-test-coverage)
2. [EphemeralAccount Test Coverage](#ephemeralaccount-test-coverage)
3. [AccountFactory Test Coverage](#accountfactory-test-coverage)
4. [ReserveContract Test Coverage](#reservecontract-test-coverage)
5. [Error-Code Coverage](#error-code-coverage)
6. [Integration Test Health](#integration-test-health)
7. [Sign-Off](#sign-off)

---

## SweepController Test Coverage

| Public function | Happy path | Error paths | Edge cases | Status |
| :--- | :--- | :--- | :--- | :--- |
| `initialize` | `test_sweep_controller_creator_is_stored` | `test_initialize_prevents_double_init` | Nonce starts at 0 | ✅ Covered |
| `execute_sweep` | `test_execute_sweep_with_valid_signature` (integration) | `test_execute_sweep_unauthorized_signer_fails`, `test_sweep_account_not_ready_without_payment` | — | ⚠️ Partial (see below) |
| `claim` | `test_full_lifecycle_deploy_init_record_claim_verify_state` (integration) | `test_claim_rejects_unauthorized_recipient`, `test_double_claim_rejected` | Multi-asset, flexible destination | ✅ Covered |
| `can_sweep` | `test_can_sweep_returns_true_for_ready_account` | `test_can_sweep_returns_false_without_payment` | Active/PaymentReceived/Swept states | ✅ Covered |
| `get_nonce` | `test_get_nonce_initial` | — | — | ✅ Covered |
| `update_authorized_destination` | — | — | — | ❌ **Untested** |
| `update_authorized_signer` | — | — | — | ❌ **Untested** |
| `apply_signer_update` | — | — | — | ❌ **Untested** |
| `get_pending_signer_update` | — | — | — | ❌ **Untested** |
| `migrate` | — | — | — | ❌ **Untested** |
| `get_version` | — | — | — | ❌ **Untested** |
| `get_reserve_info` | — | — | — | ❌ **Untested** |
| `fee_estimate` | — | — | — | ❌ **Untested** |

**`execute_sweep` gaps:** The happy path is tested in integration, but the following
error paths are never exercised:
- `AccountExpired` (2005) — sweeping an expired account via Ed25519 path
- `AccountAlreadySwept` (2006) — double-sweep via Ed25519 path (only tested via `claim`)
- `InvalidSignature` (2007) — malformed signature bytes
- `SignatureVerificationFailed` (2008) — valid-length but wrong signature
- `InvalidNonce` (2010) — replay with stale nonce
- `NotAdmin` (2013) — non-admin calling admin-only functions

---

## EphemeralAccount Test Coverage

| Public function | Happy path | Error paths | Edge cases | Status |
| :--- | :--- | :--- | :--- | :--- |
| `initialize` | `test_initialize` | `test_initialize_requires_creator_authorization` | — | ✅ Covered |
| `record_payment` | `test_record_payment`, `test_multiple_payments` | 4 error-path tests | Duplicate asset, too many assets | ✅ Covered |
| `sweep` (Ed25519) | — | — | — | ❌ **Untested** |
| `sweep_claim` | `test_sweep_claim_authorized_controller_succeeds` | `test_sweep_returns_already_swept_error`, `test_sweep_after_expiry_is_rejected` | — | ✅ Covered |
| `expire` | — | `test_expire_returns_not_expired_error` | — | ⚠️ Partial (happy path untested directly) |
| `recover` | — | — | — | ⚠️ Integration-only |
| `reclaim_reserve` | — | — | — | ❌ **Untested** |
| `upgrade` | — | — | — | ❌ **Untested** |
| `simulate_sweep` | — | — | — | ❌ **Untested** |

**Critical gap:** The Ed25519-based `sweep()` function — the primary off-chain signer
entry point — has zero direct unit tests. It is exercised only indirectly through
SweepController integration tests.

---

## AccountFactory Test Coverage

| Public function | Happy path | Error paths | Edge cases | Status |
| :--- | :--- | :--- | :--- | :--- |
| `initialize` | — | 4 tests (double-init, auth, panic code) | — | ✅ Covered |
| `batch_initialize` | `test_batch_initialize_returns_one_success_per_request` | — | Salt uniqueness, nonce monotonicity | ⚠️ Partial |

**Gaps:**
- `batch_initialize` before `initialize` (`NotInitialized` error) — the `expect()` panic
  path is never tested.
- Partial failure within a batch — untested.
- `batch_initialize` auth failure — never directly tested.

---

## ReserveContract Test Coverage

| Public function | Happy path | Error paths | Edge cases | Status |
| :--- | :--- | :--- | :--- | :--- |
| `initialize` | `test_initialize_stores_admin` | `test_initialize_twice_panics` | — | ✅ Covered |
| `set_base_reserve` | 6 happy-path tests | 4 error-path tests (zero, negative, min, above max) | — | ✅ Covered |
| `get_base_reserve` | `test_set_and_get_base_reserve` | `test_get_base_reserve_returns_none_when_not_set` | — | ✅ Covered |
| `require_base_reserve` | `test_set_and_get_base_reserve` | `test_require_base_reserve_panics_when_not_set` | — | ✅ Covered |
| `has_base_reserve` | `test_set_and_get_base_reserve` | `test_has_base_reserve_returns_false_when_not_set` | — | ✅ Covered |
| `get_admin` | 2 tests | — | — | ✅ Covered |

**Gaps:**
- `Unauthorized` (3002) on `set_base_reserve` — every test uses `mock_all_auths()`,
  so non-admin rejection is never directly tested.

---

## Error-Code Coverage

Count of error variants never exercised in any test, by contract:

| Contract | Total variants | Untested | Percentage untested |
| :--- | :--- | :--- | :--- |
| SweepController (2000–2999) | 19 | 13 | 68% |
| EphemeralAccount (1000–1999) | 15 | 7 | 47% |
| AccountFactory (4000–4999) | 1 | 0 | 0% |
| ReserveContract (3000–3999) | 3 | 1 | 33% |

**SweepController untested error codes:**
`TransferFailed` (2001), `InsufficientBalance` (2003), `AccountExpired` (2005),
`AccountAlreadySwept` (2006), `InvalidSignature` (2007), `SignatureVerificationFailed` (2008),
`InvalidNonce` (2010), `NotAdmin` (2013), `Overflow` (2014), `InvalidEstimateInput` (2015),
`TimeLockNotElapsed` (2016), `NoPendingSignerUpdate` (2017), `NotInitialized` (2018).

---

## Integration Test Health

| Test file | Total | Passing | Ignored | Notes |
| :--- | :--- | :--- | :--- | :--- |
| `sweep_controller/tests/integration.rs` | 34 | 30 | **4** | All ignored tests are blocked by Soroban auth-tree mock limitation |

**Ignored tests** (critical paths with no working test):
1. `test_claim_succeeds_with_recipient_auth_and_relayable_flow`
2. `test_claim_records_recipient_authorization_context`
3. `test_claim_rejects_wrong_recipient_for_locked_destination`
4. `test_initialize_with_authorized_destination`

These tests validate the claim authorization chain and locked-destination enforcement.
Their `#[ignore]` status means these critical paths have no working test.

---

## Sign-Off

| Role | Name | Signature / commit SHA | Date (UTC) |
| :--- | :--- | :--- | :--- |
| Operator |  |  |  |
| Reviewer  |  |  |  |

---

## Related Issues

- [#313](https://github.com/bridgelet-org/bridgelet-core/issues/313) — Missing test coverage for EphemeralAccount::sweep with Ed25519 signatures
- [#327](https://github.com/bridgelet-org/bridgelet-core/issues/327) — Incomplete test coverage for claim() authorization context
- [`postmortems/ed25519-verify-panic-vs-result.md`](postmortems/ed25519-verify-panic-vs-result.md) — ed25519_verify panic-vs-result mismatch
- [`postmortems/claim-nonce-bypass.md`](postmortems/claim-nonce-bypass.md) — claim nonce-bypass postmortem
