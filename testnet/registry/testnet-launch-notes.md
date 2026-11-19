# Testnet Launch Notes

Basic historical context: when the Bridgelet testnet deployments were stood up
and by whom, as far as it can be established from public sources, so this isn't
lost when the people involved move on.

> **Provenance.** Everything below comes from (a) this repository's git history
> and (b) public Horizon / Soroban RPC reads performed on **2026-11-18**. Names
> are git commit authors, not confirmed owners; the deployer is identified only
> by its public address. Anything not provable from those sources is marked
> unknown.

## Summary

| Question | Answer |
|---|---|
| When was the first testnet deployment? | The deployer account's first contract upload is **2026-01-13T17:22:41Z**, the same day as the repo's first commits |
| Current deployment | **2026-10-10 ~04:37–04:38 UTC** (`deployments/testnet.json` `deployedAt` = `2026-10-10T04:38:14Z`) |
| By whom? | Transactions came from `GCKGORHRRXLVWDVPR565V6JTMYP43TGH4KH5X7REVW7AL4LMZZLF4PAA` (the deployer / `creatorAddress` / `reserveAdminAddress`). The person holding that key is **not recorded**. Commit authors for deployment artifacts: see below |
| Are the contracts live now? | **Yes, observed 2026-11-18** (below). This conflicts with `testnet/registry/contract-status.md` (dated 2026-09-24, "not deployed"), which predates the 2026-10-10 deployment |

## Timeline

### Git history (authors as recorded by git)

| Date | Commit | Author | Change |
|---|---|---|---|
| 2026-01-13 | `2ef283e` | phertyameen | "smart contracts and basic tests added" |
| 2026-01-13 | `040a583` | phertyameen | removed `DEPLOYER_SECRET` variable from the deploy script |
| 2026-07-10 | `596a30d` | phertyameen | build/test/deploy scripts extended to the account and reserve contracts |
| 2026-07-12 | `741aec2` | phertyameen | "contracts deployed" (deployment artifacts) |
| 2026-07-13 | `3872a8a` | phertyameen | stellar CLI update |
| 2026-10-11 | `e7efd55` | limxiy | "redeployment of contracts except ephemral" (artifacts for the 2026-10-10 deployment) |

### On-chain deployment activity from the deployer account (Horizon)

| UTC time | Activity |
|---|---|
| 2026-01-09T18:16:35Z | Account created with a 10,000 XLM starting balance (`testnet/registry/faucet-funded-vs-genesis-note.md`) |
| 2026-01-13T17:22:41Z | First `UploadContractWasm`, then `CreateContract` |
| 2026-02-16 | Several `CreateContract` / `UploadContractWasm` transactions (~10:06–10:28Z) |
| 2026-05-05 | `UploadContractWasm` + two `CreateContract` (~11:43Z) |
| 2026-07-12 10:27–10:29Z | Full deployment set (uploads, creates, invokes); matches commit `741aec2` |
| 2026-10-10 04:37:12–04:38:12Z | Deployment set recorded in `deployments/testnet.json`; includes `UploadContractWasm` `669a00ec…c24032`, `CreateContract` `24430d82…f93b`, `InvokeContract` `9167e242…5262`, `CreateContract` `d54087e8…58fb`, `InvokeContract` `60dbc9fc…74c7`, `UploadContractWasm` `e8df8e3e…8193`, `CreateContract` `aea9da48…d8ee`, `InvokeContract` `be7b3ccb…e65f` |

Mapping individual transactions to individual contracts was not verified.

## What is live (read-only checks, 2026-11-18, latest ledger ≈ 5,803,170)

| Contract | ID (from `deployments/testnet.json`) | Code fetched? | SHA-256 of fetched WASM |
|---|---|---|---|
| EphemeralAccount | `CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3` | Yes | `5e667ea0687341bdccc81538492143b573777dd04b2450cdeb89b02cee62c58e` (**matches** `ephemeralAccountWasmHash` in `deployments/testnet.json`) |
| SweepController | `CAJB4RRM5JRPXLITC5JUA7Q3LU2WQJDRPXVKMORXWHJJBPZRYWDJ5V4S` | Yes | `c496d74e2a19575f402a39376a131f90dbf3861ec920203d20a4937c6b2e9c41` |
| ReserveContract | `CBG3Y4GWFTL4D37JGV2XWAWC7EUQ7XGH4XM3SFLQYYSKPB2IYMAS54HY` | Yes | `50dc31cadc6b9457bff8f6a1531f850e01663ccf5509bac173c748cdf78c0166` |
| AccountFactory | `CDBY4L3NTLW4OBZZTY73VL3NNWO2AGFFGBRJVDL3IMKDDLFF2W57WK6H` | Yes | `f56240e5714363ec6c7ad274c41fa23128f63cdd906e0c781a0bb2fdf169c038` |

Method: `stellar contract fetch --id <ID> --network testnet --out-file …` then
`shasum -a 256`. Whether these hashes correspond to the current source tree was not
checked.

Read-only simulations (`--send=no`) on the same date:

| Call | Result |
|---|---|
| `SweepController.get_nonce` | `9` |
| `ReserveContract.get_admin` | `GCKGORHRRXLVWDVPR565V6JTMYP43TGH4KH5X7REVW7AL4LMZZLF4PAA` |
| `ReserveContract.get_base_reserve` | `null` (no base reserve set) |
| `EphemeralAccount.get_status` / `get_info` at the deployed ID | status `1` (`PaymentReceived`), `payment_count` 1 (910000000 of the XLM asset contract, timestamp 1791252462), `expiry_ledger` 3587499 (long past), `creator` `GA32P2FG…B37XV5`, `swept_to` set to the recovery address |
| `stellar events` for all four contracts, ledger ≥ 5682300 | "No events" (nothing inside the RPC retention window) |

The deployed ephemeral-account contract is therefore not a blank template: it has
been initialized and used. This is an observation only; its cause was not
investigated. Compare `testnet/runbooks/stuck-ephemeral-account.md`.

## What is unknown

- Who operated the deployer key on each date. Git authors differ (`phertyameen`,
  `limxiy`) and may not be the key holder.
- Why the 2026-10-10 redeploy happened, beyond the commit message
  ("except ephemral").
- Any announcement made at launch; none is recorded in `testnet/docs/changelog.md`.

## Follow-ups

- Update `testnet/registry/contract-status.md`, `wasm-hash-reference.md` and
  `last-verified.json` from the table above (not done in this change).
- Record the deployer key's custodian in `testnet/docs/contact-and-ownership.md`
  or `testnet/security/admin-key-hygiene.md`.
