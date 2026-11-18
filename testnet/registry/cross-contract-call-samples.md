# Cross-Contract Call Samples: `SweepController` → `EphemeralAccount`

Reference for the cross-contract calls made during a successful `execute_sweep`
and a successful `claim`.

> **Partial / not observed.** The issue asks for real, observed call traces from
> testnet. **No real trace has been captured.** Producing one needs a
> state-changing run (deploy, initialize, record payment, sign, sweep) and, on
> 2026-11-18, no events from the four contracts were retained in the Soroban RPC
> window (`oldestLedger` 5682211) and no sweep transactions were found in the
> deployer account's history. What follows is the call tree **read from the
> contract source**, plus the exact steps to capture a real trace and where to put
> it. Replace the "expected" blocks with captures when you have them.

## Call trees (from source)

### `execute_sweep(ephemeral_account, destination, auth_signature)`

`contracts/sweep_controller/src/lib.rs`

```text
SweepController.execute_sweep
├─ validate_destination            (locked mode: UnauthorizedDestination if different)
├─ AuthContext::verify             (Ed25519 over destination + nonce + contract_id; nonce check)
└─ sweep_account
   ├─ increment_nonce
   ├─ authorize_as_current_contract([ EphemeralAccount.sweep(destination, auth_signature) ])
   ├─ EphemeralAccount.sweep(destination, auth_signature)     ← cross-contract call 1
   │    ├─ state checks (initialized, not swept, payment received, not expired)
   │    ├─ authorized_controller.require_auth()   (satisfied by the controller's pre-authorization)
   │    ├─ status := Swept; swept_to := destination
   │    ├─ emit  swept_mul   SweepExecutedMulti
   │    └─ reclaim_reserve_to → emit reserve ReserveReclaimed
   ├─ EphemeralAccount.get_info()                              ← cross-contract call 2
   └─ emit  sweep  SweepCompleted { ephemeral_account, destination, amount }
```

No token transfer happens in either contract; the SDK performs the payout.

### `claim(recipient, ephemeral_account)`

```text
SweepController.claim
├─ recipient.require_auth()        (recipient's Soroban auth entry; relayer pays fees)
├─ validate_destination(recipient)
├─ EphemeralAccount.get_info()                                 ← cross-contract call 1 (read payments first)
├─ authorize_claim
│    ├─ authorize_as_current_contract([ EphemeralAccount.sweep_claim(recipient) ])
│    └─ EphemeralAccount.sweep_claim(recipient)                ← cross-contract call 2
│         ├─ state checks; controller authorization
│         ├─ status := Swept; swept_to := recipient
│         ├─ emit  swept_mul   SweepExecutedMulti
│         └─ reclaim_reserve_to → emit reserve ReserveReclaimed
└─ emit  sweep  SweepCompleted { ephemeral_account, destination: recipient, amount }
```

Observations to verify in a real trace: the order of `swept_mul`, `reserve` and
`sweep` within the transaction (the source suggests account-side events first,
the controller's `sweep` last); the number of diagnostic entries; and the
resource usage.

## How to capture a real trace

Use throwaway testnet keys only (`testnet/security/testnet-vs-mainnet-key-reuse-warning.md`).

1. Deploy a fresh `EphemeralAccount`, initialize it with the controller as
   `authorized_controller`, and record a payment
   (`testnet/examples/signed-sweep-walkthrough.md`).
2. Submit `execute_sweep` (and, in a second account, `claim`) as described in the
   walkthroughs.
3. Fetch the result and events immediately (RPC retention is limited):
   ```bash
   curl -s https://soroban-testnet.stellar.org -H 'content-type: application/json' \
     -d '{"jsonrpc":"2.0","id":1,"method":"getTransaction","params":{"hash":"<TX_HASH>"}}' \
     > getTransaction.json
   stellar events --id "$EPHEMERAL_ACCOUNT_ID" --network testnet --start-ledger <L> --output json
   stellar events --id "$SWEEP_CONTROLLER_ID"  --network testnet --start-ledger <L> --output json
   ```
   `getTransaction` returns the diagnostic and contract events in `resultMetaXdr`
   (decode with `stellar xdr decode --type TransactionMeta`).
4. Save, per scenario: transaction hash, ledger, the ordered event list, the
   invocation tree if available (simulation output via
   `--sim-only`), and the contract IDs and WASM hashes used.
5. Paste a trimmed version under "Captured traces" below. Never include secrets.

## Captured traces

| Scenario | Tx hash | Ledger | Date | Captured by | Notes |
|---|---|---|---|---|---|
| `execute_sweep` success | _not yet captured_ | | | | |
| `claim` success | _not yet captured_ | | | | |
