# Testnet Data Retention and PII Policy

## Purpose

This document states one rule: **do not associate real people, real customer
records, or real credentials with anything on Bridgelet testnet.** It covers
contract metadata, event payloads, contract arguments, off-chain records, the
documentation tree, and issues.

It exists because of a structural property of the environment, not because
anyone suspects bad faith. Stellar testnet is a **shared, public,
read-without-authentication** network. Anything written to it is visible to
everybody, immediately and permanently, and it is copied onwards by systems
this project does not control.

## The rule, and why "delete it later" is not available

The reason for this policy is one specific fact, and it is worth stating before
any list of prohibited items:

> **Once a value is written to testnet — or to this repository, or to a public
> issue — treat it as public, permanent, and outside the project's control.**

Concretely:

| Stage | Who can see it | Can it be recalled |
|---|---|---|
| Submitted transaction | Every RPC node the network serves | No |
| Ledger history | Every archival node and RPC provider | No |
| Explorer indexing | Every public block/testnet explorer | No |
| Third-party mirrors, caches, analytics, indexer archives | The operator of that service | No |
| This repository, its issues, its PRs, its forks | Anyone | Only by history rewrite, which does not un-disclose it |

Testnet XLM and Bridgelet test tokens have no value. That is the **only**
thing that is not real about the environment. The *behaviour* is real: the same
Rust code, the same `soroban-sdk` 22.0.0 serialization, the same event encoding,
the same Ed25519 verification, the same error codes. And a value that was
harmless when written can stop being harmless — a "throwaway" address becomes an
employee's address on a payroll, a customer's address in a support ticket, a
seed that gets funded or reused.

So the answer to "it's fine, we'll clean it up afterwards" is not "we will try".
It is: **by the time you would want to delete it, the copy is no longer yours
to delete.** The only remaining control is not writing it.

## Status and verification

| Statement | Basis |
|---|---|
| `EphemeralAccount::initialize` stores `creator`, `expiry_ledger`, `recovery_address`, `authorized_controller`, `admin` in instance storage | Read from `contracts/ephemeral_account/src/lib.rs` |
| `EphemeralAccount` events carry `Address` and `i128` values in their payloads | Read from `contracts/ephemeral_account/src/lib.rs`; topic/payload shapes also in `testnet/registry/event-topics.md` |
| `EphemeralAccount::expire` and `::recover` move funds to `recovery_address` | Read from source |
| No Bridgelet contract is currently deployed | `testnet/registry/contract-status.md` records all four as undeployed, IDs `TBD`, Last Verified `Never` |
| The main README status banner reads "Active Development - MVP. Not audited." | Read from `README.md` line 5 |

Nothing in this document is an on-chain observation. Because the deployment
status is unverified, the metadata claims above are **what the source
specifies**, and they are the reason the policy is written now rather than
later.

## Scope

**In scope:** anything a contributor or integrator writes to Stellar testnet
through Bridgelet contracts, anything recorded in this repository, and anything
posted in an issue, PR, or support channel.

**Explicitly out of scope:**

- Testnet XLM and Bridgelet test tokens. Not sensitive; this policy is not
  about them.
- On-chain *contract code*. The WASM is public by design.
- A hypothetical future mainnet deployment. This policy deliberately says
  nothing about how a production deployment would treat the same data.
- Test harness fixtures that are synthetic in the sense defined below. These
  are **permitted**, and are the intended substitute.

## What must never be written

### Synthetic is the operative word

A value is synthetic if **all three** of these hold:

1. it was generated for the purpose of testing;
2. it maps to no real person, organisation, customer, employee, or system; and
3. it is **not** a transformation of real data.

Point 3 is the one people get wrong. A real customer name with the vowels
changed is still a real customer name. So are:

- a real address with one character altered;
- a real email at a real domain with a different local part;
- a real phone number with the last digit changed;
- a real government ID number with a digit inserted;
- a real card number with a substituted digit, even an Luhn-valid one;
- a real person's real name reused as a "test" label.

Redaction that is reversible by inference is not redaction. If a colleague could
work out whose value it is, it is not synthetic.

### Never written, in any form, anywhere

| Category | Examples of what must never appear |
|---|---|
| Direct identity | Real names, usernames tied to a person, real email addresses, real phone numbers, real postal addresses, real handles |
| Government / national identity | National ID, tax ID, passport, driving-licence, voter, national insurance, or health-service numbers |
| Financial credentials | Payment-card numbers, CVV, bank account and sort/I BAN numbers, payment credentials of any kind |
| Key material | Wallet seed phrases, mnemonic phrases, private keys, `S...` Stellar secret keys, Ed25519 signing seeds, passphrase-protected key files — of any chain, on any network |
| Customer / employee records | Names paired with balances, transaction history, account identifiers, employment data, support-ticket contents |
| Medical and financial personal data | Diagnoses, prescriptions, health records, salary, tax returns, credit history, investment holdings |
| Credentials | Production API keys, access tokens, bearer tokens, OAuth refresh tokens, CI secrets, database URLs with embedded passwords, `.env` files containing any of the above |
| Live-system identifiers | Real production contract IDs on a production network, real customer account IDs, real internal service endpoints |

A production API key is the category people underestimate. "It's a
read-only key", "it's for a sandbox", "it expires" — none of that matters. The
key is a string, and strings get copied.

### Off-chain records count too

The same prohibition applies to anything an operator writes down: a shell
transcript, a CI log, a local `.env`, a test fixture, a screenshot pasted into
an issue, a support-ticket paste. `testnet/docs/support.md` already tells
reporters never to post secret keys. This policy is broader: the ban on real
personal data covers ticket text and log files too, not only key material.

## The metadata angle: what actually lands on chain

The issue this policy closes names "accounts/metadata". Concretely, for the
current source, these values persist in contract storage and/or event payloads:

| Value | Where it ends up |
|---|---|
| `creator` | Stored at `initialize()`; emitted in the `created` event as `AccountCreated { creator, expiry_ledger }` |
| `expiry_ledger` | Stored; emitted in the same `created` event |
| `recovery_address` | Stored; emitted in the `expired` event as `AccountExpired { recovery_address, total_amount, reserve_amount }` |
| `authorized_controller` | Stored; not emitted |
| `admin` | Stored; not emitted |
| Asset `Address` and `amount: i128` per payment | Stored; emitted in `payment` / `multi_pay`, and nested in `swept_mul` |
| `destination` | Stored; emitted in `swept_mul` and in the controller's `sweep` event |
| `swept_to` | Stored; returned by `get_info()` |
| `sweep_id` and `remaining_reserve` | Emitted in `reserve` |

`testnet/registry/event-topics.md` is the reference for the exact topic symbols
and payload struct shapes. The mechanism that matters here is simple: **an
`Address` in an event payload is world-readable.** There is no ACL on an event,
no per-subscriber encryption, and no way to emit an event that only one party
can read. Anyone who can run a `getEvents` query against the public testnet RPC
can enumerate `created` events and recover the full set of `creator` addresses
associated with accounts, then the `recovery_address` set from `expired`
events, then destination addresses and amounts.

Two consequences worth being explicit about:

1. **An address is a pseudonymous identifier, and pseudonymous is not
   anonymous.** If the same address appears in a public issue, a public
   on-chain explorer, or a public repository, it is a public link between
   "this person" and "this account" regardless of any policy here.
2. **A `recovery_address` set to a real personal wallet is a real risk**, not a
   formality. Funds from every account that expires unswept are recoverable to
   that address by anyone who can call `expire()` — and `expire()` is
   deliberately permissionless so that funds cannot be stranded. Publishing a
   colleague's personal wallet as the recovery address for a shared testnet
   account therefore advertises a destination that other people can move
   funds to.

### Fields this repository does not have

For accuracy, two things often named in metadata-retention discussions do not
exist in this codebase and must not be written about as if they did:

- There is **no `cargo_description` field** anywhere in the contracts. A
  repository-wide search for that identifier returns nothing.
- There is **no free-text `notes` or label field** in any contract type.
  `bridgelet_shared::AccountInitRequest` has exactly two fields,
  `expiry_ledger: u32` and `recovery_address: Address`.

The reason this is stated rather than assumed: a contributor reading a
sibling document could believe a `cargo_description` argument exists and
"document" it. If such a field is ever added, this policy applies to it
immediately, and the addition needs a changelog entry.

## Why testnet is different, in specifics

Not adjectives — mechanisms:

1. **The deployment is public and shared.** Every unrelated integrator, and
   every passer-by, can read everything. `testnet/security/README.md` sets out
   the multi-tenant consequence: an action by one party degrades the
   experience of every other party, silently in the case of `upgrade()`.
2. **No audit, no SLA, no commitment.** The main `README.md` banner reads
   **"Active Development - MVP. Not audited."** There is no uptime promise, no
   support commitment, and no change-management process behind this network
   beyond the ones documented in `testnet/`.
3. **There is no confidentiality benefit to writing real data here anyway.**
   Anyone can build the same contracts from the same public source and run
   them locally. A real secret put on testnet is exposed *and* buys nothing,
   because the functionality it would protect is fully reproducible offline.
4. **This repository and its history are public too.** A real value pasted into
   an issue body, a PR description, a code block, a test fixture, or a doc is
   exposed on exactly the same timescale as one written to a ledger. Fork PRs
   are not private.
5. **Testnet resets and redeployments happen.** `testnet/docs/changelog.md`
   records the reset/redeploy history, and `testnet/docs/network-config.md`
   warns that an SDF reset wipes every account and contract ID. A reset is not
   a privacy control: it destroys the live reference, not the archival copies
   held by third parties.
6. **A hypothetical production deployment would be held to a higher standard
   that does not exist here.** The distinction is not that testnet data is
   unimportant; it is that the operational-security guarantees, monitoring,
   and incident process that would make a production deployment a reasonable
   place to handle real data are not present, and are not committed to.

## Relationship to the existing no-secrets rules

This policy does not restate the tree's existing discipline; it cites and
extends it.

| Document | What it already requires |
|---|---|
| [`testnet/config/README.md`](../config/README.md) | "No secrets, ever" for the config tree; `.env.example` at the repo root is the only place deployment-time credentials live |
| [`testnet/security/README.md`](../security/README.md) | Observations are **public information only** — contract IDs, addresses, transaction hashes, ledger numbers, event data; never a secret key, seed, or someone else's signature |
| [`testnet/security/admin-key-hygiene.md`](../security/admin-key-hygiene.md) | Dedicated testnet keys, never in git in any form, rotate-before-anything-else if exposed |

Those documents permit public on-chain identifiers, correctly: an address or a
contract ID is public information and is exactly what an observation log
should contain.

**This policy is stricter, in one specific way: it says do not associate real
people with those identifiers in the first place.** A public `G...` address is
acceptable *as an address*. The same address annotated as "Alice's testnet
wallet" in a doc, an issue, or a commit message is a disclosure, and the
annotation is the part that is not permitted.

## The wallet-key rule

Stated separately because it is the rule most likely to be broken by someone
with good intentions and a busy afternoon.

> **Never use a mainnet secret key, or any key that holds or may hold real
> funds, against testnet. Ever. Not once, not "just to check", not "it's the
> same person".**

- `SIGNER_SECRET_KEY` in `.env.example` is a **testnet-only** deployer key. It
  pays fees and signs `initialize` calls. Generate it for this purpose.
- `tools/sweep-signer` wants a **signing-only** Ed25519 seed that is never a
  funded Stellar account. It is configured via `--signer-seed-hex` /
  `--signer-seed-hex`'s `SWEEP_SIGNING_KEY_SEED` env var, or
  `--signer-secret` / `AUTHORIZED_SIGNER_SECRET`. Both are secrets. Derive the
  public half with the `pubkey` subcommand and never put the private half
  anywhere shared.
- Never use a key from a personal wallet, a hardware wallet, an exchange, a
  local sandbox you also use for value-bearing work, or another project.
- Do not "just try" a key on testnet to see whether it works. A read-only or
  zero-value test is still a disclosure, and a wrong-signed mainnet
  transaction is a real transaction.

The `.env.example` file contains a commented example seed used in a
`--signer-seed-hex` invocation. It is a documentation artefact. **Do not treat
it as a real key, do not use it, and do not copy it into anything.**

## Pre-flight checklist

Run this before **any** submission against testnet — a deploy, an
`initialize`, a `record_payment`, a sweep, or a scripted batch.

### Substitute

- [ ] Every `Address` argument is a **freshly generated testnet identity** or a
      documented synthetic fixture address. Not a colleague's, yours, or a
      customer's.
- [ ] `recovery_address` is a synthetic destination you control and have funded
      from Friendbot — **not** a personal wallet, even yours. It is the
      "organization's fallback wallet" per `.env.example`, and it is where
      funds land if an account expires unswept. Use a testnet identity, never a
      mainnet-shaped one.
- [ ] `creator`, `admin`, and `authorized_controller` are synthetic testnet
      identities. Note that `admin` and `authorized_controller` are **fixed at
      `initialize()` with no setter and no rotation path** — see
      `testnet/security/admin-key-hygiene.md`.
- [ ] `destination` for a sweep is synthetic.
- [ ] `AUTHORIZED_SIGNER_PUBLIC_KEY` and its seed come from a testnet-only
      keypair generated for this purpose.
- [ ] `SIGNER_SECRET_KEY` is the testnet-only deployer key.

### Verify

- [ ] `stellar keys address <identity>` on every identity above returns what
      you expect, and you know which ones are testnet-only.
- [ ] No address in your working set has ever been seen on mainnet. If you are
      unsure, generate a new one; it costs a Friendbot request.
- [ ] Your batch inputs (`AccountInitRequest` vectors) contain only
      `expiry_ledger` and `recovery_address` — both synthetic. There is no
      free-text field, so nothing should have crept in.
- [ ] Your working tree has no uncommitted `.env`, and the key material you are
      using is coming from a secret store or an ephemeral shell export, not a
      file.

### Redaction-check before submitting

- [ ] The exact invocation, including the argument values, has been read
      through once. Arguments are the thing most often copied from a real
      session.
- [ ] Any free text you are about to put in a script, a comment, a log line,
      or an issue is free of names, emails, ticket numbers, and customer
      references.
- [ ] The `expiry_ledger` and amounts are plausible test values and not a copy
      of anything from a real system.
- [ ] Nothing you are about to paste into a GitHub issue, PR, or support
      channel contains key material. `testnet/docs/support.md` repeats this
      rule for reporters.

## If real data has already been written

Be straightforward about the limit: **on-chain data generally cannot be
deleted.** A testnet reset destroys the live reference and does nothing about
archival nodes, explorer indexes, or mirrors. So remediation is about what is
still controllable, not about making the disclosure disappear.

Do these, in this order:

1. **Stop the practice.** Find the script, fixture, template, or habit that
   produced it and fix it before doing anything else. A repeat of the same
   value is a second disclosure.
2. **Rotate anything that was a key or credential.** Immediately, and before
   anything cosmetic. Removing a secret from git history does not
   un-disclose it. `testnet/security/admin-key-hygiene.md` gives the ordering.
3. **Establish the blast radius** from public data, not from memory: the
   contract ID, the ledger range, the transaction hashes, and the emitted
   events. Those are all obtainable and all permitted to record.
4. **Report it.** Use the private route — the repository's **Report a
   vulnerability** control, or the contact in
   [`testnet/docs/contact-and-ownership.md`](../docs/contact-and-ownership.md).
   `testnet/security/README.md` is the index for that directory; there is no
   separate `testnet/security/disclosure-process.md` today, and inventing a
   route that does not exist would waste the moment that matters. Do **not**
   open a public issue, per `testnet/docs/support.md`.
5. **Tell affected people.** If a real person's data was exposed, they should
   hear it from the project rather than find it in an explorer. This is a
   courtesy obligation, not a technical step, and it is the one most often
   skipped.
6. **Record the mechanism** in
   [`testnet/runbooks/incident-postmortem-template.md`](../runbooks/incident-postmortem-template.md)
   so the next person does not repeat it. An unlogged mistake is a repeat
   waiting to happen.

## Summary table

| Data category | Permitted on testnet? | Use instead |
|---|---|---|
| Real person name, email, phone, address | **No** | A generated testnet identity; synthetic labels like `alice-testnet` in private notes only |
| Government / national ID number | **No** | A generated placeholder that maps to nothing |
| Real postal address | **No** | A synthetic address in a reserved/test block, or no address at all |
| Card or bank details | **No** | Nothing. There is no testnet flow that needs them |
| Wallet seed phrase / private key / `S...` secret | **No** | A testnet-only keypair generated for this purpose |
| Ed25519 signing seed | **No** | A signing-only seed generated for this purpose, never a funded account |
| Production API key or token | **No** | A testnet-scoped key from the provider's own test tier, if one exists |
| Real customer or employee record | **No** | A synthetic record generated for the test |
| Real medical or financial personal data | **No** | A clearly synthetic value with no real-world referent |
| Public contract ID (`C...`) | Yes, once verified | Record it with its verification date and ledger |
| Public testnet address (`G...`) as an address | Yes | An identity generated for testnet use only |
| Public address annotated with a real person's identity | **No** | Leave the annotation out; keep the mapping in a private note |
| Transaction hash, ledger number, event payload | Yes | These are exactly what `testnet/security/` says to record |
| Synthetic test account, expiry, and amount values | **Yes — this is the point** | Generate them |
| Testnet XLM and test tokens | Yes | Friendbot; see `testnet/faucet/README.md` |
| The contract source and WASM | Yes, by design | Nothing to change |

## What would count as fixing this

This policy is documentation and process. Concretely, it cannot currently be
enforced, and saying so is more useful than implying a guarantee:

- **No Bridgelet contract is deployed** (`testnet/registry/contract-status.md`),
  so there is nothing in which to enforce a constraint on arguments.
- Even once deployed, `EphemeralAccount::initialize` validates expiry, rejects
  double initialization, and requires `creator` auth — and nothing else. It
  cannot tell a synthetic address from a real one. A hypothetical production
  deployment could plausibly add an allowlist of approved recovery
  destinations or refuse to accept a mainnet-shaped address; neither is
  designed, built, or committed to.
- The controls that exist today are: this checklist, code review, the
  pre-existing no-secrets rules in `testnet/config/` and `testnet/security/`,
  and the willingness of the person submitting to run the redaction pass.

Legitimate follow-ups, in rough order of value:

1. **Get one honest incident report through this path.** The path is untested;
   running it once on a synthetic "we did this" case would find the gaps.
2. **Publish a short copy-pasteable redaction checklist** in
   `testnet/docs/getting-started.md`, so it is read by people who never open
   `testnet/policies/`.
3. **Add a pre-commit secret/identifier scan** to the existing CI workflows,
   scoped to `testnet/`. This catches pasted keys; it does not catch a real
   name.
4. **Decide whether a production deployment gets a destination allowlist**,
   and record that decision wherever production governance ends up living.
5. Do not build a mechanism that gives the appearance of enforcement without
   the enforcement.

## Related Documentation

- [`testnet/config/README.md`](../config/README.md) — the no-secrets discipline
  this extends
- [`testnet/security/README.md`](../security/README.md) — public-information-only
  rule for observations; the shared-deployment argument
- [`testnet/security/admin-key-hygiene.md`](../security/admin-key-hygiene.md) —
  key handling and the exposure-response ordering
- [`testnet/security/known-testnet-abuse-patterns.md`](../security/known-testnet-abuse-patterns.md) —
  what an unauthenticated party can already do to the shared deployment
- [`testnet/registry/event-topics.md`](../registry/event-topics.md) — exactly
  what lands in event data, and why it is world-readable
- [`testnet/registry/contract-status.md`](../registry/contract-status.md) —
  current deployment status (all four contracts undeployed)
- [`testnet/registry/known-test-accounts.md`](../registry/known-test-accounts.md) —
  where long-lived demo accounts belong
- [`testnet/docs/support.md`](../docs/support.md) — the reporting routes, and
  the rule against posting key material
- [`testnet/docs/contact-and-ownership.md`](../docs/contact-and-ownership.md) —
  escalation contacts
- [`testnet/docs/changelog.md`](../docs/changelog.md) — the reset and
  redeployment record
- [`testnet/runbooks/incident-postmortem-template.md`](../runbooks/incident-postmortem-template.md) —
  where to record what happened
- [`testnet/policies/contribution-guide.md`](contribution-guide.md) — how to
  propose changes to this directory
- `contracts/ephemeral_account/src/lib.rs` — the source for every behavioural
  claim above
- `tools/sweep-signer/` — the signing-only CLI; documented, never modified

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-26 | Initial testnet data retention and PII policy |
