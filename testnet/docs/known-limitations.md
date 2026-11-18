# Known Limitations on Testnet

The MVP stub behaviours and gaps that matter when you test against the live
testnet contracts. This is the testnet-relevant subset of the main README's
"Current Stub Inventory" and "Implementation Notes"
([README](../../README.md#mvp-status)); read it so you don't carry a wrong
assumption into a real integration.

> Bridgelet Core is an MVP and **unaudited**. Source of truth: the contract code in
> `contracts/` and the README. If they change, this page must change with them.

## Stubs and partial implementations

| # | Limitation | What actually happens | Why it matters on testnet | More |
|---|---|---|---|---|
| 1 | **`EphemeralAccount::sweep()` does not verify its signature** | The `auth_signature` argument is ignored. Authorization is only `authorized_controller.require_auth()`, i.e. whoever that address is. A garbage signature is accepted | A successful direct call does **not** prove signature verification works. Never design around it | [security/unverified-signature-path-warning.md](../security/unverified-signature-path-warning.md), [security/authorized-controller-confusion.md](../security/authorized-controller-confusion.md) |
| 2 | **Real signature verification lives only in `SweepController`** | `execute_sweep` verifies Ed25519 over `hash(destination + nonce + contract_id)` with replay protection. `claim()` uses the recipient's Soroban auth entry | Always sweep via the controller | [examples/signed-sweep-walkthrough.md](../examples/signed-sweep-walkthrough.md) |
| 3 | **The contracts do not move tokens on sweep** | `sweep_account` validates state and emits events; the SDK performs the payout via Horizon | "Sweep succeeded" ≠ "funds arrived". Verify the balance separately | [examples/multi-asset-sweep.md](../examples/multi-asset-sweep.md) |
| 4 | **`AccountFactory::batch_initialize` drops error detail** | On a per-account failure it returns `error: None` instead of the actual error | You see *that* an account failed, not *why*. Re-run the account individually to diagnose | [runbooks/account-factory-batch-failure.md](../runbooks/account-factory-batch-failure.md) |
| 5 | **Reserve tracking is duplicated and unwired** | `EphemeralAccount` uses its own hardcoded `BASE_RESERVE_STROOPS` (`1_000_000_000` stroops in source). `ReserveContract` stores an admin-set value that nothing reads | Changing `ReserveContract::set_base_reserve` does not change sweep behaviour | [registry/reserve-contract-status.md](../registry/reserve-contract-status.md), [runbooks/insufficient-reserve-errors.md](../runbooks/insufficient-reserve-errors.md) |
| 6 | **`AccountFactory` is not covered by the tooling** | It is a workspace member, but is not built by `scripts/build.sh`, not tested in CI, and was not deployed by older versions of the deploy script. The current deploy script and `deployments/testnet.json` do list an `accountFactory` ID | Do not assume it behaves as documented; verify its presence and hash first | [registry/account-factory-status.md](../registry/account-factory-status.md), [examples/batch-account-creation-mock.md](../examples/batch-account-creation-mock.md) |

## Operational limitations

| Limitation | Detail |
|---|---|
| **CI is not a safety net** | `.github/workflows/test.yml` is entirely commented out; `deploy-testnet.yml` validates the build for three contracts (not `account_factory`) and its deploy steps are commented out. Nothing runs automatically |
| **Expiry is not automatic** | An account becomes *expirable* at `expiry_ledger`, but the `expired` event/state change happens only when someone calls `expire()` or `recover()` ([config/expiry-ledger-testing.md](../config/expiry-ledger-testing.md)) |
| **Contracts can be upgraded** | `EphemeralAccount::upgrade(new_wasm_hash)` is admin-gated and emits no event; pin and check WASM hashes ([runbooks/upgrade-verification.md](../runbooks/upgrade-verification.md)) |
| **Testnet resets and redeploys** | Contract IDs, accounts and nonces vanish on a reset or redeploy ([policies/reset-policy.md](../policies/reset-policy.md)) |
| **Records can be stale** | Registry tables, deployment artifacts and docs can disagree; verify on-chain ([registry/verify-contract-live.md](../registry/verify-contract-live.md)) |
| **Support is best effort** | No SLA, no on-call ([docs/support.md](support.md)) |

## Things this page does not cover

- Mainnet readiness: the contracts are unaudited.
- Bugs in the contracts beyond the documented stubs; see
  [security/disclosure-process.md](../security/disclosure-process.md).

## Practical guidance

1. Route every sweep through `SweepController`.
2. Treat a direct `EphemeralAccount::sweep()` success as meaningless for authorization.
3. Verify balances independently of events.
4. Re-verify contract IDs and WASM hashes at the start of each test session.
5. Check [common-misconceptions.md](common-misconceptions.md) (if merged) for more.
