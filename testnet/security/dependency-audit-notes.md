# Dependency Audit Notes: SDK and CLI Version Pairing

## Purpose

A record of which versions of the Soroban SDK, `stellar-cli`, and the Rust
toolchain produced the artifacts the testnet deployment is (or was) running —
kept so that when an SDK advisory or a behaviour change is reported, someone can
answer one question quickly:

> **Is the thing on testnet built from a version that the advisory affects?**

The premise is that **a deployed contract's behaviour is frozen at whatever
version built it.** Upgrading `soroban-sdk` in `Cargo.toml` changes nothing
about a WASM module already uploaded and already running. There is no runtime
SDK. So "we fixed it in the SDK" and "the deployment is fixed" are different
statements, and only one of them is about the deployment.

## Status

| Question | Answer as of 2026-09-26 |
|---|---|
| Are any contracts deployed? | **No.** All four are undeployed per `testnet/registry/contract-status.md`. |
| Is the WASM hash in `deployments/testnet.json` verified? | **No.** `config.ephemeralAccountWasmHash` is unverified, and `testnet/registry/wasm-hash-reference.md` records `TBD` for all four contracts. |
| Is the version chain below established? | **Documented, not established.** Every link is recorded; none has been confirmed against a live deployment, because there is none. |
| Does this document say the current deployment is unaffected by any SDK advisory? | **No, and it cannot.** There is no deployment. See [What this is not](#what-this-document-is-not). |

The table below is a *record of intent and provenance*, accurate as of the last
review. It is written ahead of the deployment for the same reason the rest of
`testnet/security/` is.

## The one coupling that matters most

**`tools/sweep-signer` pins `soroban-sdk = "22.0.0"` for a correctness reason,
not a tidiness reason.** Its `Cargo.toml` says so:

> Pin to the same soroban-sdk version bridgelet-core builds against, so
> `Address::to_xdr()` serializes identically to what the deployed contract
> expects. Update this if bridgelet-core's soroban-sdk version changes.

Why this is a correctness issue and not a version nit:

`SweepController` verifies every sweep signature against
`SHA256(destination ‖ nonce ‖ controller_id)`, where each component is
serialised with `soroban_sdk::Address::to_xdr()` — the SDK's own XDR
serialisation, **not** a hand-rolled encoding of the `G...`/`C...` StrKey. If
the tool and the contract serialise an address one byte differently, the tool
produces a digest over different bytes, and the contract rejects a signature
that is cryptographically perfectly valid. The failure is a bare
`ed25519_verify` trap with no error code and no diagnostics — the exact
symptom documented in
[`testnet/runbooks/failed-sweep-signature.md`](../runbooks/failed-sweep-signature.md)
as `Cause 2: Wrong Contract ID in Message` and `Cause 3: Destination Address
Mismatch`.

The format is spelled out in
[`docs/SIGNATURE_FORMAT.md`](../../docs/SIGNATURE_FORMAT.md), which warns at
length against hand-rolling the XDR encoding. That warning and this coupling
are the same fact seen from two directions.

**Therefore: if `soroban-sdk` is ever changed in `contracts/*/Cargo.toml`,
`tools/sweep-signer/Cargo.toml` must be changed in the same commit, and the
change must be checked against the *resolved* version in both lockfiles, not
against the declared requirement string.** See [The resolved version is what
builds](#the-resolved-version-is-what-builds) below — the declared string is
not the version that compiles.

## The version record

Declared versus resolved, and where the authoritative value lives. All values
below were read from the working tree on 2026-09-26.

| Component | Declared | Resolved in lockfile | Authoritative source | Enforced? |
|---|---|---|---|---|
| `soroban-sdk` — `ephemeral_account`, `sweep_controller`, `reserve_contract`, `account_factory`, `shared` | `"22.0.0"` | **22.0.11** | `contracts/*/Cargo.toml`; `Cargo.lock` | Yes, by `Cargo.lock` |
| `soroban-sdk` (dev-dependencies) | `{"version": "22.0.0", "features": ["testutils"]}` | 22.0.11 | `contracts/*/Cargo.toml`; `Cargo.lock` | Yes |
| `soroban-token-sdk` — `sweep_controller` only | `"22.0.0"` | 22.0.11 | `contracts/sweep_controller/Cargo.toml`; `Cargo.lock` | Yes |
| `stellar-xdr` (transitive) | not declared | 22.1.0 | `Cargo.lock` | Yes |
| `soroban-env-host` (transitive) | not declared | 22.1.3 | `Cargo.lock` | Yes |
| `soroban-spec`, `soroban-sdk-macros` (transitive) | not declared | 22.0.11 | `Cargo.lock` | Yes |
| `ed25519-dalek` — workspace | not declared directly | 2.2.0 | `Cargo.lock` | Yes, by `Cargo.lock` |
| `ed25519-dalek` — `tools/sweep-signer` | pinned `=2.1.1` | 2.1.1, with 3.0.0 also present in the graph | `tools/sweep-signer/Cargo.toml`; `tools/sweep-signer/Cargo.lock` | Yes |
| `stellar-strkey` — `tools/sweep-signer` | `"0.0.9"` | 0.0.9 | `tools/sweep-signer/Cargo.lock` | Yes |
| `stellar-cli` | **23.4.1, in prose only** | not recorded anywhere machine-readable | `scripts/build.sh`, `README.md`, `docs/testing.md`, `testnet/config/soroban-cli-identity-setup.md`, `testnet/registry/verify-contract-live.md`, `.github/workflows/*.yml` | **No** — see below |
| Rust toolchain | 1.89.0, in CI only | not recorded | `.github/workflows/deploy-testnet.yml` (`actions-rs/toolchain`, `toolchain: 1.89.0`) | Only in CI, and that workflow does not deploy |
| WASM build target | `wasm32v1-none` | n/a | `scripts/build.sh`; `scripts/deploy-testnet.sh` (`WASM_DIR`) | Yes, by the script |
| Protocol / ledger version | **not recorded** | n/a | nowhere | **No** |
| Deployment tag | `testnet-v1.x` | n/a | `testnet/policies/versioning-scheme.md` | Not applied to any recorded deployment |

### The resolved version is what builds

`soroban-sdk = "22.0.0"` is a **caret requirement**, not a pin. In Cargo it
means `>=22.0.0, <23.0.0`, which is satisfied by 22.0.11 — and 22.0.11 is what
both `Cargo.lock` files currently resolve to. `Cargo.toml` tells you the
admissible range; `Cargo.lock` tells you what was actually compiled.

Two consequences worth internalising:

1. **An advisory against 22.0.5 through 22.0.10 would not apply to a build
   resolved by these lockfiles**, and an advisory against 22.1.x would not
   apply at all. The resolved version is the one that matters.
2. **The two lockfiles can drift apart.** `Cargo.lock` and
   `tools/sweep-signer/Cargo.lock` are separate files, updated separately, and
   both currently resolve `soroban-sdk` to 22.0.11. If one is regenerated and
   the other is not, the tool and the contracts stop being built from the same
   SDK — and per the coupling above, that is a signature-correctness risk, not
   a tidiness problem.

### Is there a pinned `stellar-cli` version?

**A version is named in seven places and enforced in none of them.** This is
itself worth recording.

Where `23.4.1` appears:

- `scripts/build.sh` — `cargo install --locked stellar-cli --version 23.4.1`,
  followed immediately by "(or a newer stellar-cli release — check
  `stellar --version`)"
- `README.md` and `docs/testing.md` — same install line, in the prerequisites
- `testnet/config/soroban-cli-identity-setup.md` and
  `testnet/registry/verify-contract-live.md` — same version in a prerequisite
  list
- `.github/workflows/deploy-testnet.yml` and `.github/workflows/test.yml` —
  `cargo install --locked stellar-cli --version 23.4.1`

Where it is **not** pinned:

- No `.tool-versions`, no `rust-toolchain.toml`, no CI matrix, and no
  checked-in `stellar --version` output. The two CI workflows also install
  `stellar-cli` twice by two different mechanisms — an `install.sh` from
  `main` and then a `cargo install --version 23.4.1` — with no check that they
  agree.
- `testnet/docs/getting-started.md` says **"v22 or later"** for the same
  prerequisite. `scripts/build.sh` says 23.4.1 *or newer*. So the repository
  does not have one answer; it has three.
- Neither `stellar-cli` version nor Rust version affects the *WASM* that ends
  up deployed — `stellar contract build` invokes `cargo` for the contract
  crates, and the contract crates pin their own SDK through `Cargo.lock`. The
  CLI is a *deployment-time* and *diagnostics-time* tool. That narrows the
  blast radius; it does not eliminate it, because a CLI change can alter how a
  transaction is built, encoded, or simulated, and a wrong simulation is how
  you conclude a transaction will work when it will not.

**Conclusion to carry into triage: the SDK is pinned by a lockfile; the CLI is
pinned by a habit.** If a CLI behaviour change is the concern, this repository
cannot currently answer the question, and the fix is to capture
`stellar --version` output in a deployment record.

## The build-to-deployed-artifact chain

The chain you need in order to answer "is the live WASM in range":

```
soroban-sdk requirement   soroban-sdk in Cargo.lock   build commit   deployed commit   WASM hash on testnet
  "22.0.0"        ->          22.0.11             ->     ?        ->   741aec2      ->   5e667ea0...62c58e
(caret range)              (what compiled)            (unknown)   (changelog)         (unverified)
```

What each link is sourced from:

| Link | Source | State |
|---|---|---|
| Declared SDK requirement | `contracts/*/Cargo.toml` | Recorded. Identical across all five workspace members. |
| Resolved SDK version | `Cargo.lock` | Recorded: 22.0.11. |
| Build commit — the commit the WASM was compiled from | — | **Unknown.** No artefact in the repository records it. |
| Deployed commit | `testnet/docs/changelog.md`, `## 2026-07-12` entry: `Deployed commit: 741aec2` | Recorded. |
| Deployment timestamp | `deployments/testnet.json`, `deployedAt: 2026-07-12T10:29:25Z` | Recorded. |
| WASM hash | `deployments/testnet.json`, `config.ephemeralAccountWasmHash`; repeated in `testnet/docs/network-config.md` and `testnet/docs/changelog.md` | Recorded but **unverified**. |
| Deployed commit → SDK resolved version | would require rebuilding commit `741aec2` and reading *its* `Cargo.lock` | **Not done.** The current `Cargo.lock` reflects `main`, not that commit. |

### The weak link, stated plainly

The chain is documented end to end and **established at exactly one point**: the
lockfile on `main`, as read today.

Three things are missing, in increasing order of how much they matter:

1. **Nothing is deployed.** `testnet/registry/contract-status.md` records all
   four contracts as undeployed with IDs `TBD`. There is no live WASM for the
   chain to terminate in.
2. **The recorded WASM hash is unverified.** It appears in three places and
   none of them is a measurement.
   `testnet/registry/wasm-hash-reference.md` records `TBD` for all four
   contracts, and
   [`testnet/security/README.md`](README.md) says to treat the `2026-07-12`
   IDs as unverified until someone confirms them on-chain per
   `testnet/registry/verify-contract-live.md`.
3. **The `deployedAt` commit and the WASM hash are not tied together.** The
   changelog says commit `741aec2` was deployed; `deployments/testnet.json`
   says a WASM hash was installed; nothing records that the second was produced
   by building the first. `scripts/deploy-testnet.sh` builds and deploys in one
   run and writes `deployments/testnet.json`, but it records **no commit SHA**
   — it writes `deployedAt` and the IDs and the WASM hash, and stops. So even
   a perfectly executed deploy produces this file without the one field that
   would make the chain closable.

Point 3 is the actionable one, and it is a small fix: the deploy record needs
a commit SHA. Whether that is a change to `scripts/deploy-testnet.sh` is not
this document's call — `scripts/` is explicitly out of scope for this work, and
`testnet/tooling/decision-log.md` records that boundary. Recording the
requirement here is the contribution; the change is a separate, scoped piece of
work.

## Triage when an SDK advisory lands

A procedure, not a promise. Run it in order; do not skip to step 4.

**Step 1 — Establish what the advisory actually covers.** Record the advisory
identifier, the affected version range, and whether it is a security issue, a
behaviour change, or a build/toolchain change. A behaviour change with no CVE
is still worth running; "not a CVE" is not a reason to skip.

**Step 2 — Read the recorded resolved version.** From the table above:
`soroban-sdk` 22.0.11, `soroban-token-sdk` 22.0.11, `stellar-xdr` 22.1.0.
Use the *resolved* versions, not the `"22.0.0"` requirement strings.

**Step 3 — Compare.**

| If the affected range contains the resolved version | Then |
|---|---|
| Yes, and the deployed WASM was built from a commit whose `Cargo.lock` resolves inside the range | The deployment is in range. Triage continues. |
| Yes, but the deployed WASM was built from a commit resolving **outside** the range | The deployment is not affected. The *source* is. Say so explicitly and separately, because they are different statements and only one is about testnet. |
| No overlap at all | No action on the deployment. Update the record if the advisory changes the picture. |

**Step 4 — Determine whether the deployed WASM was actually built inside the
range.** This is the step that needs evidence, and it is the one people skip.
The two legitimate ways to establish it:

- Read the `Cargo.lock` **at the deployed commit**, not at `main`. Checkout the
  deployed commit, read its `Cargo.lock`, compare.
- Reproduce the build. `testnet/registry/wasm-hash-reference.md` Method 3
  describes rebuilding at a given commit and comparing hashes. Note the caveat
  in that document: the same WASM hash from two rebuilds is evidence; a
  mismatch may be a non-deterministic build rather than a different commit.
  Issue #532 ("Reproducible build verification confirming deployed WASM hash
  matches committed source") is the open work on that.

**Step 5 — Act, per branch.**

| Branch | Action |
|---|---|
| Deployment confirmed in range, exploitability plausible on testnet | Report through [`disclosure-process.md`](disclosure-process.md). Log the investigation in [`reentrancy-observation-log.md`](reentrancy-observation-log.md) if it touched a sweep path. |
| Deployment confirmed in range, no plausible testnet exploitation | Record the analysis here with the date and the reasoning. A documented "we looked and here is why it does not apply" is worth more than silence. |
| Deployment confirmed out of range | Bump the SDK in a separate commit from any behaviour change, then refresh this document. |
| **Record stale or missing** | **The current state. Continue below.** |

### The stale-or-missing branch — the one that matters today

Right now, steps 1–3 are answerable and step 4 is not. The honest output of
this procedure, run against the repository as it stands, is:

> The resolved SDK version is known. The deployed WASM is not. No Bridgelet
> contract is deployed. Therefore **whether any deployment is in range cannot
> be determined from this repository**, and no statement to the contrary should
> be published from it.

Three things follow, and they are the useful output:

1. **Do not publish a reassurance.** "We are not affected" is not derivable
   from anything recorded here.
2. **Do not upgrade the SDK as a response to the advisory, in isolation.** An
   SDK bump changes `Address::to_xdr()` behaviour potentially, and it must
   move `tools/sweep-signer` with it. An advisory-driven upgrade that skips
   the coupling check is a signature-correctness regression caused by a
   security response.
3. **Record the advisory in this document's version history** with the date,
   the affected range, the resolved version at that time, and the outcome
   (`unanswerable — no deployment`, if that is the outcome). A dated record of
   a question that could not be answered is more useful next time than an
   empty one, because it shows the question was asked.

## Refresh procedure and cadence

**When this document must be updated:**

| Trigger | Required change |
|---|---|
| `soroban-sdk` (or `soroban-token-sdk`, `stellar-xdr`) changes in any `Cargo.toml` | The version table, the version history, and — check — `tools/sweep-signer/Cargo.toml` |
| `Cargo.lock` or `tools/sweep-signer/Cargo.lock` is regenerated | The resolved-version column, even if the declared string is unchanged |
| `stellar-cli` or the Rust toolchain version changes | The corresponding row, and whether the pin is now enforced |
| A deployment happens, or `testnet/registry/contract-status.md` changes | The build-to-deployed-artifact chain, and the "weak link" section |
| An SDK advisory lands | The triage record described above |
| A WASM hash is verified on-chain | The hash, its verification date, and the commit it was rebuilt from |

**Who.** Whoever made the change. There is no review role for this document
specifically; the useful constraint is that whoever updates the SDK updates
`tools/sweep-signer` in the same commit, and the PR description says so.

**Cadence.** **None is established, and inventing one would be false
precision.** What exists is a *linkage* rather than a schedule:
`testnet/docs/changelog.md` has its own "How to update this file" process,
which requires an entry at the top for every run of `scripts/deploy-testnet.sh`,
every in-place upgrade, and every SDF testnet reset taking the deployment down.
A redeploy is therefore already a moment where this document must be revisited,
and it is the right moment, because the redeploy path is what produces new
contract IDs and a fresh WASM hash.

Cross-reference
[`testnet/policies/reset-policy.md`](../policies/reset-policy.md) for the reset
side of the same trigger, and
[`testnet/policies/versioning-scheme.md`](../policies/versioning-scheme.md) for
the `testnet-v1.x` deployment tags — noting that no recorded deployment carries
one, so a redeploy is currently the *only* trigger that reliably fires.

**The one thing a cadence would buy is freshness of the `stellar-cli` pin.** If
a single recurring maintenance item is ever agreed, "check the SDK/CLI versions
and update this file" is the highest-value candidate, because the SDK is pinned
by a lockfile and the CLI is not pinned at all.

## What this document is not

Stated explicitly, because a document with a title like this one attracts
assumptions it cannot support.

| It is not | Why |
|---|---|
| A CVE feed, advisory mirror, or subscription | Nothing here watches anything. Someone has to notice an advisory and run the triage above. |
| A continuous dependency scanner | No `cargo audit`, no Dependabot, no scheduled job. This is a hand-maintained record. |
| **A claim that the current deployment is unaffected by any advisory** | **There is no deployment.** This is the one thing this document must never be cited for. It records versions and a broken chain; it establishes no conclusion about any live artefact. |
| A lockfile | The lockfiles are the authoritative source. This file describes them and points at them. |
| A compatibility matrix | Nothing in the repository tests across SDK versions. The claim "22.0.11 works" is a build result, not a tested matrix. |
| A place to record deployment IDs | Those belong in `deployments/testnet.json` and `testnet/registry/`. This file only points at them. |

---

## Related Documentation

- [`README.md`](README.md) — index of this directory; the standing rules this
  document inherits
- [`disclosure-process.md`](disclosure-process.md) — the route for a finding
  that triage produces
- [`reentrancy-observation-log.md`](reentrancy-observation-log.md) — where an
  investigation touching a sweep path gets logged
- [`tools/sweep-signer/`](../../tools/sweep-signer) — the tool whose SDK pin is
  the correctness coupling above (referenced, not modified)
- [`docs/SIGNATURE_FORMAT.md`](../../docs/SIGNATURE_FORMAT.md) — the signed
  message format, and the warning against hand-rolled XDR encoding
- [`testnet/runbooks/failed-sweep-signature.md`](../runbooks/failed-sweep-signature.md)
  — the diagnostic for a signature-format divergence
- [`testnet/registry/wasm-hash-reference.md`](../registry/wasm-hash-reference.md)
  — WASM hash verification; the basis of step 4 above
- [`testnet/registry/contract-status.md`](../registry/contract-status.md) — the
  deployed/undeployed state this document is written ahead of
- [`testnet/registry/upgrade-history.md`](../registry/upgrade-history.md) —
  where a redeployed or upgraded WASM hash should be recorded
- [`testnet/docs/changelog.md`](../docs/changelog.md) — the "How to update this
  file" process that defines the refresh trigger
- [`testnet/policies/versioning-scheme.md`](../policies/versioning-scheme.md) —
  deployment tags
- [`testnet/tooling/decision-log.md`](../tooling/decision-log.md) — why
  `scripts/` is out of scope for this work

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-26 | Initial SDK/CLI version pairing record and advisory triage procedure |
