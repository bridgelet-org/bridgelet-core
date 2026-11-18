# Testnet Threat Model (Lightweight)

Scope: the Bridgelet contracts and documentation **as used on Stellar testnet**.
This is intentionally separate from any mainnet threat model. The contracts are
MVP and unaudited (README), and testnet holds no real value, so the question is
not "what can an attacker steal?" but "what do we actually lose?"

> This is a short reasoning document, not a security review. It does not
> certify anything, and conclusions about the contracts' own security belong in
> `docs/security.md`.

## Assets worth protecting on testnet

| Asset | Why it matters |
|---|---|
| Integration credibility | Integrators judge the project by how reliably testnet works and how clear the docs are |
| Developer/debugging time | Griefing or silent drift wastes the time of everyone testing |
| Accuracy of the registry | Wrong contract IDs/status mislead integrators (`testnet/registry/`) |
| Public demo integrity | Demos run on testnet; failures there are visible and embarrassing |
| Key hygiene habits | Practices formed on testnet carry into mainnet (`testnet/security/testnet-vs-mainnet-key-reuse-warning.md`) |
| Long-lived demo accounts | Shared fixtures that others rely on (`testnet/registry/known-test-accounts.md`) |

## Threats that actually matter

| Threat | Example | Impact | Mitigation |
|---|---|---|---|
| Griefing of shared fixtures | Someone sweeps/expires a demo account or drains a relayer | Lost time, broken demos | Etiquette and isolation (`testnet/policies/`, `testnet/monitoring/balance-thresholds.md`) |
| Stale or wrong registry data | IDs from a previous deployment or reset presented as current | Integrators debug phantom failures | Verification routine (`testnet/registry/verify-contract-live.md`) |
| Public-demo failure | Account expires or runs dry just before a presentation | Reputational | Pre-demo checklist; fresh accounts for demos |
| Key leakage that spills to mainnet | A reused key exposed in testnet logs | Real mainnet loss | Never reuse keys across networks |
| Misleading integration lessons | Developers conclude `sweep()` verifies signatures because testnet calls succeed | Insecure mainnet design | `testnet/security/unverified-signature-path-warning.md`, `testnet/security/authorized-controller-confusion.md` |
| Event/indexer confusion | Missing events mistaken for contract bugs | Wasted effort | `testnet/runbooks/event-not-appearing-in-horizon.md` |
| Upgrade surprise | Admin `upgrade()` changes behaviour mid-test | Confusing failures | `testnet/runbooks/upgrade-verification.md` |
| Spam/abuse of public contracts | Many junk accounts or payments | Noise, hard-to-read event streams | `testnet/security/known-testnet-abuse-patterns.md` |

## Explicitly not a concern here

- Loss of real funds: testnet XLM and tokens have no value.
- Economic attacks (front-running, MEV, price manipulation) that depend on real value.
- Regulatory or custody questions about test assets.
- Perfect availability: testnet is best effort and periodically reset by the SDF
  (`testnet/policies/reset-policy.md`).
- Hardening the contracts to mainnet standards as part of testnet operations; that
  belongs to audit/mainnet work.

## Assumptions

- Testnet identities and keys are throwaway and never reused elsewhere.
- Admin addresses on testnet differ from any planned mainnet addresses.
- Anyone can interact with deployed contracts; there is no access control beyond
  what the contracts themselves enforce.

## Review

Revisit this document when: a public demo is scheduled, a new contract is
deployed, upgrade/admin addresses change, or real value is ever introduced into
testnet-like flows. Report issues via `testnet/security/disclosure-process.md`.
