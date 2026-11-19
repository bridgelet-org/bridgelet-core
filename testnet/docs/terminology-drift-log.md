# Testnet Terminology and Fact Drift Log

## Purpose

This log records **testnet-facing terminology and fact drift**: claims in this
repository that disagree with another file in the repository, with the source,
or with the absence of a referenced artifact. It is a map of known conflicts,
not a set of corrections to the source documents and not evidence of a live
deployment.

Use [`comparison-with-mainnet-plans.md`](comparison-with-mainnet-plans.md) for
the separate question of what is known or undecided about a future mainnet.

## Source precedence

When a testnet-facing claim is disputed:

1. **On-chain observation** is the only proof of liveness, IDs, hashes, and
   current state.
2. **Contract source** is authoritative for interfaces, error codes, storage
   keys, and behavior in a build.
3. [`../registry/contract-status.md`](../registry/contract-status.md) is the
   canonical quick-reference for deployment status.
4. Deployment records and artifacts are **historical claims** until verified;
   they are not liveness proof.
5. Prose in guides, changelogs, comments, and templates is the weakest evidence.

At the last review recorded here, the canonical registry still lists all four
contracts as undeployed with `TBD`/`N/A` IDs and `Last Verified: Never`, despite
the conflicts below. Do not resolve a conflict by copying a concrete ID from an
older artifact.

## Entry format

Add one short entry per distinct claim. Do not merge different failure modes
such as "exists in source", "covered by a script", and "verified live".

```markdown
### D-NNN — Short topic

| Field | Value |
|---|---|
| Logged | YYYY-MM-DD |
| Claim | The exact claim, quoted or closely paraphrased |
| Where | File and section that makes the claim |
| Conflict | Source, canonical registry, missing file, or other claim |
| Safe reading | What a reader should do until the conflict is resolved |
| Evidence | Links to both sides of the conflict |
| Status | open | resolved-by-doc-fix | resolved-by-code-fix | resolved-by-verification |
```

`unassigned` is an acceptable owner value; do not invent an owner. Link to the
commit or issue that resolves an entry when one exists.

## Known drift entries

| ID | Logged | Topic | Claim and conflict | Safe reading | Evidence |
|---|---|---|---|---|---|
| D-001 | 2026-11-18 | Contract count claims | The source tree has four contracts. Older deployment prose and artifacts present four IDs as current; `overview.md` says the deploy tooling covered only two; the current `deploy-testnet.sh` now contains deploy steps for all four. The canonical registry records **zero** deployed. | Separate "four contracts in the workspace", "four contracts a script attempts to deploy", and "four contracts verified live". Only the registry plus an on-chain check answers the last one. | [`overview.md`](overview.md), [`../registry/contract-status.md`](../registry/contract-status.md), [`../../scripts/deploy-testnet.sh`](../../scripts/deploy-testnet.sh), [`../../README.md`](../../README.md) |
| D-002 | 2026-11-18 | Nonexistent local-testing script | Earlier README drafts referenced `scripts/test-local.sh`. The current root README explicitly says it does not exist; the FAQ names `scripts/test.sh` as the local entrypoint. | Do not cite or run `test-local.sh`. `scripts/test.sh` is a unit-test entrypoint, not a local sandbox deployment. | [`../../README.md`](../../README.md) §Testing, [`faq.md`](faq.md) §Is there a local sandbox or `test-local.sh` flow? |
| D-003 | 2026-11-18 | Stale null-placeholder claim | `network-config.md` says `deployments/testnet.json` "still contains `null` placeholders". The file currently contains concrete `C...` IDs. `reset-policy.md` already records the contradiction. | Treat the note as stale. Concrete IDs in that file remain unverified deployment claims; they do not override the canonical registry. | [`network-config.md`](network-config.md), [`../../deployments/testnet.json`](../../deployments/testnet.json), [`../policies/reset-policy.md`](../policies/reset-policy.md) |
| D-004 | 2026-11-18 | Registry vs deployment artifact IDs | The canonical registry records IDs as `TBD`/`N/A` and `Last Verified: Never`. `deployment-artifacts/contract-ids.txt`, `deployments/testnet.json`, `network-config.md`, and the changelog contain concrete historical IDs; a later commit updated the artifact and JSON without reconciling the registry or docs. | A copied ID is a claim, not liveness. Use the registry for status and verify any ID at runtime before use. Do not mix the older and newer ID sets. | [`../registry/contract-status.md`](../registry/contract-status.md), [`../../deployment-artifacts/contract-ids.txt`](../../deployment-artifacts/contract-ids.txt), [`../../deployments/testnet.json`](../../deployments/testnet.json), [`changelog.md`](changelog.md) |
| D-005 | 2026-11-18 | Reserve/AccountFactory deployment claims | Older prose and registry notes say these contracts are not built, not included in the deploy script, and not deployed. The current `deploy-testnet.sh` comments and code say it deploys and initializes both. The canonical registry still says not deployed, and cross-contract wiring into `EphemeralAccount` is documented as absent. | A script capability is not a deployment record, and a deployment record is not liveness. Keep "not wired" distinct from "not deployed". Do not infer that either contract is live. | [`overview.md`](overview.md), [`../registry/reserve-contract-status.md`](../registry/reserve-contract-status.md), [`../registry/account-factory-status.md`](../registry/account-factory-status.md), [`../../scripts/deploy-testnet.sh`](../../scripts/deploy-testnet.sh), [`../registry/contract-status.md`](../registry/contract-status.md) |
| D-006 | 2026-11-18 | Referenced Friendbot guide is absent | `testnet/config/friendbot.md` is cited as the reference for funding patterns and rate limits, but no such file exists. `testnet/faucet/README.md` does exist and covers basic funding. | Treat the references as broken. Link to the existing faucet README until the referenced guide is created or the links are fixed. | [`../examples/integration-test-harness-notes.md`](../examples/integration-test-harness-notes.md), [`../tooling/decision-log.md`](../tooling/decision-log.md), [`../tooling/faucet-helper-spec.md`](../tooling/faucet-helper-spec.md), [`../faucet/README.md`](../faucet/README.md) |
| D-007 | 2026-11-18 | Incorrect reset-notice path | The reset announcement template tells integrators that updated IDs and WASM hashes will be published in `config/testnet_contracts.json`. That path does not exist. `reset-policy.md` names `deployment-artifacts/contract-ids.txt` and `deployments/testnet.json` instead. | Do not use the template as-is. Correct the destination before publishing a notice. | [`../reset/communication-template.md`](../reset/communication-template.md), [`../policies/reset-policy.md`](../policies/reset-policy.md) |
| D-008 | 2026-11-18 | Error-code namespacing claim vs source | The changelog says errors are namespaced per contract (`1000–1999`, `2000–2999`, `3000–3999`), and support gives an example like `#2003`. The source enums are flat (`1–15`, `1–13` with `12` unused, and `1–6`), with no offset helper. | Treat namespacing as not implemented. Decode against the source enums; do not assume contiguous codes. | [`changelog.md`](changelog.md) §Unreleased, [`support.md`](support.md), [`../examples/error-handling.md`](../examples/error-handling.md), [`../policies/contribution-guide.md`](../policies/contribution-guide.md) |
| D-009 | 2026-11-18 | Standalone passphrase claim vs source | The changelog says `initialize` will check a network passphrase whose expected value on `main` is the Standalone passphrase and that testnet calls would fail with `Error(Contract, #1007)`. Source searches find no passphrase check, no `1007`, and no `Standalone` in contract or tool Rust files. `getting-started.md` separately assigns `#1007` to `Unauthorized`. | Treat the changelog text as an unimplemented forecast, not current behavior. Use the Testnet passphrase from `network-config.md` exactly; do not code against `#1007`. | [`changelog.md`](changelog.md) §Unreleased, [`../policies/breaking-change-notice.md`](../policies/breaking-change-notice.md), [`../examples/error-handling.md`](../examples/error-handling.md), [`../policies/contribution-guide.md`](../policies/contribution-guide.md), [`network-config.md`](network-config.md) |

## Reviewer checklist

Before writing or approving a testnet-facing claim:

- [ ] Does the claim say "exists in source", "covered by a script", or "verified
      live"? Do not let those mean the same thing.
- [ ] Does [`../registry/contract-status.md`](../registry/contract-status.md)
      agree, or has the disagreement been logged here?
- [ ] Was any contract ID, WASM hash, or admin address verified at runtime, or
      only copied from a file?
- [ ] Does every referenced path exist? Check `testnet/config/friendbot.md` and
      `config/testnet_contracts.json` class references especially carefully.
- [ ] Are signatures, error codes, `AccountStatus` values, and passphrases
      checked against source rather than another document?
- [ ] Is an "Unreleased" changelog item labeled as a forecast, not an
      implemented feature?
- [ ] Are reset, notice, ownership, and mainnet items kept as
      Known/Unknown/Undecided rather than silently decided in prose?
- [ ] Are secrets and signing material absent from examples, logs, and issues?
- [ ] If the drift was fixed, did the fixing change also update or close the
      relevant entry with an issue, commit, or verification link?

## Related documentation

- [`overview.md`](overview.md) — deployment overview and artifact trust levels
- [`comparison-with-mainnet-plans.md`](comparison-with-mainnet-plans.md) —
  known and undecided future-mainnet differences
- [`../registry/contract-status.md`](../registry/contract-status.md) — canonical
  deployment status
- [`../policies/reset-policy.md`](../policies/reset-policy.md) — worked example
  of ID and record drift
- [`../policies/contribution-guide.md`](../policies/contribution-guide.md) —
  source-over-document rule and unresolved ownership questions
- [`../policies/breaking-change-notice.md`](../policies/breaking-change-notice.md)
  — changelog-vs-source discrepancies
- [`contact-and-ownership.md`](contact-and-ownership.md) — who maintains the
  records, and the gaps in that ownership

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-11-18 | Initial log of nine known testnet-facing terminology and fact drifts, with entry format and reviewer checklist |
