# Checking Cached Testnet Config for Staleness

When the Bridgelet contracts are redeployed to testnet, they get **new contract
IDs**. That happens after every SDF testnet reset and after any manual
redeploy. An integrator still holding the old IDs in a `.env`, a config file
or a test fixture ends up calling contracts that no longer exist, or worse,
contracts that are no longer the canonical Bridgelet deployment. Nothing
announces this; calls just start to fail or behave strangely.

This page describes a short, **read-only** check to run regularly. It confirms
that your cached config still points at live, correct contracts. None of the
steps submit a transaction or cost fees.

---

## When to run it

- When your app, worker or test suite starts up.
- On a schedule. Once a day is plenty; testnet resets are rare.
- Whenever a call fails with "contract not found", a missing-entry error, or
  an unexpected decode or interface error.
- After any notice in [`../docs/changelog.md`](../docs/changelog.md) or the
  [`../reset/`](../reset/README.md) process.

---

## What to cache alongside the contract IDs

Store these with your cached contract IDs:

| Value | Example | Why |
|---|---|---|
| Contract IDs | `EPHEMERAL_ACCOUNT_CONTRACT_ID=C...` | The values being checked |
| `checked_at_ledger` | `5811113` | Ledger sequence when you last confirmed the IDs. Used to detect a reset (step 1). |
| `source` | `deployment-artifacts/contract-ids.txt @ <commit>` | Shows where the IDs came from |

---

## The check

Prerequisites: stellar-cli 23.x, plus a **funded** testnet identity to use as
the simulation source. Simulation needs an existing source account, but nothing
is signed or submitted. See
[`soroban-cli-identity-setup.md`](soroban-cli-identity-setup.md) to set one up.

### Step 1: Same network, no reset since the last check

```bash
RPC=${STELLAR_SOROBAN_RPC_URL:-https://soroban-testnet.stellar.org}

curl -s -X POST "$RPC" -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getNetwork"}'
curl -s -X POST "$RPC" -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getLatestLedger"}'
```

- The passphrase must be `Test SDF Network ; September 2015`.
- The latest `sequence` must be **greater than or equal to** your cached
  `checked_at_ledger`. If it is **lower**, testnet has been reset since your
  last check. Treat every cached ID as stale and go to
  [If the check fails](#if-the-check-fails).

### Step 2: Each contract exists and responds

Simulate one cheap, read-only function on each contract. `--send no` means
simulate only. Each of these functions works on an uninitialized instance as
well, so a healthy contract never errors on them.

```bash
SRC=testnet-identity   # your funded identity

stellar contract invoke --id "$EPHEMERAL_ACCOUNT_CONTRACT_ID" --network testnet --source "$SRC" --send no -- get_status
# expected: 0 (Active), 1 (PaymentReceived), 2 (Swept) or 3 (Expired)

stellar contract invoke --id "$SWEEP_CONTROLLER_CONTRACT_ID" --network testnet --source "$SRC" --send no -- get_nonce
# expected: a non-negative integer

stellar contract invoke --id "$RESERVE_CONTRACT_CONTRACT_ID" --network testnet --source "$SRC" --send no -- has_base_reserve
# expected: true or false

stellar contract invoke --id "$ACCOUNT_FACTORY_CONTRACT_ID" --network testnet --source "$SRC" --send no -- batch_initialize_count --requests '[]'
# expected: 0
```

**Any error is a failure.** That includes contract not found, missing ledger
entry, archived entry, and "function not found". Don't try to sort errors into
benign and stale; a correct, live contract returns a value for each call above.

### Step 3: The ID points at the *right* contract

Step 2 shows that *something* exists at each ID. Step 3 shows it is the Bridgelet
contract you expect, and not, say, two IDs swapped in your config. Check that
each deployed interface includes a function unique to that contract:

```bash
stellar contract info interface --id "$EPHEMERAL_ACCOUNT_CONTRACT_ID" --network testnet | grep -q record_payment    && echo ok
stellar contract info interface --id "$SWEEP_CONTROLLER_CONTRACT_ID"  --network testnet | grep -q execute_sweep     && echo ok
stellar contract info interface --id "$RESERVE_CONTRACT_CONTRACT_ID"  --network testnet | grep -q set_base_reserve  && echo ok
stellar contract info interface --id "$ACCOUNT_FACTORY_CONTRACT_ID"   --network testnet | grep -q batch_initialize  && echo ok
```

### Step 4: Your IDs match the published IDs

The repository's current deployment record is
[`deployment-artifacts/contract-ids.txt`](../../deployment-artifacts/contract-ids.txt).
Compare your cached values against the upstream copy:

```bash
curl -s https://raw.githubusercontent.com/bridgelet-org/bridgelet-core/main/deployment-artifacts/contract-ids.txt \
  | diff - my-cached-contract-ids.txt && echo "IDs match upstream"
```

If the IDs differ but your old IDs still pass steps 2 and 3, you are talking
to a **superseded** deployment. It still works, but it is no longer canonical.
Switch to the published IDs.

If every check passes, write the current ledger sequence back to
`checked_at_ledger`.

---

## Copy-paste script

`check-testnet-staleness.sh`: exits non-zero if any check fails. It reads IDs
from a `KEY=VALUE` file in the same format as `contract-ids.txt`.

```bash
#!/usr/bin/env bash
# Usage: ./check-testnet-staleness.sh path/to/cached-contract-ids.txt [checked_at_ledger]
set -uo pipefail

IDS_FILE=${1:?path to cached KEY=VALUE contract ID file}
CACHED_LEDGER=${2:-0}
RPC=${STELLAR_SOROBAN_RPC_URL:-https://soroban-testnet.stellar.org}
SRC=${STELLAR_ACCOUNT:-testnet-identity}
EXPECTED_PASSPHRASE="Test SDF Network ; September 2015"
UPSTREAM=https://raw.githubusercontent.com/bridgelet-org/bridgelet-core/main/deployment-artifacts/contract-ids.txt

set -a; source "$IDS_FILE"; set +a
fail=0
rpc() { curl -s -m 20 -X POST "$RPC" -H 'Content-Type: application/json' -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"$1\"}"; }

# 1. network + reset detection
rpc getNetwork | grep -qF "\"passphrase\":\"$EXPECTED_PASSPHRASE\"" \
  || { echo "FAIL: $RPC is not Stellar testnet (or unreachable)"; exit 2; }
LEDGER=$(rpc getLatestLedger | sed -n 's/.*"sequence":\([0-9]*\).*/\1/p')
[ -n "$LEDGER" ] || { echo "FAIL: could not read latest ledger"; exit 2; }
if [ "$LEDGER" -lt "$CACHED_LEDGER" ]; then
  echo "STALE: latest ledger $LEDGER < cached $CACHED_LEDGER -> testnet was reset"; exit 1
fi

# 2 + 3. liveness and identity per contract: id | read-only call | unique function
check() {
  local name=$1 id=$2 fn_unique=$3; shift 3
  if [ -z "$id" ]; then echo "SKIP: $name (no ID cached)"; return; fi
  if ! out=$(stellar contract invoke --id "$id" --network testnet --source "$SRC" --send no -- "$@" 2>&1); then
    echo "STALE: $name $id did not respond: $(echo "$out" | tail -n1)"; fail=1; return
  fi
  if ! stellar contract info interface --id "$id" --network testnet 2>/dev/null | grep -q "$fn_unique"; then
    echo "WRONG: $name $id has no '$fn_unique' function; ID points at a different contract"; fail=1; return
  fi
  echo "ok:    $name $id -> $out"
}
check EphemeralAccount "${EPHEMERAL_ACCOUNT_CONTRACT_ID:-}" record_payment   get_status
check SweepController  "${SWEEP_CONTROLLER_CONTRACT_ID:-}"  execute_sweep    get_nonce
check ReserveContract  "${RESERVE_CONTRACT_CONTRACT_ID:-}"  set_base_reserve has_base_reserve
check AccountFactory   "${ACCOUNT_FACTORY_CONTRACT_ID:-}"   batch_initialize batch_initialize_count --requests '[]'

# 4. drift against the published IDs (warning only)
if upstream=$(curl -sf -m 20 "$UPSTREAM"); then
  diff <(grep -E '^[A-Z_]+=' <<<"$upstream" | sort) <(grep -E '^[A-Z_]+=' "$IDS_FILE" | sort) >/dev/null \
    || echo "WARN:  cached IDs differ from upstream contract-ids.txt; you may be on a superseded deployment"
fi

[ "$fail" -eq 0 ] && echo "PASS at ledger $LEDGER; store this as checked_at_ledger" || exit 1
```

Example:

```bash
chmod +x check-testnet-staleness.sh
./check-testnet-staleness.sh deployment-artifacts/contract-ids.txt 5811113
```

### Without stellar-cli (JavaScript SDK)

To check existence in code, fetch each contract's instance ledger entry. The
SDK throws if it is missing:

```ts
import { rpc, xdr } from "@stellar/stellar-sdk";

const server = new rpc.Server(process.env.STELLAR_SOROBAN_RPC_URL ?? "https://soroban-testnet.stellar.org");

async function contractIsLive(contractId: string): Promise<boolean> {
  try {
    await server.getContractData(
      contractId,
      xdr.ScVal.scvLedgerKeyContractInstance(),
      rpc.Durability.Persistent,
    );
    return true;
  } catch {
    return false; // not found (redeployed/reset) or archived: treat as stale
  }
}
```

This covers step 2 only. Pair it with steps 1 and 4 (`getNetwork`,
`getLatestLedger`, and the upstream ID comparison) for a complete check.

---

## If the check fails

1. **Stop sending writes** (`record_payment`, `execute_sweep`, `claim`, …)
   with the cached IDs.
2. Rule out an RPC problem first. Run the same check against a fallback from
   [`rpc-fallback.md`](rpc-fallback.md). If the fallback passes, the primary
   RPC is the problem, not your config.
3. Get the current IDs from
   [`deployment-artifacts/contract-ids.txt`](../../deployment-artifacts/contract-ids.txt)
   on `main`, and check [`../docs/changelog.md`](../docs/changelog.md) for a
   redeploy or reset notice.
4. If testnet was reset, every **account** is gone too. Re-fund your
   identities and recreate any ephemeral accounts or fixtures; see
   [`../reset/post-reset-checklist.md`](../reset/post-reset-checklist.md).
5. Re-run the check against the new IDs, then update your cache, including
   `checked_at_ledger`.

If the published IDs fail the check too, the repository record is itself out of
date. Report it through [`../docs/support.md`](../docs/support.md) with the
script output.

---

## Related

- [`../registry/verify-contract-live.md`](../registry/verify-contract-live.md): deeper per-contract health checks
- [`rpc-fallback.md`](rpc-fallback.md): alternative RPC endpoints
- [`../runbooks/contract-not-found-error.md`](../runbooks/contract-not-found-error.md)
- [`../reset/post-reset-checklist.md`](../reset/post-reset-checklist.md)
