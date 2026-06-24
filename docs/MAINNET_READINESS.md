# Mainnet Readiness Checklist — bridgelet-core

This document gates promotion of `bridgelet-core` from testnet to mainnet.
Every item must be checked off and linked to verifiable evidence before a mainnet deployment is approved.

---

## 1. Authorization & Security

- [ ] **Real sweep auth implemented** — `verify_sweep_authorization()` uses `env.crypto().ed25519_verify()` against the `authorized_signer` stored at `initialize()`. No stub or `require_auth()` shortcut remains.
- [ ] **`record_payment()` relayer auth** — Only the registered relayer address can call `record_payment()`; `relayer.require_auth()` is enforced on every call.
- [ ] **`initialize()` creator auth** — `creator.require_auth()` is called before any state is written; double-initialization is rejected.
- [ ] **No privileged escape hatches** — No admin-override or backdoor function exists that bypasses auth checks.

## 2. Token Transfers

- [ ] **Real token transfers in `sweep()`** — `SweepController` executes actual SAC token transfers (via `token::Client::transfer`) for every asset in the payment list; no stub transfer path remains.
- [ ] **Multi-asset sweep tested** — Integration test covers ≥ 2 different assets in a single sweep and confirms all balances land at the destination.

## 3. Test Coverage

- [ ] **Unit test coverage ≥ 90%** — `cargo tarpaulin` (or equivalent) reports ≥ 90 % line coverage for both `ephemeral_account` and `sweep_controller`.
- [ ] **Integration test on local Stellar sandbox passing** — `scripts/test.sh` (or equivalent) passes against a local `stellar-quickstart` node with no failures.
- [ ] **Auth-rejection tests** — Tests confirm that unauthorized callers to `initialize()`, `record_payment()`, and `sweep()` are rejected with the correct error codes.
- [ ] **Replay-attack tests** — Tests confirm a used sweep signature cannot be replayed to trigger a second sweep.

## 4. Storage & Ledger

- [ ] **Storage TTL tested** — Instance and persistent storage TTL extensions are exercised in tests; entries do not expire prematurely under expected account lifetimes.
- [ ] **Upgrade mechanism tested** — If contract upgradability is planned, the upgrade path is documented and tested (or explicitly marked out-of-scope for v1 with a note).

## 5. Error Codes

- [ ] **Error codes stable** — All `contracterror` variants have fixed numeric discriminants that will not change across upgrades. No reordering since last testnet deployment.
- [ ] **Error codes documented** — Every error variant is described in `docs/api-reference.md` with the cause and expected caller behaviour.

## 6. Security Audit

- [ ] **Security audit completed** — An independent audit has been performed and the report is published at `docs/security-audit.md`.
- [ ] **All critical / high findings resolved** — Every critical and high-severity finding from the audit has a documented resolution or accepted risk entry.

## 7. Operational Readiness

- [ ] **Testnet deployment verified** — Both contracts are deployed to Futurenet / Testnet and all acceptance tests pass against those live deployments.
- [ ] **Monitoring & alerting in place** — Off-chain event listeners for `AccountCreated`, `PaymentReceived`, `SweepExecutedMulti`, and `AccountExpired` are running and alerting on anomalies.
- [ ] **Key management documented** — The procedure for rotating `authorized_signer` keys and the `relayer` address is written and reviewed.

---

## Sign-off

| Role | Name | Date | Evidence link |
|------|------|------|---------------|
| Lead Engineer | | | |
| Security Reviewer | | | |
| Product Owner | | | |
