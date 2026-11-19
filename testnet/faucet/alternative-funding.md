# Alternative Funding Paths When Friendbot Is Down

## Status

This page lists funding paths an integrator can actually use when the public
Stellar Testnet faucet is unavailable. Friendbot is operated by the Stellar
Development Foundation, **not** by Bridgelet. Bridgelet testnet support is
best effort, and no path below is a guaranteed source of test XLM.

No project-specific faucet service, measured limit, or dedicated Bridgelet
faucet is recorded in this repository. Do not invent a service, endpoint, or
funded account during an incident.

---

## Decision flow

```text
Need test XLM for a G... address?
|
+-- Already funded on Horizon?
|   +-- yes -> reuse it; no funding request needed
|
+-- Is Friendbot responding normally?
|   +-- yes -> request once, verify on Horizon, then retry with backoff
|             only if needed (see troubleshooting.md)
|
+-- Are RPC/Horizon also failing, or is a testnet reset suspected?
|   +-- yes -> follow ../runbooks/rpc-outage.md; pause, preserve evidence,
|             and do not replay pre-reset work
|
+-- Have you already got a funded account you control?
|   +-- reuse your own pool from a CI secret store (see bulk-funding-notes.md)
|   +-- a Bridgelet transfer source? none is registered today:
|       ../registry/known-test-accounts.md lists long-lived contract
|       instances, not funded G... accounts, and all rows are TBD
|
+-- Ask in the #testnet-support community channel listed in
|   ../docs/contact-and-ownership.md (unverified; no response guaranteed),
|   or open a [testnet] issue as described in ../docs/support.md
|
+-- Otherwise wait, re-check the official Stellar status page, and retry
    later with a freshly generated G... key
```

---

## Path 1: retry with backoff while verifying each attempt

Friendbot may be the only failing component. Before treating it as down,
confirm the account's state on Horizon and follow
[`troubleshooting.md`](troubleshooting.md). A response is not proof that the
balance changed, and a timeout is not proof that the request failed.

Retry with bounded exponential backoff and jitter. Do not run a tight loop,
and do not switch networks or passphrases to work around a throttle.

## Path 2: community Discord / support channel

[`../docs/contact-and-ownership.md`](../docs/contact-and-ownership.md) lists a
**Community Discord / Support Channel** of `#testnet-support`. This listing is
**unverified** in this clone: it is a documented contact, not a confirmed
staffed faucet, and no response time or funding is promised. Use it to ask
whether others see the same failure, not as a funding service.

For a broader or reproducible failure, open a `[testnet]` issue per
[`../docs/support.md`](../docs/support.md). Include the public address, UTC
time, and exact response. **Never post a secret key (`S...`), seed phrase, or
signing material** in a channel or issue.

## Path 3: no registered Bridgelet transfer source

[`../registry/known-test-accounts.md`](../registry/known-test-accounts.md)
registers long-lived **`EphemeralAccount` contract instances** (`C...`) for
demos and documentation. It is not a registry of funded `G...` accounts, and
every row is currently `TBD` pending deployment. There is therefore **no
Bridgelet `G...` transfer source registered today**, so do not rely on this
path, infer an address from older artifacts, or fabricate one.

Even once those contract rows are populated, a `C...` instance is not a
classic account that can send or receive a top-up. A transfer path would need
a separate, confirmed funded `G...` account whose maintainers agree to send
funds; none is registered here. Send only to a recipient's public `G...`
address, never to a contract ID, and use a verified pool or an agreed transfer
from [`bulk-funding-notes.md`](bulk-funding-notes.md) instead.

## Path 4: reuse your own funded pool

If you already funded a small pool of accounts during an earlier run, reuse
it. [`bulk-funding-notes.md`](bulk-funding-notes.md) and
[`../examples/integration-test-harness-notes.md`](../examples/integration-test-harness-notes.md)
describe the pool pattern and why it removes Friendbot from the hot path.
Reuse the funded *Stellar* account; deploy a fresh `EphemeralAccount`
**contract instance** for each test that needs one.

## What is not an alternative

- **A made-up faucet, RPC endpoint, or "free XLM" service.** Never substitute
  an unverified provider during an incident. The fallback policy in
  [`../runbooks/rpc-outage.md`](../runbooks/rpc-outage.md) applies: no invented
  provider is approved by being referenced.
- **Proxies or VPNs to evade throttling.** Slow down instead; throttling is a
  signal, not an obstacle to route around.
- **Mainnet assets or a mainnet wallet.** Test XLM has no value; never move
  mainnet funds to test a testnet flow.
- **Funding a contract ID.** Friendbot needs a public `G...` account address.

---

## Cautions

- **Never share secret keys.** A secret key (`S...`) or seed phrase must never
  appear in a request URL, chat message, issue, log, or committed file. Only
  ever exchange public `G...` addresses.
- **Never send to contract IDs.** `C...` addresses are contracts, not funded
  accounts. [`../registry/contract-status.md`](../registry/contract-status.md)
  records all four Bridgelet contracts as undeployed: two with `TBD` IDs
  (EphemeralAccount, SweepController) and two with `N/A` IDs (ReserveContract,
  AccountFactory), despite conflicting older artifacts. Do not treat any ID
  from those artifacts as live.
- **Respect the reserve and ledger requirements.** A Stellar account must
  exist on-ledger and hold a minimum balance (base reserve plus any entries)
  before it can pay a fee; leave headroom, not just the exact transfer amount.
  Ledgers close roughly every 5 seconds, so wait for confirmation on Horizon
  before assuming a transfer or faucet request landed.
- **Check for a broader outage first.** If Horizon or Soroban RPC is also
  failing, or the latest ledger went backwards, follow
  [`../runbooks/rpc-outage.md`](../runbooks/rpc-outage.md): pause dependent
  automation, keep evidence, and reconcile after recovery rather than
  re-funding and replaying blindly.

## Related documents

- [`troubleshooting.md`](troubleshooting.md)
- [`bulk-funding-notes.md`](bulk-funding-notes.md)
- [`../runbooks/rpc-outage.md`](../runbooks/rpc-outage.md)
- [`../docs/contact-and-ownership.md`](../docs/contact-and-ownership.md)
- [`../docs/support.md`](../docs/support.md)
- [`../docs/network-config.md`](../docs/network-config.md)
- [`../registry/known-test-accounts.md`](../registry/known-test-accounts.md)
- [`../registry/contract-status.md`](../registry/contract-status.md)
- [`../monitoring/balance-thresholds.md`](../monitoring/balance-thresholds.md)

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-11-18 | Initial alternative funding guidance |
