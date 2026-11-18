# Historical Testnet Deployment Generations

## Status

This file is an **archive of recorded contract-ID claims**, not a deployment
registry and not evidence of liveness. The canonical state is
[`testnet/registry/contract-status.md`](../registry/contract-status.md): all
four contracts are recorded as undeployed, with `TBD`/`N/A` IDs and `Never`
last verified. Nothing below overrides that table.

Two conflicting sets of IDs are checked into this repository. This file keeps
both, with their sources, instead of choosing one. Neither set has a recorded
on-chain verification, so every row is an **unverified historical record**.

| Question | Answer |
|---|---|
| Are these currently deployed contracts? | **Not established.** The canonical registry says no contract is deployed. |
| Why keep the IDs at all? | Old bug reports, screenshots, fixtures, and CI logs contain them. Without a dated archive they are indistinguishable from current values. |
| Does a newer date mean a verified deployment? | **No.** It only means a newer file was written. |
| Who may add a generation? | The deployer/operator role that performs the change; see [`pre-reset-freeze-notice.md`](pre-reset-freeze-notice.md). No authorisation policy is invented here. |

---

## Table schema

Every generation uses the same fields. Leave a field explicitly
`not recorded` rather than filling it from another generation.

| Field | Required | Rule |
|---|---|---|
| Generation label | Yes | `GEN-<n>`, assigned in order of the **source date**, never by guessed deployment order. |
| Recorded from (UTC) | Yes | Earliest date in the source that names the IDs. Use `YYYY-MM-DD`, or the full timestamp if the source provides one. |
| Recorded until (UTC) | Yes | Date of the next generation's source, or `unknown` when no later record exists. This is a record-keeping boundary, **not** a claim that a contract disappeared on that date. |
| Contract IDs | Yes | One row per contract, copied verbatim. Write `unchanged`, `TBD`, or `not recorded` where that is what the source says. |
| WASM hash | Yes | Copy only a hash the source records. Otherwise `not recorded`. Never reuse a hash across generations to fill a gap. |
| Deployed commit | Yes | Copy only if the source records one. Artifact files without a commit field get `not recorded`. |
| Source | Yes | Repository path and section or key. A generation without a source is not admissible here. |
| Verification | Yes | Always `unverified historical record` unless the canonical registry and `verify-contract-live.md` independently support a stronger statement. |
| Notes | No | Differences from other generations, missing changelog entries, or conflicting statements. |

---

## Generations at a glance

| Generation | Date range as recorded | Primary source(s) | Verification |
|---|---|---|---|
| `GEN-1` | 2026-07-12 -> unknown; superseded **in records** by `GEN-2` on 2026-10-10 | [`testnet/docs/changelog.md`](../docs/changelog.md) section "2026-07-12: Initial testnet deployment"; [`testnet/docs/network-config.md`](../docs/network-config.md) section "Deployed contract IDs" | Unverified historical record |
| `GEN-2` | 2026-10-10 -> unknown; no later record in this repository | [`deployments/testnet.json`](../../deployments/testnet.json) `deployedAt: 2026-10-10T04:38:14Z`; [`deployment-artifacts/contract-ids.txt`](../../deployment-artifacts/contract-ids.txt) | Unverified historical record |

"Superseded in records" means only that a newer file exists. It does not mean
`GEN-1` was ever deployed, that `GEN-2` replaced it on-chain, or that either
set resolves today.

---

## `GEN-1` - 2026-07-12 record

| Field | Value |
|---|---|
| Generation label | `GEN-1` |
| Recorded from | 2026-07-12 |
| Recorded until | unknown; `GEN-2` is dated 2026-10-10 |
| Deployed commit | `741aec2` (recorded in the changelog entry) |
| Verification | Unverified historical record |

| Contract | Recorded ID | WASM hash | Source |
|---|---|---|---|
| EphemeralAccount | `CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3` | `5e667ea0687341bdccc81538492143b573777dd04b2450cdeb89b02cee62c58e` | changelog section 2026-07-12; `network-config.md` section "Deployed contract IDs" and "Other useful testnet values" |
| SweepController | `CBEU4X5MNGOECBSTNEUFMBALH2YI5YV4UIH7YXRNOLR2DNLZQD4Z5KWE` | not recorded per contract | changelog section 2026-07-12; `network-config.md` section "Deployed contract IDs" |
| ReserveContract | `CACYMICFLHSPKVMCK336IEGCWAQB4XBC7P7HTJSQY3R7Y2YASE4AX3MS` | not recorded per contract | changelog section 2026-07-12; `network-config.md` section "Deployed contract IDs" |
| AccountFactory | `CARBMZY4466SWP3RTN3DR2F4JYNR5UCXD2ICFISV2YI4TS3AAZV2TQ24` | not recorded per contract | changelog section 2026-07-12; `network-config.md` section "Deployed contract IDs" |

Notes:

- The changelog describes the shared `EphemeralAccount` instance as already
  initialized with an expiry ledger that has passed. That is a 2026-07-12
  claim, not a current state assertion.
- `network-config.md` also states that `deployments/testnet.json` contains
  `null` placeholders. That statement is stale: the JSON now contains IDs.
  See the drift example in
  [`testnet/policies/reset-policy.md`](../policies/reset-policy.md).

---

## `GEN-2` - 2026-10-10 record

| Field | Value |
|---|---|
| Generation label | `GEN-2` |
| Recorded from | 2026-10-10T04:38:14Z (`deployedAt` in `deployments/testnet.json`) |
| Recorded until | unknown; no later record |
| Deployed commit | not recorded; neither artifact file records a deployed commit |
| Verification | Unverified historical record |

| Contract | Recorded ID | WASM hash | Source |
|---|---|---|---|
| EphemeralAccount | `CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3` | `5e667ea0687341bdccc81538492143b573777dd04b2450cdeb89b02cee62c58e` (`config.ephemeralAccountWasmHash`) | `deployments/testnet.json`; hash value also appears in `GEN-1` sources |
| SweepController | `CAJB4RRM5JRPXLITC5JUA7Q3LU2WQJDRPXVKMORXWHJJBPZRYWDJ5V4S` | not recorded per contract | `deployments/testnet.json`; `deployment-artifacts/contract-ids.txt` |
| ReserveContract | `CBG3Y4GWFTL4D37JGV2XWAWC7EUQ7XGH4XM3SFLQYYSKPB2IYMAS54HY` | not recorded per contract | `deployments/testnet.json`; `deployment-artifacts/contract-ids.txt` |
| AccountFactory | `CDBY4L3NTLW4OBZZTY73VL3NNWO2AGFFGBRJVDL3IMKDDLFF2W57WK6H` | not recorded per contract | `deployments/testnet.json`; `deployment-artifacts/contract-ids.txt` |

Notes:

- The `EphemeralAccount` ID and its recorded hash are identical to `GEN-1`.
  Three IDs differ. Similarity is not evidence that the same instance is live.
- `testnet/docs/changelog.md` has no 2026-10-10 entry. Per
  [`reset-policy.md`](../policies/reset-policy.md), a redeploy without a
  changelog entry is not a properly recorded redeploy. This is why the
  generation is archived as a record of a file, not of a deployment.
- The artifact files contain public configuration values used at deploy time.
  Do not copy key material, seeds, or signatures into this archive.

---

## Recording a future generation

Create a generation only for a new deployment that changes contract IDs. A
hash-only in-place upgrade is recorded in
[`testnet/registry/upgrade-history.md`](../registry/upgrade-history.md) and
[`testnet/registry/wasm-hash-reference.md`](../registry/wasm-hash-reference.md),
not here.

- [ ] A new sequential `GEN-<n>` label is assigned from the UTC date of the
      deployment record, not from the date someone noticed it.
- [ ] The previous generation's `Recorded until` is set to that same date.
- [ ] All four IDs are copied from
      [`deployment-artifacts/contract-ids.txt`](../../deployment-artifacts/contract-ids.txt)
      and [`deployments/testnet.json`](../../deployments/testnet.json), with
      `unchanged`, `TBD`, or `not recorded` where applicable.
- [ ] Each WASM hash comes from the upload output or
      [`testnet/registry/wasm-hash-reference.md`](../registry/wasm-hash-reference.md);
      no placeholder is presented as a hash.
- [ ] The deployed commit and version label are copied from the dated
      [`testnet/docs/changelog.md`](../docs/changelog.md) entry.
- [ ] Sources are linked from every row.
- [ ] Verification stays `unverified historical record` until the canonical
      registry records the contracts as deployed and verified.
- [ ] The changelog entry and the
      [`testnet/reset/communication-template.md`](communication-template.md)
      announcement agree with this row.
- [ ] The same-change artefact list in
      [`testnet/policies/reset-policy.md`](../policies/reset-policy.md) is
      complete.

---

## Interpreting old bug reports and screenshots

| Evidence in the report | What it identifies | What it does **not** prove |
|---|---|---|
| SweepController ID `CBEU...` | The reporter was using a value recorded in `GEN-1` | That the contract was live, or that the report predates 2026-10-10 |
| SweepController ID `CAJB...` | The reporter was using a value recorded in `GEN-2` | That a redeploy happened, or that the contract is live now |
| EphemeralAccount ID `CB7Z...` | A value present in **both** generations | Which generation produced the report; use the controller ID, hash, ledger, or date instead |
| WASM hash `5e667...` | A hash present in both generations' records | A unique generation, or that the code was deployed |
| No ID, only an error message or screenshot of a UI label | Nothing assignable | A generation; do not infer one from the symptom |
| A date before 2026-07-12 | No recorded generation | Anything about which IDs were in use |

Procedure:

1. Copy the ID or hash **exactly** from the report and match it against the
   tables above. Do not match on a prefix as short as `C`, and do not complete
   a truncated ID from a screenshot.
2. If the evidence matches more than one generation, record `ambiguous`.
3. Treat a nonce, expiry ledger, payment count, or terminal status in an old
   report as belonging to that report's deployment epoch. A controller ID that
   differs between generations means the nonce is not comparable; see
   [`testnet/runbooks/nonce-desync.md`](../runbooks/nonce-desync.md).
4. Re-resolve any ID before using it, using
   [`testnet/registry/verify-contract-live.md`](../registry/verify-contract-live.md).
   A historical record is a claim to check, not a target to call.
5. State the generation label in the triage note when one can be established,
   and say `generation unknown` when it cannot. Do not upgrade an ambiguous
   report into evidence of a live deployment.

A screenshot that cites the old JSON pointer from the communication template
is using a path that does not exist in this repository. The real sources are
`deployment-artifacts/contract-ids.txt` and `deployments/testnet.json`.

---

## Related Documentation

- [`testnet/registry/contract-status.md`](../registry/contract-status.md) -
  canonical deployment status; overrides everything here
- [`testnet/docs/changelog.md`](../docs/changelog.md) - the dated record a
  generation is named from
- [`testnet/docs/network-config.md`](../docs/network-config.md) - published IDs
  and the stale-JSON statement
- [`deployments/testnet.json`](../../deployments/testnet.json) - `GEN-2`
  timestamp and deploy-time configuration
- [`deployment-artifacts/contract-ids.txt`](../../deployment-artifacts/contract-ids.txt)
  - `GEN-2` IDs
- [`testnet/registry/wasm-hash-reference.md`](../registry/wasm-hash-reference.md)
  - where future hashes must be recorded
- [`testnet/policies/reset-policy.md`](../policies/reset-policy.md) - the
  worked drift example and the same-change artefact list
- [`testnet/reset/communication-template.md`](communication-template.md) -
  announcements that must agree with a new generation
- [`testnet/reset/pre-reset-freeze-notice.md`](pre-reset-freeze-notice.md) -
  when a generation may be added
- [`testnet/registry/verify-contract-live.md`](../registry/verify-contract-live.md)
  - the read-only resolution procedure
- [`testnet/runbooks/nonce-desync.md`](../runbooks/nonce-desync.md) - why nonce
  values do not survive a controller change

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-11-18 | Initial archive of the 2026-07-12 and 2026-10-10 recorded ID generations as unverified historical records |
