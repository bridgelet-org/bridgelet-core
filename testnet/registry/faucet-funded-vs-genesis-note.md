# Faucet-Funded vs Other Funding Sources

For the record: which testnet accounts used by Bridgelet were funded by Friendbot
(and so are subject to Friendbot's funding amount and to any testnet reset wipe)
versus any other source. Written for accurate troubleshooting later.

## Method

For each account, the first operation in its Horizon history was read
(`/accounts/<G>/operations?order=asc&limit=1`) on **2026-11-18**, and the funder
and starting balance recorded. No keys were used; Horizon is public and read-only.

```bash
curl -s "https://horizon-testnet.stellar.org/accounts/<G_ADDRESS>/operations?order=asc&limit=1"
```

## Observed

| Role (source) | Address | Created (UTC) | First operation | Funder | Starting balance | Balance on 2026-11-18 |
|---|---|---|---|---|---|---|
| Deployer; `creatorAddress` and `reserveAdminAddress` in `deployments/testnet.json` | `GCKGORHRRXLVWDVPR565V6JTMYP43TGH4KH5X7REVW7AL4LMZZLF4PAA` | 2026-01-09T18:16:35Z | `create_account` | `GAIH3ULLFQ4DGSECF2AR555KZ4KNDGEKN4AFI4SU2M7B43MGK3QJZNSR` | 10,000 XLM | ~19,975.8 XLM |
| `recoveryAddress` in `deployments/testnet.json` | `GAEG3HJONLNMHB6L37YX5QHHBYWAKDQZ4GV7UPOWUPI6TOKICHG2PRE6` | 2026-04-21T19:03:59Z | `create_account` | same funder | 10,000 XLM | ~7,085.2 XLM native; also a USDC trustline with balance 0 |
| `creator` of the initialized `EphemeralAccount` instance at `CB7Z22T…` (`get_info`) | `GA32P2FGYTT5OAN6GWNMUEYNFUJODWIYTX3KB4HBEHASH6IBRPB37XV5` | 2026-01-14T15:54:45Z | `create_account` | same funder | 10,000 XLM | ~9,946.96 XLM |

All three were created by the **same funder account** with a **10,000 XLM**
starting balance, which is the standard Friendbot grant described in
`testnet/faucet/README.md`. Treat all three as **Friendbot-funded**.

> Caveat: I did not independently confirm that `GAIH3ULL…SR` is the Friendbot
> account (for example by funding a fresh address through Friendbot and comparing
> the funder). The 10,000 XLM amount and the single common funder across
> unrelated accounts and dates are consistent with it. Confirm before relying on
> this.

No account in this set was funded from genesis or any other source. Accounts
that appear only as placeholders in examples (`<funded-testnet-identity>`,
`$PAYER`, `$RELAYER`) were not observed and are out of scope.

## Why it matters

| Property | Consequence |
|---|---|
| Friendbot-funded | Balance started at Friendbot's grant; further top-ups go through Friendbot or an intermediary (`testnet/faucet/rate-limit-workaround-etiquette.md`) |
| Subject to a testnet wipe | If the SDF resets testnet, these accounts disappear along with the contracts (`testnet/policies/reset-policy.md`). Creation dates in January–April 2026 show these accounts persisted for months in the history Horizon served, so wipes have not been constant, but none of this is guaranteed |
| Deployer balance | The deployer has submitted many deployment transactions (see `testnet/registry/testnet-launch-notes.md`) and still holds ~19,976 XLM, so funding is not currently a constraint |
| The recovery address has a USDC trustline | Balance 0 at the time of observation; recovered funds in other assets would need a matching trustline (see `testnet/examples/expiry-and-recovery.md`) |

## Keeping this note accurate

Re-run the method whenever the deployer, recovery or relayer addresses change, and
add a row. Record new fundings in `testnet/faucet/funding-audit-log.md` if that
file exists.
