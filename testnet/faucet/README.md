# Testnet Faucet & Account Funding Guide

Funding testnet accounts with Friendbot is documented here, in
`testnet/faucet/`. This directory is the canonical location for Bridgelet's
funding guidance, as listed in
[`../policies/contribution-guide.md`](../policies/contribution-guide.md).

> **Documentation drift.** Other documents reference `testnet/config/friendbot.md`
> as the funding reference—including the harness notes, the faucet-helper spec,
> and the tooling decision log. That file is **not present** in this clone.
> Those links are stale; use this directory instead. A referenced path is not
> evidence that a document exists.

Friendbot is a public service operated by the Stellar Development Foundation,
**not** by Bridgelet. Availability, limits, and response times are outside
Bridgelet's control; testnet support here is best effort.

---

## In this directory

| Document | Use it when |
|---|---|
| [`troubleshooting.md`](troubleshooting.md) | A Friendbot request fails, times out, is throttled, or the account does not appear |
| [`alternative-funding.md`](alternative-funding.md) | Friendbot is down and you need a safe alternative path |
| [`bulk-funding-notes.md`](bulk-funding-notes.md) | You need to fund many accounts and want a reusable, rate-limit-friendly pattern |

---

## Basic Friendbot funding

The documented endpoint is:

```text
https://friendbot.stellar.org/?addr=<YOUR_PUBLIC_G_ADDRESS>
```

Fund a generated testnet keypair with test XLM:

```bash
curl "https://friendbot.stellar.org/?addr=<YOUR_PUBLIC_G_ADDRESS>"
```

Use only a public `G...` address here. The older stub described a fixed
`10,000 test XLM` payout; treat that as an expectation rather than a
guarantee, and verify the actual balance on Horizon. The installed Stellar
CLI's `stellar keys fund <identity> --network testnet` is another client of
the same public service, not a separate faucet.

Always verify the result on Horizon:

```bash
curl --fail-with-body --silent --show-error \
  "https://horizon-testnet.stellar.org/accounts/<YOUR_PUBLIC_G_ADDRESS>"
```

A successful faucet response is not proof the balance changed; a timeout is
not proof the request failed. See
[`troubleshooting.md`](troubleshooting.md) before retrying.

---

## What Friendbot does and does not do

| Friendbot does | Friendbot does not |
|---|---|
| Fund a public `G...` account with test XLM on Stellar Testnet | Fund a `C...` contract ID or any contract address |
| Create and fund a new G... account on-ledger so it can pay transaction fees. | Mint SEP-41 test tokens, or move token balances |
| Provide a generic, public faucet endpoint | Deploy or `initialize` a Bridgelet contract, or call `record_payment` |
| Return a response for a request | Guarantee availability, a response time, or a fixed payout |
| Operate as an SDF service | Operate as a Bridgelet project service with an SLA |

`record_payment()` is bookkeeping only—it does not move value. A sweep needs a
real token balance, which is separate from funding the fee payer. See
[`../examples/integration-test-harness-notes.md`](../examples/integration-test-harness-notes.md).

---

## Relationship to account and contract funding

1. **Fund the fee payer.** A `G...` identity must exist on-ledger and hold
   native XLM before it can submit transactions. Check it on Horizon first;
   top up only when needed, as in
   [`../monitoring/balance-thresholds.md`](../monitoring/balance-thresholds.md).
2. **Contract calls need a funded source.** Deployment, `initialize`, and
   `record_payment` all require a funded signer. An unfunded identity fails
   later with a confusing auth or sequence error.
3. **Contract IDs are not funding targets.** The canonical
   [`../registry/contract-status.md`](../registry/contract-status.md) records
   all four Bridgelet contracts as **undeployed**: two with `TBD` IDs
   (EphemeralAccount, SweepController) and two with `N/A` IDs (ReserveContract,
   AccountFactory), despite conflicting older artifacts. Do not copy an ID
   from those artifacts or claim a contract is live.
4. **No Bridgelet transfer source is registered.**
   [`../registry/known-test-accounts.md`](../registry/known-test-accounts.md)
   registers long-lived `EphemeralAccount` contract instances, not funded
   `G...` accounts; all rows are `TBD`, so do not rely on it for transfers.
5. **Keep secrets out.** Never place an `S...` secret key or seed in a faucet
   URL, a log, a chat message, or version control. Only public keys travel in
   a funding request.

---

## Rate limits

No project-specific measured Friendbot limits are recorded here. Existing
documents disagree about generic behavior ("per IP" in the earlier stub,
"per address" in the harness notes); neither is a Bridgelet measurement. Do
not treat either as a fixed number, and do not hard-code a wait as a
synchronization primitive.

The safe pattern is to verify on Horizon, then retry with bounded exponential
backoff and jitter, funding accounts serially. A throttle means slow down;
it is not an outage of Bridgelet infrastructure and not something to evade
with proxies or VPNs. See
[`troubleshooting.md`](troubleshooting.md) and
[`bulk-funding-notes.md`](bulk-funding-notes.md) for the details.

---

## Related documents

- [`../examples/integration-test-harness-notes.md`](../examples/integration-test-harness-notes.md) — live-testnet suite latency and account pooling
- [`../config/soroban-cli-identity-setup.md`](../config/soroban-cli-identity-setup.md) — testnet identities and funding commands
- [`../docs/network-config.md`](../docs/network-config.md) — endpoints and passphrase
- [`../runbooks/rpc-outage.md`](../runbooks/rpc-outage.md) — broader RPC/Horizon outage guidance
- [`../docs/support.md`](../docs/support.md) — reporting testnet issues
- [`../policies/contribution-guide.md`](../policies/contribution-guide.md) — subdirectory scope map

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-11-18 | Consolidated funding guidance into a hub; documented config/friendbot.md drift |
