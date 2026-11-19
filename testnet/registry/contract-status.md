# Testnet Contract Status Registry

## Overview

This is the canonical quick-reference table for Bridgelet contract deployment status on Stellar Testnet. Updated whenever someone verifies testnet state. Pair with `testnet/registry/last-verified.json` for automated staleness checks.

---

## Contract Deployment Status Table

| Contract | Deployed | Contract ID | WASM Hash | Last Verified | Caveats |
|---|---|---|---|---|---|
| EphemeralAccount | ❌ No | TBD | TBD | Never | Not yet deployed; see deploy scripts |
| SweepController | ❌ No | TBD | TBD | Never | Not yet deployed; see deploy scripts |
| ReserveContract | ❌ No | N/A | N/A | Never | No testnet presence or wiring |
| AccountFactory | ❌ No | N/A | N/A | Never | Real but entirely undocumented/undeployed |

> **Legend**: ✅ = Deployed and verified live; ❌ = Not deployed; ⚠️ = Deployed but issues; TBD = To be determined

---

## Detailed Status by Contract

### EphemeralAccount
- **Status**: Not deployed
- **Blocker**: `scripts/deploy-testnet.sh` needs fixes (unbound `RESERVE_CONTRACT_ID` variable)
- **WASM**: Built by `scripts/build.sh` (includes `ephemeral_account`, `sweep_controller`, `reserve_contract`)
- **Dependencies**: None (standalone)
- **Next Steps**: Fix deploy script → Deploy → Verify with `verify-contract-live.md` → Update this table

### SweepController
- **Status**: Not deployed
- **Blocker**: Same as EphemeralAccount (deploy script issues)
- **WASM**: Built by `scripts/build.sh`
- **Dependencies**: Requires `authorized_signer` (Ed25519 pubkey) and optional `authorized_destination`
- **Next Steps**: Deploy after EphemeralAccount → Configure signer → Verify

### ReserveContract
- **Status**: Not deployed
- **Blocker**: Not included in `scripts/deploy-testnet.sh` at all
- **WASM**: Built by `scripts/build.sh`
- **Integration**: **Not wired** - EphemeralAccount uses hardcoded `BASE_RESERVE_STROOPS` (1 XLM), does not read from ReserveContract
- **Gap**: Two separate issues: (1) not deployed, (2) no cross-contract wiring
- **Next Steps**: Add to deploy script → Deploy → Decide on wiring → Update EphemeralAccount if needed

### AccountFactory
- **Status**: Not deployed
- **Blocker**: Not built by `scripts/build.sh`; not in CI; not in deploy script
- **WASM**: Must build manually: `cd contracts/account_factory && cargo build --target wasm32-unknown-unknown --release`
- **Dependencies**: Requires `ephemeral_account.wasm` hash at `initialize()`
- **Known Issue**: `batch_initialize` swallows per-account error detail (`error: None`)
- **Next Steps**: Add to build script → Add to CI → Add to deploy script → Deploy → Document

---

## Deployment Checklist

When deploying contracts to testnet, update this table and the corresponding status files:

- [ ] **EphemeralAccount deployed**
  - Contract ID recorded
  - WASM hash recorded
  - Verified with `verify-contract-live.md` procedure
  - `last-verified.json` updated

- [ ] **SweepController deployed**
  - Contract ID recorded
  - WASM hash recorded
  - `authorized_signer` configured
  - Verified with `verify-contract-live.md`
  - `last-verified.json` updated

- [ ] **ReserveContract deployed** (optional)
  - Contract ID recorded
  - WASM hash recorded
  - `admin` set
  - `base_reserve` configured
  - Verified with `verify-contract-live.md`
  - `last-verified.json` updated

- [ ] **AccountFactory deployed** (optional)
  - Contract ID recorded
  - WASM hash recorded
  - Initialized with `ephemeral_account` WASM hash
  - Verified with `verify-contract-live.md`
  - `last-verified.json` updated

---

## Cross-References

| Document | Purpose |
|---|---|
| `testnet/registry/last-verified.json` | Machine-readable timestamps for staleness checks |
| `testnet/registry/wasm-hash-reference.md` | WASM hash verification |
| `testnet/registry/verify-contract-live.md` | Health check procedures |
| `testnet/registry/event-topics.md` | Event topics for indexers |
| `testnet/registry/admin-addresses.md` | Admin/controller addresses on live contracts |
| `testnet/registry/upgrade-history.md` | Contract upgrade log |
| `testnet/registry/reserve-contract-status.md` | ReserveContract detailed status |
| `testnet/registry/account-factory-status.md` | AccountFactory detailed status |
| `testnet/registry/dependency-graph.md` | Contract call graph |
| `testnet/registry/known-test-accounts.md` | Long-lived test accounts for demos |

---

## Update Process

1. **Verify**: Run health checks per `verify-contract-live.md`
2. **Record**: Update this table with Contract ID, WASM Hash, Date, Caveats
3. **Sync**: Update `last-verified.json` with ISO timestamps
4. **Notify**: Commit changes; CI/bot can detect staleness (>30 days)

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-24 | Initial testnet contract status registry |

---

## Quick reference (one-line status)

This is the compact, copy-pasteable status card for integrators. It reflects the IDs recorded in `deployments/testnet.json` (committed `deployedAt`: `2026-10-10T04:38:14Z`) and the timestamps in `last-verified.json`. Keep it in sync with the detailed sections above.

| Contract | Deployed (Y/N) | Contract ID | Last verified live | One-line caveat |
|---|---|---|---|---|
| ephemeral_account | Y - supported deployment | `CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3` | 2026-09-24 (stale - predates deployment record; re-verify) | Deployed and initialized per account; route sweeps through `sweep_controller`, never call `sweep()` directly. |
| sweep_controller | Y - supported deployment | `CAJB4RRM5JRPXLITC5JUA7Q3LU2WQJDRPXVKMORXWHJJBPZRYWDJ5V4S` | 2026-09-24 (stale - predates deployment record; re-verify) | Recommended entry point; real Ed25519 verification with nonce replay protection; initialized unlocked (`authorized_destination = null`). |
| reserve_contract | N - ID listed, not part of supported deployment | `CBG3Y4GWFTL4D37JGV2XWAWC7EUQ7XGH4XM3SFLQYYSKPB2IYMAS54HY` | 2026-09-24 (do not treat as live) | ID appears in `deployments/testnet.json`, but the deploy script never deploys it and it is not wired into `ephemeral_account` (which uses its own hardcoded `BASE_RESERVE_STROOPS`). |
| account_factory | N - ID listed, not part of supported deployment | `CDBY4L3NTLW4OBZZTY73VL3NNWO2AGFFGBRJVDL3IMKDDLFF2W57WK6H` | 2026-09-24 (do not treat as live) | ID appears in `deployments/testnet.json`, but it is not built by `scripts/build.sh`, not in CI, and not deployed by the tooling; `batch_initialize` drops per-account error detail. |

**Reconciling the source-of-truth discrepancy (read this before relying on any row).** The committed `deployments/testnet.json` lists IDs for **all four** contracts, while the main README and `testnet/docs/overview.md` state that **only `ephemeral_account` and `sweep_controller`** are part of the supported testnet deployment (the deploy script never deploys `reserve_contract` or `account_factory`, and references an unassigned `RESERVE_CONTRACT_ID`). These statements are not proof that four contracts are live. The two "Y" rows are the supported integration targets; the two "N" rows carry IDs that may be stale, hand-entered, or unusable. Additionally, the `2026-09-24` timestamps in `last-verified.json` predate the `2026-10-10` `deployedAt` of the current deployment record, so they cannot be a live confirmation of these specific IDs - treat every row as needing re-verification via `verify-contract-live.md` before you depend on it.

### How to update this table

1. Run the manual health check in `testnet/registry/verify-contract-live.md` (the "Copy-paste health-check script" section) using the IDs read from `deployments/testnet.json`. Do not edit that file as part of this step.
2. For each contract whose read-only call (`is_expired`, `can_sweep`, `has_base_reserve`) returns a sane value, set **Deployed (Y/N)** to `Y`, copy the full ID from `deployments/testnet.json` verbatim, set **Last verified live** to today's UTC date (`YYYY-MM-DD`), and refine the caveat.
3. For each contract that is "not found" or that is outside the supported deployment (`reserve_contract`, `account_factory`), set **Deployed (Y/N)** to `N` and keep a caveat explaining why. An ID being present in `deployments/testnet.json` is not, by itself, evidence of a live contract.
4. Write the same UTC timestamp into `testnet/registry/last-verified.json` for every contract you actually verified, using the existing keys (`ephemeral_account`, `sweep_controller`, `reserve_contract`, `account_factory`). Add a new key only if a new contract is introduced; never remove existing keys, so the JSON stays valid for staleness checks.
5. If an upgrade changed behavior, also update `wasm-hash-reference.md` and `upgrade-history.md` (see the "Update Process" above).
6. Commit only the registry/config documentation files. `deployments/testnet.json` is owned by `scripts/deploy-testnet.sh` and is out of scope for a registry verification batch.
