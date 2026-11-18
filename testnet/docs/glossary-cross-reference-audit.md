# Glossary Cross-Reference Audit

Housekeeping record: confirm that the terms in `testnet/docs/glossary.md` are
cross-linked from the runbooks and examples that use them, so the glossary does
not become an orphaned page.

## Finding: the glossary does not exist yet

`testnet/docs/glossary.md` is **not in the repository** at the time of this
audit, and no document links to a glossary (`grep -ril glossary testnet docs
README.md` returns nothing). The cross-link check the issue describes therefore
cannot be performed. This document records that, the candidate terms and their
current usage, and the procedure to run once a glossary exists.

## Candidate terms and current usage

Number of `testnet/` files (case-insensitive substring match) that mention each
term, as a measure of where links would matter. Counted with `grep -rli`.

| Term | Files mentioning it |
|---|---|
| sweep | 70 |
| reserve | 50 |
| nonce | 49 |
| claim | 49 |
| WASM hash | 33 |
| authorized_controller | 27 |
| relayer | 27 |
| passphrase | 27 |
| ephemeral account | 23 |
| SEP-41 | 21 |
| Soroban RPC | 20 |
| Horizon | 16 |
| stroop | 16 |
| Friendbot | 14 |
| locked mode | 11 |
| expiry ledger | 9 |
| recovery address | 9 |
| auth entry | 6 |
| authorized signer | 3 |

(The counts reflect the tree as of this PR, before any other pending
documentation PRs merge. Re-run after merging.)

## Recommended minimum glossary (when it is written)

Terms most likely to confuse a newcomer: ephemeral account, sweep, claim (gas-free),
relayer, nonce, authorized controller, authorized signer, locked mode, expiry
ledger, recovery address, reserve (network vs contract-internal), stroop,
SEP-41, auth entry, WASM hash, passphrase, Friendbot, Soroban RPC vs Horizon.

## Procedure to run once `glossary.md` exists

1. List glossary terms:
   ```bash
   grep -E '^(##|###|\*\*)' testnet/docs/glossary.md
   ```
2. For each term, find documents that use it and check whether they link to the
   glossary anchor on the **first** meaningful use:
   ```bash
   term="relayer"
   grep -rli -- "$term" testnet | xargs grep -L "glossary.md" 
   ```
   The second command lists files that use the term but never link the glossary.
3. Prioritize runbooks and examples (`testnet/runbooks/`, `testnet/examples/`,
   `testnet/config/`) over registry tables.
4. Add the link (`[relayer](../docs/glossary.md#relayer)`) at the first use in each
   document; do not link every occurrence.
5. Check the other direction: every glossary entry should point to the most useful
   detail document.
6. Confirm the glossary itself is linked from `testnet/README.md` (if present),
   `testnet/docs/getting-started.md` and `testnet/docs/faq.md`.
7. Record results in the table below.

## Audit results table (to fill in)

| Term | Glossary anchor exists | Docs using it | Docs linking glossary | Gaps fixed |
|---|---|---|---|---|
| _pending glossary_ | | | | |

## Follow-up

Open a follow-up issue to write `testnet/docs/glossary.md`, then run this audit.
`testnet/docs/common-misconceptions.md` is a natural place to link the glossary from.
