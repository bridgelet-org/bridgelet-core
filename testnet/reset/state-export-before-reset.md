# Exporting State Before a Reset

Before wiping or redeploying testnet, snapshot anything worth keeping for
debugging or historical reference. Contract state does not survive a reset
(`testnet/policies/reset-policy.md`), and neither does the ability to query it.

> Status: this is a process definition. No export tooling exists in this
> repository; the commands use `stellar-cli` and `curl` against Soroban RPC.
> Run the export while the contracts are still live, and **before** the redeploy
> that replaces them.

## 1. Decide what is worth keeping

| Item | Keep? | Why |
|---|---|---|
| Final state of long-lived demo accounts (`testnet/registry/known-test-accounts.md`) | Yes | Reference for docs and regression comparison |
| Edge-case accounts (expired with partial reserve, multi-asset, locked mode, nonce > 0) | Yes, labelled | Hard to recreate exactly |
| Current registry IDs, WASM hashes, config (`deployments/testnet.json`, registry tables) | Yes | Needed to interpret the export |
| Controller nonce and authorized destination/signer public key | Yes | Explains signatures and behaviour |
| Event history for the above | Yes, if still within the RPC retention window | Not recoverable later |
| Throwaway test accounts | No | Noise |
| Any secret key or signing seed | **Never** | Do not export secrets |

## 2. Export procedure

Create a directory for the export, for example `exports/<UTC-date>/` (kept
outside the repository, or committed only if it contains no sensitive data):

```bash
STAMP=$(date -u +%Y%m%dT%H%M%SZ)
OUT="export-$STAMP"; mkdir -p "$OUT"

# Network state
curl -s https://soroban-testnet.stellar.org -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getLatestLedger"}' > "$OUT/latest-ledger.json"
cp deployments/testnet.json "$OUT/deployments.testnet.json"
cp testnet/registry/last-verified.json "$OUT/last-verified.json"

# Controller
stellar contract invoke --id "$SWEEP_CONTROLLER_ID" --source "$READER" --network testnet \
  --send=no -- get_nonce > "$OUT/controller-nonce.txt"

# Each account worth keeping
for ID in $ACCOUNT_IDS; do
  stellar contract invoke --id "$ID" --source "$READER" --network testnet --send=no -- get_info   > "$OUT/$ID.get_info.json"
  stellar contract invoke --id "$ID" --source "$READER" --network testnet --send=no -- get_status > "$OUT/$ID.get_status.txt"
  stellar contract events --id "$ID" --network testnet --start-ledger "$START_LEDGER" --output json > "$OUT/$ID.events.json"
done

# Controller events
stellar contract events --id "$SWEEP_CONTROLLER_ID" --network testnet \
  --start-ledger "$START_LEDGER" --output json > "$OUT/controller.events.json"
```

Optionally also store raw storage reads (`testnet/examples/reading-contract-storage.md`).

## 3. Annotate

Add a short `README.md` to the export: why each account was kept, what scenario it
represents, the network passphrase, the ledger range, the date, and the exporter.
Without this, snapshots are not interpretable later.

## 4. Store and reference

- Store the export somewhere durable and shared with maintainers.
- Add a line to `testnet/docs/changelog.md` for the reset: where the export lives.
- Review for sensitive data before sharing (no secrets; public keys and contract
  IDs only).

## 5. Verify before wiping

Open at least one exported `get_info` and one events file and check they are
non-empty and decode. Do not start the reset until this passes.

## Limits

Soroban RPC only retains a window of recent ledgers; events older than that cannot
be exported. Run the export soon after anything you care about happens, not only
right before a reset. See `testnet/policies/who-can-request-a-reset.md` for when
resets may happen and `testnet/reset/communication-template.md` for notices.
