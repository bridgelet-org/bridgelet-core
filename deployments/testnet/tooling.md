 testnet/tooling/event-tail-spec.md: spec for a small tool to tail testnet events for a given contract in real time
Repo Avatar
bridgelet-org/bridgelet-core
File: testnet/tooling/event-tail-spec.md

Specify a lightweight 'tail -f'-style tool concept for streaming live testnet events (using the topic format documented in testnet/registry/event-topics.md) to a terminal during manual testing/demo sessions, as a design doc for future implementation.
testnet/tooling/README.md: index proposed testnet-facing tooling that lives outside the project's scripts/ folder
Repo Avatar
bridgelet-org/bridgelet-core
File: testnet/tooling/README.md

Since this batch explicitly must not touch the scripts/ folder, use testnet/tooling/ as a home for tooling proposals and specs that are testnet-integration-facing rather than deployment-facing, with a clear note in this README explaining that distinction so a future contributor doesn't mistakenly think this is where deploy scripts belong.
estnet/examples/integration-test-harness-notes.md: guidance for writing a project's own integration tests against live testnet
Repo Avatar
bridgelet-org/bridgelet-core
File: testnet/examples/integration-test-harness-notes.md

Since scripts/test.sh only covers unit tests (out of scope here) and there's no local sandbox entrypoint, document good practices for a consuming project's integration test suite that talks to real testnet: expected latency, test-account funding/cleanup strategy, and how to avoid tests interfering with each other's shared testnet state
testnet/examples/error-handling.md: map each contract Error variant to example integrator-facing handling code
Repo Avatar
bridgelet-org/bridgelet-core
File: testnet/examples/error-handling.md

Cross-reference each contract's errors.rs variants (as documented, not edited, via docs/api-reference.md) with example handling logic for an integration test or SDK wrapper, so integrators aren't left guessing what a raw Soroban error code maps back to in domain terms.