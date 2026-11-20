# Dry-Running the Reset Procedure

Propose a way to validate the reset/redeploy procedure **before** the next real
one, so gaps are found in a calm review rather than during a live reset.

> **Status:** proposal. This repository does not currently contain the documents
> the issue assumes: there is no `testnet/reset/README.md` procedure and no
> `testnet/reset/post-reset-checklist.md`. What exists is
> `testnet/policies/reset-policy.md`, `testnet/reset/communication-template.md`,
> `testnet/registry/verify-contract-live.md`, `testnet/docs/changelog.md`, and
> `scripts/deploy-testnet.sh`. The dry run below is written against those, and its
> first output should be the missing procedure and checklist.

## Principles

1. **Never touch the shared registry IDs during a dry run.** Use a scratch
   identity and scratch contracts.
2. **Follow the documents literally.** If you must improvise a step, that is a gap.
3. **Record every gap** as an issue or a documentation fix.

## Dry-run approach

### A. Paper walkthrough (30–60 minutes, no network)

For each step in the reset procedure and checklist, check:

- Does the step name the exact file/command?
- Is each referenced file present (`ls` it)? Use a link check on `testnet/`.
- Does each step say who performs it and what success looks like?
- Is the order dependency-correct? (deploy before init; registry update after
  verification; announcement before execution per
  `testnet/policies/who-can-request-a-reset.md`.)
- Are rollbacks and "what if the deploy script fails" covered?
  (`scripts/deploy-testnet.sh` has known issues recorded in
  `testnet/registry/contract-status.md`.)

### B. Scratch redeploy on testnet

1. Create a **scratch deployer identity** funded via Friendbot
   (`testnet/faucet/README.md`), not the project key.
2. Run the deploy procedure with scratch values (new identity, a scratch signer
   public key) so nothing in the real registry changes.
3. Walk the **post-reset checklist** against the scratch deployment:
   - verify each contract with `testnet/registry/verify-contract-live.md`;
   - read nonce/admin/signer values and compare with the deployment output;
   - run one end-to-end example (`testnet/examples/signed-sweep-walkthrough.md`);
   - draft (do not send) the announcement from `testnet/reset/communication-template.md`;
   - draft the registry and changelog updates.
4. Time each phase and note which steps needed knowledge not in the docs.

### C. Failure injection (optional)

Repeat with a deliberately wrong input (wrong passphrase, unfunded deployer, stale
WASM hash) and confirm the docs' troubleshooting points to the right runbook
(`testnet/runbooks/wrong-network-passphrase.md`, `upgrade-verification.md`).

### D. State export rehearsal

Run `testnet/reset/state-export-before-reset.md` against the scratch deployment and
confirm the export is complete and interpretable.

## Dry-run report template

| Field | Value |
|---|---|
| Date / reviewer | |
| Documents followed (commit) | |
| Steps that were missing or wrong | |
| Steps needing undocumented knowledge | |
| Time taken per phase | |
| Issues opened | |
| Verdict (ready / not ready) | |

## Cadence

Run a dry run: before any planned redeploy, after any change to the deploy script
or the procedure documents, and at least once per quarter even without a
planned reset.

## Clean up

Discard the scratch deployer key; record the scratch contract IDs as scratch so
they are not mistaken for registry entries.
