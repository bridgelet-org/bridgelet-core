# Runbook: Suspected Reentrancy on Testnet

## Purpose

Practical, **testnet-side** checklist when a live transaction *looks* like it
might have re-entered Bridgelet contracts (`EphemeralAccount` sweep /
`record_payment` paths, factory, controllers).

Design-level Soroban guarantees and unit-level arguments live in
[`docs/reentrancy-analysis.md`](../../docs/reentrancy-analysis.md) (**do not
duplicate that analysis here**). This runbook only covers **what evidence to
gather on-chain** to confirm or rule out reentrancy in an incident.

## When to use

- Unexpected double spend, double sweep, or payment recorded twice in one tx
- Event sequences that appear “out of order” relative to status transitions
- A counterparty contract or token is callback-capable and was invoked mid-flow

## Non-goals

- Re-deriving the formal reentrancy analysis
- Changing contract code during the investigation
- Treating “called the function twice in two transactions” as reentrancy

## 1. Freeze the evidence set

Collect before any remediation txs:

| Item | Why |
|---|---|
| Transaction hash(es) | Single vs multi-tx is the first split |
| Contract ids + WASM hashes | Pin the deployed build |
| Ledger sequence / timestamp | Ordering anchor |
| Source account / auth entries | Who could have invoked what |
| Horizon + Soroban event payloads | Raw topics and bodies |

Do not upgrade or re-initialize the suspect contract until hashes are recorded
(see `testnet/registry/verify-contract-live.md` when available).

## 2. Establish the invocation boundary

Reentrancy (for this checklist) means: **within one committed transaction**,
contract A is entered, invokes external work, and is entered again **before**
the first activation finishes, producing an extra committed effect.

Gather:

1. Full operation list for the transaction.  
2. Cross-contract call tree if available from diagnostics/tracing tools.  
3. Whether more than one Bridgelet entrypoint appears in that tree.

If the surprising effects appear in **separate** transactions, treat as
replay, race, or duplicate submission—not reentrancy—unless a bridge between
txs is proven.

## 3. Event ordering checklist

Using [`../registry/event-topics.md`](../registry/event-topics.md) as a name
guide, list events **in host emission order** for the suspect transaction:

1. Payment / multi-payment events (`record_payment` path)  
2. Status / lifecycle events (e.g. active → swept)  
3. Sweep / transfer-out events  
4. Reserve / reclaim events  

Questions:

- Does a second `payment` / `multi_pay` for the **same asset** appear **after**
  a successful record without an intervening successful error path?
- Does a second sweep-related transfer appear **after** status already
  indicates swept, **inside the same tx**?
- Are status transitions monotonic, or do they oscillate within the tx?

## 4. Status transition checklist

For the `EphemeralAccount` (or factory child) involved:

1. Read current status and any “swept to” / payment metadata.  
2. From events + tx meta, reconstruct status at each event boundary.  
3. Flag any pattern where external token calls sit **between** a check and a
   write that the analysis document claims are ordered check→write→external.

On-chain confirmation of the documented order (status updated before external
transfers) **weighs against** classical reentrancy. A committed external
transfer **before** the corresponding status write in the same tree is a
**signal to escalate**, not yet proof.

## 5. Decision table (field use)

| Observation | Prefer classification |
|---|---|
| Two txs, second fails `AlreadySwept` / `DuplicateAsset` | Replay or duplicate submit |
| One tx, single entry, expected events once | Normal success |
| One tx, external callback into same contract mid-frame + extra effect | **Suspected reentrancy** — escalate |
| Same asset paid twice across two txs | Integrator double-submit (see [multi-payment-edge-cases.md](multi-payment-edge-cases.md)) |
| Missing events due to RPC gaps | **Inconclusive** — follow [rpc-outage.md](rpc-outage.md), re-query |

## 6. Controlled reproduction (optional)

Only after preserving live evidence:

- Throwaway account and assets only  
- No shared production testnet wallets  
- Pre-declare expected call tree and max loss bound  
- Sequential double-invoke is **not** a reentrancy repro  

If safe reproduction is impossible, record **inconclusive**.

## 7. Close out

Pick one: **supported reentrancy** / **not reentrancy** / **inconclusive**.
File [`incident-postmortem-template.md`](incident-postmortem-template.md).
Report suspected vulnerabilities through the project’s private security
channel (see `docs/security.md` if present).

## Related documentation

- [`../../docs/reentrancy-analysis.md`](../../docs/reentrancy-analysis.md) — design analysis (reference only)
- [`multi-payment-edge-cases.md`](multi-payment-edge-cases.md) — duplicate payment behavior
- [`rpc-outage.md`](rpc-outage.md) — incomplete event fetches
- [`../registry/event-topics.md`](../registry/event-topics.md) — event names
