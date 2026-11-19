# Friendbot Troubleshooting

## Status

Friendbot is a public Stellar Development Foundation service, **not** a
Bridgelet project service. Bridgelet does not operate it, cannot guarantee its
availability or response time, and provides best-effort support only. Most
failures are generic network, throttle, or upstream-service events; diagnose
them as such.

---

## Before you retry

1. **Verify on Horizon before retrying.** Read the public `G...` address at
   `https://horizon-testnet.stellar.org/accounts/<YOUR_PUBLIC_KEY>`. A HTTP
   success from Friendbot is not proof that the balance increased, and a
   timeout is not proof that the request failed.
2. **Record the evidence.** UTC time, exact URL (public address only), HTTP
   status or error text, and the Horizon balance you observed.
3. **Retry with backoff.** Use bounded exponential backoff with jitter and a
   small attempt cap. Never retry in a tight loop.
4. **Never put a secret key (`S...`), seed, or signature in a URL or log.**
   Only the public `G...` address belongs in a faucet request.

If Horizon itself is unavailable, stop and follow
[`../runbooks/rpc-outage.md`](../runbooks/rpc-outage.md) instead of treating a
failed balance read as "account not funded".

---

## Common failures and next steps

| Symptom | Likely meaning | Next step |
|---|---|---|
| Response says the account already exists / is already funded | Friendbot normally funds a new account; an existing account may be rejected, and a top-up is not guaranteed | Check the balance on Horizon first. If it is funded, do nothing and reuse the account. If it is low, use a verified funded pool or an agreed transfer ([`alternative-funding.md`](alternative-funding.md)). Do not keep requesting top-ups or treat repeated rejections as throttling. |
| Malformed address, invalid `addr`, or client-side validation error | The `addr` value is not a valid public `G...` key; it may contain whitespace, a `C...` contract ID, or an `S...` secret | Regenerate or read the address with `stellar keys address <identity>` and resend the corrected public key. Never paste a secret key. |
| `429 Too Many Requests` or an explicit throttle message | Generic rate-limit signal | Verify on Horizon, then wait and retry with backoff and jitter. Existing docs describe limits "per IP" and "per address" inconsistently; neither is a recorded project measurement, so do not assume a fixed wait. Do not use a proxy or VPN to evade it. |
| `5xx` from the faucet, or a connection failure that looks like service down | Upstream/public-service failure | Record the response, check the official Stellar status page, and wait. Pause dependent automation per [`../runbooks/rpc-outage.md`](../runbooks/rpc-outage.md); report a persistent, reproducible failure as a `[testnet]` issue. |
| DNS or TLS failure | Local resolver, network path, or a mistyped endpoint—not necessarily a public outage | Confirm the endpoint against [`../docs/network-config.md`](../docs/network-config.md). Preserve local evidence before declaring an outage; retry from another environment only to establish scope, not to bypass a limit. |
| Timeout with unknown outcome | The request may still have landed | Do **not** resubmit. Poll Horizon for the account and balance first. If the account appears, you are done; if it does not after several ledger closes, wait and retry once with backoff. |
| Account not found after a success response | Propagation/ledger timing, a wrong address/network, or a testnet reset | Re-read the exact public key, confirm the passphrase `Test SDF Network ; September 2015`, wait a few ledger closes (~5 s each), and poll Horizon. If old IDs and accounts no longer resolve, treat it as a reset—see below. |
| Testnet reset (ledgers go backwards, old accounts/IDs vanish) | The SDF resets testnet periodically; all state is wiped | Stop replaying pre-reset work, do not reuse old credentials or assume prior balances, reconcile deployment records, and follow section 6 of [`../runbooks/rpc-outage.md`](../runbooks/rpc-outage.md). Re-funding a fresh key is a separate step. |

---

## Choosing a next step

```text
Friendbot request failed
|
+-- Can you read the account on Horizon?
|   +-- yes, funded -> no action; reuse it
|   +-- yes, low balance -> a top-up is not guaranteed; Friendbot may
|       reject an existing account. Use a verified funded pool or an
|       agreed transfer (see alternative-funding.md). Repeated
|       "already exists" rejections are a wrong path, not throttling
|   +-- no -> outcome unknown; do not retry yet
|
+-- DNS/TLS or client validation error? -> fix the local configuration first
+-- 429 -> backoff with jitter, then verify on Horizon before another attempt
+-- 5xx / timeout -> record, check status, pause dependent work, retry later
+-- Success but no account -> wait for ledger closes, poll Horizon, check for reset
```

For alternatives when the faucet stays unavailable, see
[`alternative-funding.md`](alternative-funding.md). For funding many accounts
without a request per account, see [`bulk-funding-notes.md`](bulk-funding-notes.md).

---

## Escalation

If the failure affects several integrators or persists beyond ordinary
upstream flakiness, open a **`[testnet]`** issue with the `testnet` label per
[`../docs/support.md`](../docs/support.md). Include the transaction hash if
one was submitted, the public address, and the observed response. Response is
best effort and no time is guaranteed. Do not report Friendbot as a Bridgelet
service outage.

## Related documents

- [`README.md`](README.md)
- [`alternative-funding.md`](alternative-funding.md)
- [`bulk-funding-notes.md`](bulk-funding-notes.md)
- [`../runbooks/rpc-outage.md`](../runbooks/rpc-outage.md)
- [`../docs/network-config.md`](../docs/network-config.md)
- [`../docs/support.md`](../docs/support.md)
- [`../config/soroban-cli-identity-setup.md`](../config/soroban-cli-identity-setup.md)

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-11-18 | Initial Friendbot failure and recovery guidance |
