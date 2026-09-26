# Testnet Explorer Links

## Status

The IDs below are copied from `deployments/testnet.json`,
`deployment-artifacts/contract-ids.txt`, and `testnet/docs/network-config.md`.
They are **recorded candidate IDs**, not a claim that the contracts are
currently live. The canonical status registry still reports that the contracts
have not been verified, and a Stellar testnet reset can invalidate every ID.

Always perform the read-only checks in
[`../registry/verify-contract-live.md`](../registry/verify-contract-live.md)
before treating an explorer link as an active deployment.

## Recorded contract links

| Contract | Recorded ID | Stellar Expert | Lumenscan |
|---|---|---|---|
| `EphemeralAccount` | `CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3` | [contract](https://stellar.expert/explorer/testnet/contract/CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3) | [account](https://testnet.lumenscan.io/account/CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3) |
| `SweepController` | `CBEU4X5MNGOECBSTNEUFMBALH2YI5YV4UIH7YXRNOLR2DNLZQD4Z5KWE` | [contract](https://stellar.expert/explorer/testnet/contract/CBEU4X5MNGOECBSTNEUFMBALH2YI5YV4UIH7YXRNOLR2DNLZQD4Z5KWE) | [account](https://testnet.lumenscan.io/account/CBEU4X5MNGOECBSTNEUFMBALH2YI5YV4UIH7YXRNOLR2DNLZQD4Z5KWE) |
| `ReserveContract` | `CACYMICFLHSPKVMCK336IEGCWAQB4XBC7P7HTJSQY3R7Y2YASE4AX3MS` | [contract](https://stellar.expert/explorer/testnet/contract/CACYMICFLHSPKVMCK336IEGCWAQB4XBC7P7HTJSQY3R7Y2YASE4AX3MS) | [account](https://testnet.lumenscan.io/account/CACYMICFLHSPKVMCK336IEGCWAQB4XBC7P7HTJSQY3R7Y2YASE4AX3MS) |
| `AccountFactory` | `CARBMZY4466SWP3RTN3DR2F4JYNR5UCXD2ICFISV2YI4TS3AAZV2TQ24` | [contract](https://stellar.expert/explorer/testnet/contract/CARBMZY4466SWP3RTN3DR2F4JYNR5UCXD2ICFISV2YI4TS3AAZV2TQ24) | [account](https://testnet.lumenscan.io/account/CARBMZY4466SWP3RTN3DR2F4JYNR5UCXD2ICFISV2YI4TS3AAZV2TQ24) |

## Copy-ready links

### `EphemeralAccount`

```text
https://stellar.expert/explorer/testnet/contract/CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3
https://testnet.lumenscan.io/account/CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3
```

Useful source topics are `created`, `payment`, `multi_pay`, `swept_mul`,
`expired`, and `reserve`.

### `SweepController`

```text
https://stellar.expert/explorer/testnet/contract/CBEU4X5MNGOECBSTNEUFMBALH2YI5YV4UIH7YXRNOLR2DNLZQD4Z5KWE
https://testnet.lumenscan.io/account/CBEU4X5MNGOECBSTNEUFMBALH2YI5YV4UIH7YXRNOLR2DNLZQD4Z5KWE
```

Useful source topics are `sweep`, `dest_auth`, and `dest_upd`.

### `ReserveContract`

```text
https://stellar.expert/explorer/testnet/contract/CACYMICFLHSPKVMCK336IEGCWAQB4XBC7P7HTJSQY3R7Y2YASE4AX3MS
https://testnet.lumenscan.io/account/CACYMICFLHSPKVMCK336IEGCWAQB4XBC7P7HTJSQY3R7Y2YASE4AX3MS
```

The current source uses `init` and `reserve` topics. This contract is not
wired into `EphemeralAccount` by the current testnet deployment tooling.

### `AccountFactory`

```text
https://stellar.expert/explorer/testnet/contract/CARBMZY4466SWP3RTN3DR2F4JYNR5UCXD2ICFISV2YI4TS3AAZV2TQ24
https://testnet.lumenscan.io/account/CARBMZY4466SWP3RTN3DR2F4JYNR5UCXD2ICFISV2YI4TS3AAZV2TQ24
```

`AccountFactory` emits no direct event in the current source. Its generated
`EphemeralAccount` children emit their own `created` events.

## Network entry points

| Resource | URL |
|---|---|
| Stellar Expert testnet | <https://stellar.expert/explorer/testnet> |
| Lumenscan testnet | <https://testnet.lumenscan.io/> |
| Stellar Testnet Horizon | <https://horizon-testnet.stellar.org/> |
| Stellar Testnet Soroban RPC | <https://soroban-testnet.stellar.org/> |
| Friendbot | <https://friendbot.stellar.org/> |
| Network passphrase | `Test SDF Network ; September 2015` |

Lumenscan is a secondary, account-oriented view. Its availability and Soroban
contract rendering are not guaranteed by this repository. Prefer Stellar
Expert plus direct RPC/Horizon evidence when the two explorers disagree.

## Dynamic links

Fresh child `EphemeralAccount` instances have IDs that cannot be listed here.
Construct the same routes after each run:

```text
https://stellar.expert/explorer/testnet/contract/<EPHEMERAL_ACCOUNT_ID>
https://testnet.lumenscan.io/account/<EPHEMERAL_ACCOUNT_ID>
https://stellar.expert/explorer/testnet/tx/<TRANSACTION_HASH>
```

Never guess or truncate a transaction hash. Preserve the exact value returned
by the submitter.

## Link and deployment verification

Before declaring a link healthy:

1. Confirm the network passphrase is the exact testnet value above.
2. Confirm the Soroban ledger is advancing.
3. Invoke an expected read-only method through a simulation.
4. Use `get_nonce` for `SweepController`.
5. Use `get_info` on a known initialized child account rather than relying on
   the `get_status` fallback for an uninitialized instance.
6. Compare the current on-chain WASM/code hash with a deployment record.
7. Record the UTC verification time, ledger, and evidence.

An explorer's HTTP 200 response proves only that a page was served. A missing
page can indicate a stale ID, a testnet reset, indexing delay, or an explorer
limitation; it is not automatically a contract-health verdict.

## Maintenance

Reconcile these links with:

- `deployments/testnet.json`
- `deployment-artifacts/contract-ids.txt`
- `testnet/docs/network-config.md`
- `testnet/registry/contract-status.md`
- `testnet/registry/last-verified.json`

If those files disagree, keep the warning and do not label an ID verified.
Update this page only after recording the evidence that made the change.
