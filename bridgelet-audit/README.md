<!--
Purpose: Index for the bridgelet-audit/ knowledge base.
Owner: @JudeDaniel6 (bridgelet-audit/ initiative contributor).
Status: Documentation-only. Introduces the folder and points at every file.
-->

# bridgelet-audit/

> **What this folder is.** Internal operations and security knowledge base
> for the Bridgelet Core Soroban contracts. Files here are deliberately
> separate from `docs/` (which is user-facing API reference) and from
> `contracts/`, `scripts/`, and `tools/` (which contain executable artifacts).
>
> **What this folder is NOT.** It is not part of the contract code base, the
> SDK, or any user-visible documentation. Nothing in this folder is shipped
> to downstream users; it exists to keep operational knowledge next to the
> code it pertains to.

## Table of Contents

1. [Where to Start](#where-to-start)
2. [Folder Layout](#folder-layout)
3. [Conventions](#conventions)
4. [File Index](#file-index)
5. [Adding New Entries](#adding-new-entries)

---

## Where to Start

| You are… | Read this first |
| :--- | :--- |
| **Deploying a `SweepController`** for the first time | [`checklists/sweep-controller-initialization-checklist.md`](checklists/sweep-controller-initialization-checklist.md) |
| **About to call `AccountFactory::batch_initialize`** in production | [`runbooks/validate-batch-initialize-salt-uniqueness.md`](runbooks/validate-batch-initialize-salt-uniqueness.md) |
| **Considering rotating** the locked sweep destination on a live controller | [`runbooks/verify-claim-vs-execute-sweep-nonce-state.md`](runbooks/verify-claim-vs-execute-sweep-nonce-state.md) |
| **Reviewing code** for audit-readiness across the contracts in `contracts/` | This README, plus the existing `docs/security.md`. |

---

## Folder Layout

```text
bridgelet-audit/
├── README.md                                          ← this file
├── checklists/                                        ← go-live review items
│   └── sweep-controller-initialization-checklist.md
└── runbooks/                                          ← operational procedures
    ├── validate-batch-initialize-salt-uniqueness.md
    └── verify-claim-vs-execute-sweep-nonce-state.md
```

Categories:

- **`checklists/`** — short, checkbox-style documents a reviewer follows at
  go-live. They answer *is this deployment ready?*.
- **`runbooks/`** — longer, procedure-style documents an operator follows
  before, during, or after an operation. They answer *how do I do this
  safely?*.

A new document belongs in `checklists/` if it is **binary** (each item is a
yes/no). It belongs in `runbooks/` if it has **sequenced steps** an operator
must follow in order.

---

## Conventions

Every file in this folder shares the same basic shape:

```markdown
<!--
Purpose: <one sentence>.
Owner: <github-handle or role>.
Status: Documentation-only. No <contracts|docs|scripts|tools> changes are
introduced by this file.
-->

# <Title>

| Field | Value |
| :--- | :--- |
| **Related issue** | [#NNN](https://github.com/bridgelet-org/bridgelet-core/issues/NNN) |
| **Owner / reviewer** | `_operator-name_` |
| **Last reviewed** | `_ISO-8601 date_` |
```

After the body, every file must end with a **Related Issues** section
listing every other file in this folder that the reader would benefit from
next.

Files use ATX headers, fenced code blocks, and tables consistent with the
existing `docs/` folder. They must remain Markdown-only — no embedded HTML,
no images, and no executable code blocks that the CI cannot statically
verify.

---

## File Index

| Document | Type | Closes | One-line description |
| :--- | :--- | :--- | :--- |
| [`checklists/sweep-controller-initialization-checklist.md`](checklists/sweep-controller-initialization-checklist.md) | Checklist | [#295](https://github.com/bridgelet-org/bridgelet-core/issues/295) | Confirm destination mode, signer custody, and authorized-controller binding before `SweepController` go-live. |
| [`runbooks/validate-batch-initialize-salt-uniqueness.md`](runbooks/validate-batch-initialize-salt-uniqueness.md) | Runbook | [#290](https://github.com/bridgelet-org/bridgelet-core/issues/290) | Reproduce `AccountFactory::batch_initialize`'s deterministic salt addresses and check the ledger for collisions before submitting a batch. |
| [`runbooks/verify-claim-vs-execute-sweep-nonce-state.md`](runbooks/verify-claim-vs-execute-sweep-nonce-state.md) | Runbook | [#288](https://github.com/bridgelet-org/bridgelet-core/issues/288) | Decide whether `update_authorized_destination`'s lock is genuinely in force by reading `get_nonce()` and cross-checking `SweepCompleted` events. |

---

## Adding New Entries

1. Pick the right sub-folder (`checklists/` or `runbooks/`) using the
   binary-vs-sequenced criterion above.
2. Use the conventions header template verbatim. Replace `_operator-name_`
   and `_ISO-8601 date_` with concrete values before opening a PR.
3. End the file with a *Related Issues* section that lists every other
   file in this folder cross-referenced from inside the body.
4. Add a row to [File Index](#file-index) above, with a one-line description
   that does not duplicate the title.
5. Open a PR linking the issue number in the form `Closes #NNN`.

CI runs `cargo fmt -- --check`, `cargo clippy -- -D warnings`, and
`cargo test --verbose` on `contracts/ephemeral_account`, `contracts/sweep_controller`,
and `contracts/reserve_contract`. Files in `bridgelet-audit/` are **not
validated by CI today** — reviewers must catch typos, broken anchors, and
inaccurate contract references manually. Treat any PR that touches this
folder as documentation-quality scrutiny, not a build-time check.
