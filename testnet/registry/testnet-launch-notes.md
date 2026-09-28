# Testnet Launch Notes & Historical Deployment Context

## Overview

This document preserves the institutional and historical context surrounding the initial standing up and provisioning of the Bridgelet testnet deployment environment. As team members and contributors transition between workstreams, this log ensures operational lineage, original configuration parameters, and deployment authorship remain traceable and accessible.

---

## Initial Standup Context

| Parameter | Historical Record |
|---|---|
| **Initial Deployment Epoch** | September 2026 |
| **Initial Deployer / Architect** | Bridgelet Core Protocol Team (`bridgelet-org`) |
| **Target Network** | Stellar Testnet (`Test SDF Network ; September 2015`) |
| **RPC Endpoint** | `https://soroban-testnet.stellar.org:443` |
| **Horizon API** | `https://horizon-testnet.stellar.org` |
| **Network Passphrase** | `Test SDF Network ; September 2015` |
| **Core Repositories** | `bridgelet-org/bridgelet-core` |

---

## Deployment Architecture & Lineage

### 1. Initial Infrastructure Setup
The initial testnet tooling and contract scaffolds were designed around the Soroban SDK and the Stellar CLI to facilitate zero-custody ephemeral account bridging and automated sweeps:
- **Ephemeral Account Architecture**: Created to manage transient user balances with programmable release conditions.
- **Sweep Controller**: Architected to coordinate authorized destination sweeps without persistent exposure of fund keys.
- **Reserve Contract & Account Factory**: Scaffolds for multi-account lifecycle management.

### 2. Operational Scripts
The standing-up process established the standard testnet operational scripts in the repository:
- `scripts/build.sh`: Builds release WASM binaries for target `wasm32-unknown-unknown`.
- `scripts/deploy-testnet.sh`: Handles key generation, testnet funding via Friendbot, WASM upload, and contract instantiation.
- `scripts/verify-contracts.sh`: Verifies live RPC state against local hashes.

### 3. Key Contacts & Stewardship
- **Originating Organization**: Bridgelet Protocol Engineering Team
- **Governance / Admin Roles**: Documented in `testnet/registry/admin-addresses.md`
- **Verification Authority**: Recorded in `testnet/registry/contract-status.md` and `testnet/registry/last-verified.json`

---

## Maintainer Transition Guidelines

When new engineers or maintainers take ownership of the testnet environment:
1. **Audit Live Contracts**: Check current deployment status against `testnet/registry/contract-status.md`.
2. **Review Upgrade Log**: Inspect `testnet/registry/upgrade-history.md` for any in-place WASM upgrades since initial standup.
3. **Verify Keypair Control**: Ensure testnet admin keys configured in CI/CD environments match authorized deployer identities.
4. **Update Institutional Records**: Log any new redeployments, migration epics, or coordinator handoffs directly in this file.
