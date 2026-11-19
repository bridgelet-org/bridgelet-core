# Runbook: Relayer-Side Failures on the Gas-Free `claim()` Path

## Context

`SweepController::claim(recipient, ephemeral_account)` is submitted by a
**relayer** that pays fees, while the **recipient** only signs a Soroban
authorization entry (`testnet/config/gas-free-claim-testing.md`). Failures can
therefore happen on two different sides. This runbook isolates the relayer side.

## Is it the relayer or the recipient?

| Evidence | Points to |
|---|---|
| Simulation fails with an `Error(Auth, ...)` or a contract error (`UnauthorizedDestination (13)`, `AccountExpired (6)`, `InvalidAccount (1)`, `AccountNotReady (5)`) | **Recipient / contract side**: bad or missing auth entry, wrong recipient, locked mode, account state |
| Simulation succeeds but submission is rejected with a transaction-level code (`tx_bad_seq`, `tx_insufficient_fee`, `tx_insufficient_balance`, `tx_bad_auth`, `tx_no_source_account`, `txTOO_LATE`) | **Relayer side** |
| `sendTransaction` returns `TRY_AGAIN_LATER` / `DUPLICATE` / timeouts | **Relayer or network side** |
| Transaction lands but fails in the host function (resource limit, insufficient refundable fee) | **Relayer fee/resources** |

The key discriminator: **if simulation of the exact transaction (with the
recipient's auth entry attached) passes, the recipient's signature is fine.**

## Relayer failure modes

| Mode | Signature | Check | Fix |
|---|---|---|---|
| Relayer out of funds | `tx_insufficient_balance`, `op_underfunded`, or fee-bump source underfunded | Horizon `/accounts/<relayer>`: balance vs minimum (see `testnet/runbooks/insufficient-reserve-errors.md`) | Fund via Friendbot or an intermediary (`testnet/faucet/README.md`) |
| Wrong/low fee | `tx_insufficient_fee`; or host failure on refundable/resource fee | Compare the fee with the simulation's `minResourceFee` plus inclusion fee | Re-simulate, rebuild with the returned resource fee (add headroom), resubmit |
| Stale sequence number | `tx_bad_seq` | Compare the sequence used with `getAccount` | Fetch a fresh sequence; serialize submissions per relayer account |
| Concurrent submissions | Intermittent `tx_bad_seq` | More than one worker shares one relayer account | Use a queue or a channel-account pool |
| Auth entry expired | Simulation/submit fails with an auth expiration error | `signatureExpirationLedger` of the entry vs latest ledger | Have the recipient re-sign with a later expiration; do not let it run far ahead |
| Wrong passphrase on relayer | `tx_bad_auth` with a passing simulation | `testnet/runbooks/wrong-network-passphrase.md` | Fix the relayer config |
| RPC/network trouble | timeouts, 5xx, ledger not advancing | `testnet/runbooks/rpc-outage.md` | Pause; do not resubmit blindly |
| Duplicate/replayed submission | `DUPLICATE`, or contract state already `Swept` | `get_status` on the account, `get_nonce` on the controller | Do not resubmit; verify state first (`testnet/examples/idempotency-considerations.md`) |

## Procedure

1. Record the transaction hash, relayer address, sequence used, fee, and the
   exact error.
2. Re-run simulation on the same transaction. Pass → relayer side; fail → recipient/contract side.
3. Check relayer balance, sequence and fee as above.
4. Before resubmitting, confirm on-chain state with read-only calls: a prior
   attempt may have succeeded even if the response was lost
   (`get_status` = `Swept (2)`, nonce incremented).
5. Resubmit once with corrected parameters, then verify events (`swept_mul`
   from the account, `sweep` from the controller).

## Prevent

- Alert on relayer balance (`testnet/monitoring/balance-thresholds.md`).
- Always add fee headroom over the simulated minimum.
- Never log the recipient's signed payload alongside secrets; never log relayer secret keys.
