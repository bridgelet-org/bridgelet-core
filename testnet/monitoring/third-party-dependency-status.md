# Third-Party Dependency Status

Testnet usage depends on external services this project does **not** control.
When something fails, the first triage question is: *is this us or is this
upstream?* This is the single reference point for that question.

> Status pages and endpoints below are the commonly used public ones and should
> be re-verified occasionally; this document does not poll them and nothing here
> is monitored automatically.

## Dependencies

| Dependency | Used for | Endpoint | Status / info |
|---|---|---|---|
| Friendbot | Funding testnet accounts | `https://friendbot.stellar.org/?addr=<G...>` | Stellar status: <https://status.stellar.org> |
| Horizon (testnet) | Classic payments, balances, account history | `https://horizon-testnet.stellar.org` | <https://status.stellar.org> |
| Soroban RPC (testnet) | Simulation, submission, contract state, contract events | `https://soroban-testnet.stellar.org` | <https://status.stellar.org> |
| Stellar Expert (testnet) | Explorer links used in docs and triage | `https://stellar.expert/explorer/testnet` | Third-party site; no status page assumed |
| Stellar testnet itself | The network (periodically reset by the SDF) | n/a | SDF announcements; see `testnet/policies/reset-policy.md` |

Endpoints are also listed in `testnet/docs/network-config.md`; keep them in sync.

## "Us or upstream?" quick test

| Observation | Likely |
|---|---|
| Soroban RPC `getHealth`/`getLatestLedger` fails or the ledger is not advancing | Upstream RPC (`testnet/runbooks/rpc-outage.md`) |
| Horizon endpoints error while Soroban RPC works | Upstream Horizon |
| Friendbot returns 429/5xx | Rate limit or upstream (`testnet/faucet/README.md`) |
| Stored contract IDs suddenly missing and ledger sequence dropped | Testnet reset (`testnet/policies/reset-policy.md`) |
| Everything upstream is healthy but a call fails with a contract error number | Us / our contracts (`testnet/examples/error-handling.md`) |
| Simulation passes, submission fails at transaction level | Usually client config (passphrase, fee, sequence) |

Quick health probe:

```bash
curl -s https://soroban-testnet.stellar.org -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'
curl -s -o /dev/null -w "%{http_code}\n" https://horizon-testnet.stellar.org
```

## Recording an upstream incident

When an upstream outage coincides with a Bridgelet incident, note the dependency,
the UTC window and the status-page link in the incident write-up
(`testnet/runbooks/incident-postmortem-template.md`) and tag any false alert in
`testnet/monitoring/false-positive-log.md` (if present).

## Maintenance

Review this list when `testnet/docs/network-config.md` changes or when a new
external service is relied on. Do not record API keys or private endpoints here.
