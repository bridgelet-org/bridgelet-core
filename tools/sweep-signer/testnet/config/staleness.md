 testnet/config/staleness-check.md: document how to verify your local contract-ID config still matches the live testnet deployment
Repo Avatar
bridgelet-org/bridgelet-core
File: testnet/config/staleness-check.md

If testnet is ever redeployed (new contract IDs), integrators with cached local config would silently start calling stale/nonexistent contracts. Document a simple verification step (e.g. querying the contract via RPC and checking it responds as expected) integrators can run periodically.