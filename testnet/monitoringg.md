 testnet/monitoring/uptime-check-spec.md: specify what a future automated testnet health check should verify
Repo Avatar
bridgelet-org/bridgelet-core
File: testnet/monitoring/uptime-check-spec.md

Before anyone builds actual monitoring tooling (out of scope here — that would likely live under the excluded scripts/ or a future dedicated monitoring service), write the specification: which contract methods to call, expected response shape/timing, and how often, so implementation can proceed later against an agreed spec rather than ad hoc.
testnet/monitoring/balance-thresholds.md: document minimum funding thresholds for any long-lived testnet demo/test accounts
Repo Avatar
bridgelet-org/bridgelet-core
File: testnet/monitoring/balance-thresholds.md

For the long-lived demo accounts proposed in testnet/registry/known-test-accounts.md, document minimum balance thresholds below which they need re-funding via Friendbot, so demos don't unexpectedly fail mid-presentation due to an out-of-gas test account.
testnet/monitoring/event-watchlist.md: define which on-chain events on testnet, if they stopped appearing, would indicate a problem
Repo Avatar
bridgelet-org/bridgelet-core
File: testnet/monitoring/event-watchlist.md

Document which events (SweepCompleted, AccountExpired, etc.) are expected to occur at some minimum baseline frequency if testnet is being actively used/tested, so a future monitoring integration has a concrete list of 'signals of life' to watch for absence-of-activity alerting.
testnet/monitoring/README.md: define what testnet monitoring exists today (likely: none) and what's proposed
Repo Avatar
bridgelet-org/bridgelet-core
File: testnet/monitoring/README.md

Establish a testnet/monitoring/ directory starting from an honest baseline — per the main README, CI is disabled and there's no evidence of any active testnet monitoring — and use this README to track what monitoring exists vs. is aspirational, avoiding the same 'docs describe pipelines that don't actually run' problem the main README explicitly calls out for CI.