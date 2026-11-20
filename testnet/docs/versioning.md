# Which contract version is live on testnet?

This page documents a practical process for answering one question: *given a
contract ID on Stellar testnet, which source version is actually running?*

There is no automated deployment or version-tracking pipeline today. CI
workflows are disabled, every testnet deployment is a manual run of
`scripts/deploy-testnet.sh`, and the deploy record is a plain JSON file. That
means correlating a live contract ID back to source is a manual exercise. The
steps below make it repeatable.

> Network: Stellar testnet (`Test SDF Network ; September 2015`)
> Soroban RPC: `https://soroban-testnet.stellar.org`
> Horizon: `https://horizon-testnet.stellar.org`

## The short version

1. Read the contract ID from `deployments/testnet.json`.
2. Use `git blame` / `git log` on that file to find the commit that last changed
   the ID.
3. From that commit, find the `ephemeralAccountWasmHash` value and the source it
   was built from.
4. Read the live code hash on-chain and compare it with
   `config.ephemeralAccountWasmHash`.
5. If they match, you know which source built the deployed WASM. If not, the
   record is stale and you cannot trust it.

## Step 1 - Read the contract ID

`deployments/testnet.json` is the deployment record written by
`scripts/deploy-testnet.sh`. It maps friendly names to `C...` contract IDs:

| Key | Contract | Supported on testnet? |
|---|---|---|
| `contracts.ephemeralAccount` | `ephemeral_account` | Yes |
| `contracts.sweepController` | `sweep_controller` | Yes |
| `contracts.reserveContract` | `reserve_contract` | No (not deployed/wired) |
| `contracts.accountFactory` | `account_factory` | No (not deployed) |

```bash
# Read the current IDs from the deployment record
cat deployments/testnet.json
```

Only `ephemeralAccount` and `sweepController` are part of the supported
deployment, so those are the IDs worth tracing. The other two keys may hold IDs
that were never deployed by the current tooling. See the
[Testnet Overview](./overview.md#per-contract-status) for per-contract status.

## Step 2 - Blame / log the deployment record to find the commit

The deployment record is versioned in git, so the commit that last touched a
contract ID is your best clue about when (and therefore from which source) it
was deployed.

```bash
# Who / which commit last set each ID?
git blame -L '/"contracts"/,+8' deployments/testnet.json

# Full change history of the record
git log --follow --oneline -- deployments/testnet.json

# Show the exact commit that changed a specific ID line
git log -p --follow -- deployments/testnet.json
```

Take the commit hash from `git blame` for the ID you care about and inspect it:

```bash
git show --stat <COMMIT>
git show <COMMIT> -- deployments/testnet.json
```

That tells you when the ID was recorded. The source state that produced the
deployed binary is the tree at (or just before) that commit - but do not stop
here, because the ID can be updated in a commit separate from a code change.
That is what the WASM hash in Step 4 is for.

## Step 3 - Map the commit to a WASM hash

The same deployment record stores the WASM hash of the `ephemeral_account`
binary under `config.ephemeralAccountWasmHash`. In the committed record this is
a 32-byte SHA-256 value, for example:

```
5e667ea0687341bdccc81538492143b573777dd04b2450cdeb89b02cee62c58e
```

To correlate that hash back to source:

- Check whether the commit from Step 2 also changed
  `config.ephemeralAccountWasmHash`. If it did, the new hash belongs to that
  source change.
- Build `ephemeral_account` locally at that commit and compute the WASM hash of
  the artifact produced by `./scripts/build.sh` (or a direct
  `cargo build --target wasm32-unknown-unknown --release`), then compare it with
  the recorded value. A match confirms the recorded source produced the binary.

```bash
# Inspect the build output and compute its hash (tooling may vary by CLI version)
stellar contract info wasm --wasm target/wasm32-unknown-unknown/release/ephemeral_account.wasm --network testnet
# or hash the .wasm file directly with your local sha256 tool
```

If the hash cannot be reproduced from the recorded commit, treat the mapping as
unverified. The build may have used a different compiler version or local
changes; the chain is the only authority (Step 4).

## Step 4 - Verify the on-chain code hash

The authoritative check is the contract's code hash as reported by the Stellar
network, not anything in this repository. Query the live contract and compare
its code hash with `config.ephemeralAccountWasmHash`.

```bash
# Convenience command (available in recent stellar-cli releases)
stellar contract info wasm --id <CONTRACT_ID> --network testnet
```

If your CLI does not expose that subcommand, query the contract's ledger entry
through the Soroban RPC at `https://soroban-testnet.stellar.org`: call
`getLedgerEntries` for the contract data key that holds the instance and read
its `wasm_hash` field. That on-chain hash is what matters.

Compare the result with the value in `deployments/testnet.json`:

- **Match:** the recorded ID, WASM hash and source commit agree. You can trust
  the mapping from Step 2/3.
- **Mismatch:** the record is stale (a manual redeploy happened without a
  committed update, or the record points at a different deployment). In that
  case you cannot determine the live source from this repo - check with the
  team that performed the deployment.

You can also confirm an ID is live at all by invoking a read-only method, as
described in [Overview - Where contract IDs live](./overview.md#where-contract-ids-live-and-how-far-to-trust-them).

## Caveats

- **Testnet resets.** Stellar testnet is reset periodically; all contracts and
  their state are wiped. An ID that worked last week may resolve to nothing
  today. Always re-check on-chain.
- **Manual deploys only.** CI deploy steps are commented out, so there is no
  automatic record of *what* was deployed or *when*. Every deployment is a
  human running `scripts/deploy-testnet.sh`, and each run creates new IDs.
- **IDs change on every deploy.** Even a no-op redeploy produces different
  contract IDs. Do not pin IDs in source code; load them from config.
- **No commit is recorded on-chain.** The deployment record does not store a
  git SHA. The commit you get from `git blame` is a best-effort clue, and can be
  wrong if a deploy was done from an uncommitted tree.
- **Unreproducible IDs in `deployment-artifacts/contract-ids.txt`.** That file
  lists four IDs committed by hand (commit `741aec2`), but the current script
  only deploys two contracts. Its IDs cannot be reproduced from the tooling and
  must each be verified on-chain before use. It is not a reliable source for
  version correlation.
- **`reserve_contract` and `account_factory`.** These are not deployed or wired
  by the supported tooling, so there is no live source version to trace for
  them. See the [FAQ](./faq.md#is-reserve_contract-live-on-testnet).

## See also

- [Testnet Overview](./overview.md)
- [Testnet FAQ](./faq.md)
- [Network configuration](./network-config.md)
- [Main README](../../README.md) - MVP status and CI/CD notes
- [Signature Format](../../docs/SIGNATURE_FORMAT.md)
