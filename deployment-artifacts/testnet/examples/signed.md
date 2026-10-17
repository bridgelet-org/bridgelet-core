testnet/examples/signed-sweep-walkthrough.md: worked example of constructing and submitting a signed sweep via SweepController
Repo Avatar
bridgelet-org/bridgelet-core
File: testnet/examples/signed-sweep-walkthrough.md

Walk through constructing the hash(destination + nonce + contract_id) message, obtaining a signature (contextually referencing tools/sweep-signer without editing it), and calling SweepController::execute_sweep() against testnet, with expected success and common-failure outputs.
testnet/examples/create-and-fund-ephemeral-account.md: worked example of the create-then-pay flow against testnet
Repo Avatar
bridgelet-org/bridgelet-core
File: testnet/examples/create-and-fund-ephemeral-account.md

Provide a concrete, copy-pasteable example calling initialize() then record_payment() against the live testnet EphemeralAccount contract ID (referencing it as documented in testnet/registry/contract-status.md, not hardcoding a value that could go stale here), including expected event output.