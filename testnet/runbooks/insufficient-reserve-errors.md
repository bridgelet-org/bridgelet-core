# Runbook: Insufficient Reserve Errors

## Two different "reserves"

These are easy to conflate. Establish which one you are looking at first.

| | Stellar network minimum balance | Bridgelet internal reserve tracking |
|---|---|---|
| Defined by | The Stellar protocol | `EphemeralAccount` contract code |
| Meaning | An account must hold a minimum XLM balance: `(2 + subentries) × base reserve` | Bookkeeping of an amount to hand back (`reclaim_reserve`, `reserve` event) after sweep/expiry |
| Enforced by | The network, on every operation | The contract, only in its own state |
| Failure looks like | Operation/transaction result codes | A contract call returning a contract error, or unexpected `ReserveReclaimed` values |
| Value | Network parameter (testnet base reserve; check the current value on the ledger) | `BASE_RESERVE_STROOPS = 1_000_000_000` in `contracts/ephemeral_account/src/lib.rs` (= 100 XLM at 7 decimals) |
| Wired to `ReserveContract`? | n/a | **No.** `ReserveContract` is not read by `EphemeralAccount` (see README, "Implementation Notes") |

Changing `ReserveContract::set_base_reserve` does **not** change either of these
behaviours today.

> Documentation discrepancy to resolve: `testnet/registry/contract-status.md`
> describes the hardcoded constant as 1 XLM, but the source value is
> `1_000_000_000` stroops, i.e. 100 XLM. Verify against the source before relying
> on either figure.

## Recognizing a Stellar minimum-balance failure

| Code | Typical meaning |
|---|---|
| `op_low_reserve` | An operation would add a subentry (trustline, offer, data entry, signer) or create an account without enough balance to cover the new minimum |
| `op_underfunded` | Payment/transfer would take the source below its available balance |
| `tx_insufficient_balance` | Source cannot pay the transaction fee while staying above its minimum |
| `INSUFFICIENT_REFUNDABLE_FEE` / resource-fee errors on `InvokeHostFunction` | The Soroban resource fee is too low; this is a fee problem, not a reserve problem |

Where they appear: Horizon `extras.result_codes`, or the decoded
`errorResultXdr` from Soroban RPC `sendTransaction`.

## Diagnose

1. Fetch the failing account's balance and subentry count:
   ```bash
   curl -s "https://horizon-testnet.stellar.org/accounts/<G_ADDRESS>" \
     | jq '{balances, subentry_count, num_sponsoring, num_sponsored}'
   ```
2. Compute the minimum: `(2 + subentry_count + num_sponsoring - num_sponsored) × base_reserve`.
   Fetch the current `base_reserve_in_stroops` from the latest ledger on Horizon
   (`/ledgers?order=desc&limit=1`). Do not assume a value.
3. Subtract any liabilities/selling liabilities from the spendable balance.
4. If the account is below the minimum or cannot afford the next subentry, it is
   the network reserve. If the account balance is fine and the failure is a
   contract error or surprising `ReserveReclaimed` amount, it is the internal
   reserve logic.

Typical case: a freshly generated key that was never funded (`tx_no_source_account`),
or a trustline/data-entry addition that pushed a barely funded account under the
minimum.

## Resolve

- **Network reserve:** fund the account (`stellar keys fund <name> --network testnet`
  or Friendbot; see `testnet/faucet/README.md`), then retry with a fresh sequence
  number. Fund enough to cover the minimum **plus** fees plus any planned
  subentries.
- **Internal reserve:** check `get_reserve_remaining`, `get_reserve_available`,
  `is_reserve_reclaimed` and `get_last_reserve_event` on the ephemeral account
  (read-only) and compare with the `reserve` events. Do not "fix" it by funding
  the account; the contract's bookkeeping is independent of the classic balance.
- If you changed `ReserveContract::set_base_reserve` expecting a different sweep
  amount: it has no effect on `EphemeralAccount` today.
