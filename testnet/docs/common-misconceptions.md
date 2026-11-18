# Common Misconceptions

A skimmable list of things new integrators commonly assume that the code does not
do. Each row links to the detailed explanation. Derived from the repository
README ("MVP Status") and contract source; not observed from real integrations.
Add rows as new ones are discovered.

| # | Misconception | Reality | Details |
|---|---|---|---|
| 1 | "`reserve_contract` changes sweep amounts" | It stores an admin-set base reserve value, but `EphemeralAccount` uses its own hardcoded `BASE_RESERVE_STROOPS` and never reads `ReserveContract`. Changing it has no effect on sweeps today | `testnet/registry/reserve-contract-status.md`, `testnet/runbooks/insufficient-reserve-errors.md` |
| 2 | "I can call `account_factory` on testnet" | It is a real contract but is not built by `scripts/build.sh`, not deployed by the deploy script, and not tested in CI; do not assume an instance exists | `testnet/registry/account-factory-status.md`, `testnet/examples/batch-account-creation-mock.md` |
| 3 | "`sweep()`'s signature parameter is checked" | `EphemeralAccount::sweep()` ignores `auth_signature`; only the `authorized_controller.require_auth()` gate applies. Real Ed25519 verification is in `SweepController::execute_sweep` | `testnet/security/unverified-signature-path-warning.md`, `testnet/security/authorized-controller-confusion.md` |
| 4 | "A successful sweep moved the tokens" | The contracts update state and emit events; they do not move tokens. The SDK performs the custodial payout through Horizon | `testnet/examples/signed-sweep-walkthrough.md` |
| 5 | "The sweep event amount is one number for one asset" | It is the sum of payment amounts across assets | `testnet/examples/multi-asset-sweep.md` |
| 6 | "Accounts expire automatically at `expiry_ledger`" | `expired` is emitted only when someone successfully calls `expire()`/`recover()`. Passing the ledger just makes the account expirable | `testnet/config/expiry-ledger-testing.md`, `testnet/examples/expiry-and-recovery.md` |
| 7 | "`claim()` needs the recipient to hold XLM" | The recipient signs an auth entry; a relayer submits and pays the fee | `testnet/config/gas-free-claim-testing.md`, `testnet/runbooks/relayer-submission-failures.md` |
| 8 | "Locked mode restricts who can call" | It restricts the destination only | `testnet/examples/testing-locked-mode.md` |
| 9 | "The nonce is something I track locally" | The contract verifies against its own on-chain nonce; read `get_nonce` | `testnet/config/nonce-tracking.md`, `testnet/runbooks/nonce-desync.md` |
| 10 | "Contract events come from Horizon" | They come from Soroban RPC | `testnet/runbooks/event-not-appearing-in-horizon.md` |
| 11 | "A deployed ID in a document is live" | IDs go stale after redeploys and testnet resets; docs and artifacts can disagree | `testnet/runbooks/contract-not-found-error.md`, `testnet/policies/reset-policy.md` |
| 12 | "A bad-auth rejection means my contract auth is wrong" | It is usually the network passphrase | `testnet/runbooks/wrong-network-passphrase.md` |
| 13 | "Retrying account creation is safe" | A retried deployment creates another contract; there is no on-chain replay protection for creation or payment recording | `testnet/integration/idempotency-key-recommendation.md` |
| 14 | "`batch_initialize` tells me why an account failed" | It reports that an account failed but drops the error detail | `testnet/runbooks/account-factory-batch-failure.md` |
| 15 | "Testnet is permanent" | The SDF resets testnet periodically and contracts can be redeployed or upgraded | `testnet/policies/reset-policy.md`, `testnet/integration/testing-against-upgraded-contracts.md` |
| 16 | "CI deploys to testnet" | The workflows validate the build only; the deploy steps are commented out (README, "CI/CD") | `testnet/docs/overview.md` |
| 17 | "Testnet keys are throwaway, so reuse is fine" | A reused mainnet-intended key is exposed for real | `testnet/security/testnet-vs-mainnet-key-reuse-warning.md` |

## Maintaining this list

Keep entries short, state the reality in one sentence, and link to the detailed
doc rather than duplicating it. When a misconception stops being true (for
example `reserve_contract` gets wired in), update or remove the row and the linked
documents together.
