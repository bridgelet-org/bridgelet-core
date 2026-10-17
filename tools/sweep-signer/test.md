testnet/tooling/contract-id-lookup-spec.md: spec for a small CLI that resolves a contract name to its current testnet ID
Repo Avatar
bridgelet-org/bridgelet-core
File: testnet/tooling/contract-id-lookup-spec.md

Specify (design only) a tiny lookup helper that reads testnet/registry/contract-status.md/last-verified.json and returns the current testnet contract ID for a given contract name, so integration test code could depend on a stable interface rather than hand-copying IDs, without this batch implementing or placing the tool inside the excluded scripts/ directory.
testnet/tooling/sweep-signer-integration-notes.md: document how to invoke tools/sweep-signer for testnet signature generation without modifying it
Repo Avatar
bridgelet-org/bridgelet-core
File: testnet/tooling/sweep-signer-integration-notes.md

tools/sweep-signer (a separate directory from scripts/, contracts/, deployments/, and deployment-artifacts/, so referencing its existing behavior here is fine) is the CLI referenced by .env.example for deriving AUTHORIZED_SIGNER_PUBLIC_KEY. Document, purely as usage notes (no changes to the tool itself), the exact invocation pattern for generating a testnet sweep signature end-to-end, since .env.example's inline comment only shows the pubkey subcommand, not the actual signing subcommand an integrator needs.

 testnet/examples/checking-account-status.md: worked example using is_expired() and can_sweep() as pre-flight checks
Repo Avatar
bridgelet-org/bridgelet-core
File: testnet/examples/checking-account-status.md

Show how to use the read-only is_expired() (EphemeralAccount) and can_sweep() (SweepController) methods as pre-flight checks before attempting a state-changing call, to reduce failed-transaction fee waste during integration testing.
testnet/examples/gas-free-claim-walkthrough.md: worked example of the recipient-signs/relayer-submits claim() flow
Repo Avatar
bridgelet-org/bridgelet-core
File: testnet/examples/gas-free-claim-walkthrough.md

Document the less-obvious claim() path end-to-end: how the recipient constructs and signs the Soroban auth entry, and how a relayer submits it, since this is architecturally different enough from the standard signed-sweep flow that a dedicated walkthrough will save integrators significant trial-and-error.