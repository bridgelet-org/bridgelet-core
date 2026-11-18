# Known and Undecided Differences: Future Mainnet vs Current Testnet

## Purpose

This page separates what the repository actually says about the current
Bridgelet testnet from what it does **not** say about a future mainnet. There is
no mainnet deployment record in this tree. "Mainnet" below therefore means a
hypothetical production deployment, not a described or committed rollout.

Use the labels literally:

- **Known** — recorded in repository files or source.
- **Unknown/Undecided** — no repository decision exists.
- **Do not infer** — a tempting conclusion that these files do not support.

For known documentation conflicts that are already recorded, see
[`terminology-drift-log.md`](terminology-drift-log.md).

---

## Known

| Topic | Current testnet, as recorded | Future mainnet, as recorded |
|---|---|---|
| Deployment status | [`../registry/contract-status.md`](../registry/contract-status.md) records all four contracts as undeployed, IDs `TBD`/`N/A`, `Last Verified: Never`. Conflicting artifact IDs exist but are unverified. | No mainnet deployment record exists. No mainnet contract IDs or hashes are recorded here. |
| Contract set and wiring | The workspace has four contracts. The current `deploy-testnet.sh` contains deploy/initialize steps for all four, but the canonical registry still says none is deployed. `reserve_contract` is not consumed by `ephemeral_account`, and `account_factory` wiring/tooling is documented as incomplete. | Whether all four would be deployed, and whether Reserve/AccountFactory would be wired in, is **not decided here**. |
| Deployment automation | Testnet deployment is manual via [`deploy-testnet.yml`](../../.github/workflows/deploy-testnet.yml) `workflow_dispatch`; its deploy step is active and runs `scripts/deploy-testnet.sh`. A redeploy changes IDs and destroys contract state. | No mainnet deploy workflow, approval flow, or artifact process is defined in this tree. |
| Network settings | Passphrase `Test SDF Network ; September 2015`; RPC `https://soroban-testnet.stellar.org`; Horizon `https://horizon-testnet.stellar.org`. See [`network-config.md`](network-config.md). | The mainnet passphrase `Public Global Stellar Network ; September 2015` is named in `network-config.md`, which also says endpoints and contract IDs differ. Exact mainnet endpoints are not established in this tree. |
| Signer and admin key roles | The deploy script requires `SIGNER_SECRET_KEY`, `AUTHORIZED_SIGNER_PUBLIC_KEY`, `CREATOR_ADDRESS`, `RECOVERY_ADDRESS`, and optional `RESERVE_ADMIN_ADDRESS`. These are inputs, not recorded values. Admin/signer keys must not be reused across environments. No named holder is recorded. | No custody, rotation, or separation-of-duties decision is recorded. [`../security/admin-key-hygiene.md`](../security/admin-key-hygiene.md) states expectations but names no owner. |
| Reset and redeploy | SDF testnet resets are documented, with no cadence, notice commitment, or named authority. Bridgelet redeploys are manual and must update the changelog and deployment records. See [`../policies/reset-policy.md`](../policies/reset-policy.md). | No mainnet reset, redeploy, upgrade, or emergency-change policy exists in this repository. |
| Version tags | `testnet-v1.x` and `testnet-v2.x` are defined in [`../policies/versioning-scheme.md`](../policies/versioning-scheme.md), but no recorded deployment carries a tag. | No mainnet tag scheme is decided. |
| Audit and disclosure | Root status: "Active Development - MVP. Not audited." Private vulnerability reporting is recorded as disabled; the available process is lightweight and best effort. | No mainnet audit scope, embargo, disclosure policy, or advisory process exists here. [`../security/disclosure-process.md`](../security/disclosure-process.md) describes what such a policy would require. |
| Data and funds | Testnet is not a system of record. Test XLM is worthless, contract state is transient, and redeploy or reset can erase recorded payments, nonces, and config. See [`../policies/data-retention.md`](../policies/data-retention.md). | No production data-retention, funds-handling, recovery, or user-data policy is recorded. The data-retention policy explicitly puts a hypothetical mainnet out of scope. |
| Support and ownership | Best-effort maintainers, no on-call rotation, no SLA, no named owner. See [`contact-and-ownership.md`](contact-and-ownership.md). | No mainnet support, ownership, or escalation model is recorded. |

---

## Unknown/Undecided

These are open questions. Writing them down here does not decide them.

1. **Which contracts would be deployed and wired?** The current testnet script
   attempts all four, while older docs and the canonical registry disagree about
   deployment and wiring. Whether a mainnet release would include
   `reserve_contract` or `account_factory`, and whether `ephemeral_account`
   would read the reserve, is undecided.
2. **Who holds the deployer/admin key?** No human or team is named. The expected
   practice is a dedicated, named, non-reused holder; the repository does not
   establish one.
3. **Who holds the sweep-signing key, and how is it rotated?** The signer is an
   Ed25519 public key supplied to `initialize`. There is no on-chain rotation
   path for `authorized_signer`, and no mainnet custody decision is recorded.
4. **Who may authorize a deploy, upgrade, destination change, or emergency
   intervention?** The contact roles are unverified placeholders and current
   policy documents leave deployment authority undecided.
5. **What notice period, if any, applies before a production change?** A
   testnet notice policy exists as a checklist, not an enforced control. It does
   not transfer to mainnet.
6. **What are the exact mainnet endpoints and client configuration values?**
   Only the passphrase and "different endpoints/IDs" are stated here.
7. **Will deployment tags be applied, and what do they mean?** The testnet tag
   scheme exists but is not applied to a recorded deployment. A mainnet scheme
   is not defined.
8. **What audit scope and disclosure process are required?** "Not audited" is
   the current status; no future audit date, scope, or report is committed.
9. **How would production funds, user data, backups, and recovery be handled?**
   The testnet posture explicitly permits losing all on-chain state. No
   production equivalent exists.
10. **Who maintains production docs and responds to incidents?** No named
    owner, review policy, on-call rotation, or alert route exists today.

---

## Do not infer

- **Do not infer liveness from a script or artifact.** The current deploy script
  contains steps for all four contracts and deployment files contain concrete
  IDs, but the canonical registry records none as deployed and no on-chain
  verification is recorded here.
- **Do not infer mainnet IDs, hashes, or native-asset addresses from testnet
  values.** They are network-specific. Never reuse a testnet address or hash as
  a mainnet configuration value.
- **Do not infer mainnet endpoints from the testnet URLs.** The passphrase is
  named as different; the exact mainnet RPC and Horizon URLs are not
  established by this tree.
- **Do not infer that a documented "Unreleased" feature is implemented.**
  Changelog claims about namespaced error codes, an `is_initialized()` getter,
  and a `#1007` Standalone-passphrase check are contradicted by source; see
  [`terminology-drift-log.md`](terminology-drift-log.md).
- **Do not infer that testnet permissionless access implies no production
  controls.** [`../policies/versioning-scheme.md`](../policies/versioning-scheme.md)
  describes public testnet access only.
- **Do not infer that "Not audited" on testnet means audited elsewhere.** There
  is no audit artifact in this repository.
- **Do not infer that the unverified `EphemeralAccount::sweep()` signature path
  is safe on mainnet.** It is a documented testnet caveat and a known concern
  for future security review; see
  [`../security/unverified-signature-path-warning.md`](../security/unverified-signature-path-warning.md).
- **Do not infer a reset cadence or notice commitment.** The repository
  explicitly records neither for testnet and says nothing about a production
  equivalent.
- **Do not infer ownership, SLA, or response times from the contact labels.**
  Those labels are unverified placeholders in
  [`contact-and-ownership.md`](contact-and-ownership.md).
- **Do not infer that testnet data handling is acceptable for real funds or
  user data.** The opposite is documented in
  [`../policies/data-retention.md`](../policies/data-retention.md).

---

## Related documentation

- [`overview.md`](overview.md) — current testnet deployment overview
- [`terminology-drift-log.md`](terminology-drift-log.md) — known conflicting
  testnet claims
- [`../registry/contract-status.md`](../registry/contract-status.md) — canonical
  status registry
- [`../policies/reset-policy.md`](../policies/reset-policy.md) — reset and
  redeploy behavior
- [`../policies/versioning-scheme.md`](../policies/versioning-scheme.md) —
  proposed tags and testnet access model
- [`../policies/data-retention.md`](../policies/data-retention.md) — testnet
  data posture and production gap
- [`../policies/breaking-change-notice.md`](../policies/breaking-change-notice.md)
  — notice process and known source discrepancies
- [`../security/disclosure-process.md`](../security/disclosure-process.md) —
  lightweight disclosure process, not a mainnet policy
- [`contact-and-ownership.md`](contact-and-ownership.md) — role labels and
  ownership gaps
- [`network-config.md`](network-config.md) — current network values and the
  mainnet passphrase note

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-11-18 | Initial known/undecided comparison for a hypothetical future mainnet and explicit no-inference list |
