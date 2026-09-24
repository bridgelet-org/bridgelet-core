# Bridgelet Core Fee-Payer / Sponsor Model

This document specifies which account bears the transaction fee (Soroban resource fees + inclusion fees) at each step of the Bridgelet contract lifecycle on Stellar Testnet/Mainnet.

**Cross-references:**
- [FeeSponsor Registry Contract](../contracts/fee_sponsor/README.md) (proposed)
- [bridgelet-sdk Funding Account Documentation](../../sdk/funding-account.md) (proposed)

---

## Fee Model Principles

1. **Explicit fee-payer**: Every contract-invoking step documents its expected fee-payer.
2. **Sponsorship support**: Where applicable, steps identify if/fee sponsorship (FeeBumpTransaction) can shift the fee burden.
3. **No ambiguity**: Any step with an undefined or ambiguous fee-payer is flagged and resolved.

---

## Lifecycle Steps & Fee-Payer Matrix

| Step | Contract / Function | Fee-Payer | Sponsorship Eligible? | Notes |
|------|---------------------|-----------|----------------------|-------|
| **1. Deploy Contracts** | `stellar contract deploy` (ephemeral_account, sweep_controller, reserve_contract, account_factory) | **Deployer** (source account of deploy tx) | Yes — deployer can sponsor via FeeBump | One-time per contract deployment. Deployer must have XLM for rent + deploy fees. |
| **2. Initialize EphemeralAccount** | `EphemeralAccount::initialize()` | **Creator** (passed as `creator` param, must `require_auth()`) | Yes — creator can be sponsored | Creator signs the invoke tx. Fee paid by transaction source account. |
| **3. Initialize SweepController** | `SweepController::initialize()` | **Creator** (passed as `creator` param, must `require_auth()`) | Yes | Creator signs. |
| **4. Initialize ReserveContract** | `ReserveContract::initialize()` | **Admin** (passed as `admin` param, must `require_auth()`) | Yes | Admin signs. |
| **5. Initialize AccountFactory** | `AccountFactory::initialize()` | **Deployer / Admin** (no auth required in current impl; deployer typically calls) | Yes | Current impl has no `require_auth()` — any caller can initialize. Fee paid by tx source. |
| **6. Record Payment** | `EphemeralAccount::record_payment()` | **Payment Watcher / SDK / Anyone** | Yes — any caller can be sponsored | **No auth required.** Off-chain watcher (SDK) typically calls this. Fee paid by tx source. |
| **7. Batch Initialize (AccountFactory)** | `AccountFactory::batch_initialize()` | **Creator** (passed as `creator`, must `require_auth()`) | Yes | Single tx deploys + initializes N accounts. Creator pays for entire batch. |
| **8. Execute Sweep (Signed Path)** | `SweepController::execute_sweep()` | **Relayer / SDK / Submitter** | Yes — relayer can be sponsored | Relayer submits tx with Ed25519 signature. Relayer pays fees. |
| **9. Claim (Gas-Free Path)** | `SweepController::claim()` | **Relayer / Submitter** | **N/A — this IS the gas-free path for recipient** | **Recipient only signs Soroban auth entry (no fee).** Relayer submits and pays all fees. |
| **10. Expire Account** | `EphemeralAccount::expire()` | **Anyone** (no auth required) | Yes | Typically called by recovery address or SDK watcher. Fee paid by tx source. |
| **11. Recover Expired Account** | `EphemeralAccount::recover()` | **Creator or Recovery Address** (must `require_auth()`) | Yes | Caller pays fees. |
| **12. Reclaim Reserve** | `EphemeralAccount::reclaim_reserve()` | **Anyone** (no auth required, but only useful post-sweep/expire) | Yes | Fee paid by tx source. |
| **13. Update Authorized Destination** | `SweepController::update_authorized_destination()` | **Creator** (must `require_auth()`) | Yes | Only before first sweep (nonce == 0). |
| **14. Upgrade EphemeralAccount** | `EphemeralAccount::upgrade()` | **Admin** (must `require_auth()`) | Yes | Admin pays fees. |
| **15. Set Base Reserve** | `ReserveContract::set_base_reserve()` | **Admin** (must `require_auth()`) | Yes | Admin pays fees. |

---

## Fee Sponsorship (FeeBumpTransaction) Integration

### When to Use Sponsorship

- **Relayer-sponsored sweeps**: Relayer submits `execute_sweep` but a sponsor (e.g., platform treasury) pays fees.
- **Recipient-sponsored claims**: In `claim()`, the recipient signs the auth entry; a relayer submits. A sponsor can pay the relayer's fees via FeeBump.
- **Batch operations**: `AccountFactory::batch_initialize` can be expensive; sponsor pays.

### How It Works

1. **Inner transaction**: Built by the operational actor (relayer, SDK, creator) with their auth.
2. **FeeBump envelope**: Sponsor wraps inner tx, signs as fee source, submits.
3. **On-chain**: Soroban validates inner auth (operational actor) + fee source (sponsor).

### Bridgelet-SDK Expected Behavior

| SDK Operation | Inner Tx Source | Expected Sponsor | FeeBump Used? |
|---------------|-----------------|------------------|---------------|
| `record_payment` | SDK watcher | Platform treasury (optional) | Optional |
| `execute_sweep` | Relayer | Platform treasury (optional) | Optional |
| `claim` | Relayer | Platform treasury (optional) | Optional |
| `batch_initialize` | Creator/Deployer | Platform treasury (recommended) | Recommended |

---

## Contract-Level Fee Implications

### EphemeralAccount

- **Storage**: Each account stores payments (max 10), status, reserve tracking. Rent fees paid by **deployer** at deploy; ongoing rent auto-extended via `extend_instance_ttl` in state-changing calls (creator/relayer/anyone pays).
- **No internal fee logic**: Contract does not charge fees; only standard Soroban resource fees apply.

### SweepController

- **Nonce increment**: Each `execute_sweep`/`claim` increments nonce — small storage write.
- **Token transfers**: SEP-41 `transfer()` calls incur their own resource fees (charged to tx source).
- **Auth entries**: `authorize_as_current_contract()` adds sub-invocation auth entries (minor overhead).

### ReserveContract

- **Admin-only writes**: `set_base_reserve` — admin pays.
- **Read-only**: `get_base_reserve`, `has_base_reserve`, `require_base_reserve` — no auth, minimal fees.

### AccountFactory

- **Batch deploy**: Deploys N contracts in one tx. Resource fees scale with N (WASM upload once, N instance creations).
- **Error detail dropped**: Current impl returns `error: None` on failure — monitoring must account for silent failures.

---

## Ambiguities Resolved

| Previously Ambiguous Step | Resolution |
|---------------------------|------------|
| `record_payment` — who calls? | **SDK watcher** (off-chain). No auth required. Fee-payer = tx source (watcher or sponsor). |
| `claim()` — recipient vs relayer fees? | **Relayer pays all fees.** Recipient only provides Soroban auth signature. |
| `batch_initialize` — per-account fee? | **Single tx, single fee-payer (creator).** All N accounts deployed/initialized atomically. |
| `expire()` / `recover()` — who triggers? | **Anyone** (expire) / **Creator or Recovery** (recover). No protocol-enforced fee-payer; typically SDK watcher or recovery address. |
| `ReserveContract` reads by `EphemeralAccount`? | **Not currently implemented.** `EphemeralAccount` uses hardcoded `BASE_RESERVE_STROOPS`. If wired, read would be in sweep/expire tx (fee-payer = sweep/expire tx source). |

---

## Implementation Checklist for SDK / Integrators

- [ ] **Deployer**: Fund deployer account for contract deploy + initialization fees.
- [ ] **SDK Watcher**: Fund watcher account for `record_payment` calls (or configure sponsorship).
- [ ] **Relayer**: Fund relayer account for `execute_sweep` and `claim` submissions (or configure sponsorship).
- [ ] **Sponsor (optional)**: Fund sponsor account for FeeBump transactions.
- [ ] **Recovery Address**: Fund recovery address for `expire()`/`recover()` calls if not sponsored.
- [ ] **Monitoring**: Track fee spend per role to detect anomalies.

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-09-24 | Initial fee-payer matrix for all contract lifecycle steps |

---

## Related Documents

- [API Reference](./api-reference.md) — Function signatures and auth requirements
- [Architecture](./architecture.md) — System data flow and component responsibilities
- [Security Model](./security.md) — Authorization and trust boundaries
- Bridgelet-SDK Funding Account Spec (proposed) — Off-chain fee management