# `testnet/` Link Audit

A pass confirming that cross-references between the `testnet/` documents resolve
to real files and anchors, and a repeatable way to re-run it.

## How it was run

```bash
python3 scripts/check-testnet-links.py
```

The script (`scripts/check-testnet-links.py`, added with this audit) checks every
relative Markdown link `[text](path#anchor)` in `testnet/**/*.md`, ignoring fenced
code blocks, and verifies that the target file exists and, for `.md` targets, that
the `#anchor` matches a heading slug. It also lists backticked repository paths
(`` `testnet/...`` , `docs/...`, `contracts/...`, `scripts/...`, `deployments/...`)
that don't exist, as **warnings** because some documents intentionally name
targets that are tracked separately. External URLs are not checked. Anchor
matching approximates GitHub's slug rules and can misjudge unusual headings.

## Result on the tree as it stood on `main` (before any pending PRs)

81 Markdown files (including `README.md` index files), 560 relative links.

| Check | Count |
|---|---|
| Relative links to a missing file | 2 |
| Relative links to a missing anchor | 3 |
| Backticked paths to missing files (warnings) | 15 |

## Fixed in this change (3 anchors)

| File | Broken anchor | Fixed to |
|---|---|---|
| `testnet/policies/reset-policy.md` | `#1-platform-level-reset-the-sdf-resets-testnet` | `#two-independent-kinds-of-reset` (the heading that contains that table) |
| `testnet/security/disclosure-process.md` | `#what-must-never-be-posted` | `#what-must-never-be-posted-publicly` |
| `testnet/security/disclosure-process.md` | `#what-would-count-as-making-this-heavier` | `#this-is-not-a-mainnet-disclosure-policy` (the section holding the "what would have to be true before a heavier policy is warranted" paragraph) |

## Still broken (needs an author's decision, not changed here)

| File | Reference | Problem |
|---|---|---|
| `testnet/docs/network-config.md`, `testnet/docs/overview.md` | `../../docs/testnet-deployment.md` | The file does not exist in `docs/`. `testnet/docs/overview.md` itself describes it as out of date relative to the deploy script. Restore it, or remove the links |
| `testnet/docs/faq.md`, `testnet/docs/overview.md` | `docs/testnet-deployment.md` (backticked) | Same missing file |

## Missing targets named in backticks (warnings)

These are referenced as if they exist, but are not in the tree:

| Missing target | Referenced from |
|---|---|
| `testnet/config/friendbot.md` | `examples/integration-test-harness-notes.md`, `tooling/decision-log.md`, `tooling/faucet-helper-spec.md` |
| `testnet/config/multi-asset-testing.md` | `examples/multi-asset-sweep.md` (the document says it may not be in your checkout) |
| `testnet/examples/create-and-fund-ephemeral-account.md` | `examples/signed-sweep-walkthrough.md`, `tooling/faucet-helper-spec.md` |
| `testnet/config/rpc-fallback.md` | `runbooks/rpc-outage.md` (the text calls it "separately tracked") |
| `contracts/account_factory/src/errors.rs` | `examples/error-handling.md` (deliberate; the text says the file does not exist) |

Several of these are acknowledged as pending in the referring text. The git
history shows `testnet/config/multi-asset-testing.md` was authored on 2026-09-23
but is not on `main`; check whether it was lost in a merge before rewriting it.

## Cross-references to the other documents in this batch

Documents added by other pull requests in the same batch (runbooks, policies,
integration notes and so on) link to each other. Those links resolve only after
those pull requests merge. **Re-run the script after the batch is merged** and fix
whatever is left.

## Keeping it healthy

- Run `python3 scripts/check-testnet-links.py` before merging documentation PRs
  that add or rename files under `testnet/`. Exit status is non-zero only for
  broken links, so it can be wired into CI later (not done here).
- Prefer linking an existing heading over restating its text, and re-check anchors
  when renaming headings.
- When a document names a not-yet-written target, say so in the text and track it
  in an issue.
