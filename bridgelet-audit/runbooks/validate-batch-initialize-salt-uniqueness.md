<!--
Purpose: Pre-flight validation for operators before calling AccountFactory::batch_initialize
in production. Mitigates risk that a deterministic account address derived from a
given salt/index range is already in use.
Owner: @JudeDaniel6 (closes #290 — bridgelet-audit/ knowledge-base initiative).
Status: Documentation-only. No contract changes are introduced by this file.
-->

# Pre-Flight: Validating `batch_initialize` Salt Uniqueness

> **Purpose.** Catch collisions in the deterministic address space used by
> `AccountFactory::batch_initialize` **before** burning a transaction fee and,
> worse, ending up with controls attributed to an address that was already in
> circulation.

| Field | Value |
| :--- | :--- |
| **Related issue** | [#290](https://github.com/bridgelet-org/bridgelet-core/issues/290) |
| **Owner / reviewer** | `_operator-name_` |
| **Target network** | `testnet` / `mainnet` |
| **Target factory contract** | `_C...factory-address_` |
| **Ephemeral WASM hash** | `_hex32_` |
| **Index range under consideration** | `0..N` |
| **Last reviewed** | `_ISO-8601 date_` |

## Table of Contents

1. [Background](#background)
2. [When To Run This Runbook](#when-to-run-this-runbook)
3. [Step 1 — Reproduce the Salt Derivation](#step-1--reproduce-the-salt-derivation)
4. [Step 2 — Compute the Expected Addresses](#step-2--compute-the-expected-addresses)
5. [Step 3 — Check the Ledger for Existing Contracts](#step-3--check-the-ledger-for-existing-contracts)
6. [Step 4 — Decide Go / No-Go](#step-4--decide-go--no-go)
7. [Step 5 — Run the Companion Checklist](#step-5--run-the-companion-checklist)
8. [Related Issues](#related-issues)

---

## Background

`AccountFactory::batch_initialize(creator, requests)` deterministically derives
each ephemeral account's contract address from three inputs:

| Input | Source in code | Our variable |
| :--- | :--- | :--- |
| Deploying contract address | `env.current_contract_address()` (the factory instance) | `$FACTORY_ADDRESS` |
| 32-byte salt | `salt_bytes = [0u8; 32]` then `salt_bytes[28..32] = (index as u32).to_be_bytes()` | `$SALT_BYTES[i]` for each `i` |
| Wasm hash | Loaded from `DataKey::EphemeralAccountWasmHash` in instance storage | `$WASM_HASH` |
| Init args | `()` (empty tuple, see `batch_initialize`) | n/a |

Per request, the derivation looks like:

```text
salts[i]        = 28 bytes of 0x00 || (i as u32, big-endian)
addr[i]         = deterministic_address(FACTORY_ADDRESS, salts[i], WASM_HASH, ())
account_client  = EphemeralAccountClient::new(env, addr[i])
result[i]       = account_client.try_initialize(creator, expiry_ledger, recovery_address, controller)
```

The important consequences for operators:

- **Determinism.** The address for index `i` is fully determined by the factory
  contract, the wasm hash, and `i`. Any callsite using the *same* factory
  instance and the *same* wasm hash will produce the same address sequence —
  even from a different process, host, or continent.
- **No `i` collision check happens in the contract.** `batch_initialize`
  loops the requests and deploys each one without first asking the ledger
  whether the address is already occupied. If the address is already in use
  by a *different* code path (or a previous deployment of the same factory),
  `deploy_v2` will either succeed against an unrelated contract or fail in a
  way that is easy to misread.

That second property is the reason this runbook exists.

---

## When To Run This Runbook

Run this runbook **every time** any of the following are about to change:

- [ ] A new range of `AccountInitRequest` indices is about to be submitted to
      `batch_initialize`.
- [ ] The factory contract is being deployed to a network where another
      factory instance (perhaps from a previous incarnation) already exists.
- [ ] The ephemeral account WASM is being upgraded. A different wasm hash
      invalidates the entire address space — every previously-safe index `i`
      is now a fresh surface to check.

Skip this runbook only if **all three** of these hold and the covering
ticket explicitly says so:

- The full intended range has been issued through the same factory at the
  same wasm hash within the last ledger close, **and**
- No new index is being added at either end of the range, **and**
- The factory contract address and WASM hash match the values last recorded
  on the issue/PR.

---

## Step 1 — Reproduce the Salt Derivation

Generate the index sequence you are about to submit. Use the exact rule from
`AccountFactory::batch_initialize`: 32 bytes, the last 4 bytes are the index
encoded as a big-endian `u32`, the rest are zero.

```bash
# Build each 32-byte salt for the intended index range [0, N).
N=50
mkdir -p /tmp/bridgelet-batch-preflight
for i in $(seq 0 $((N-1))); do
  python3 - <<PY > "/tmp/bridgelet-batch-preflight/salt-$(printf '%05d' "$i").bin"
import struct, sys
salt = bytearray(32)
salt[28:32] = struct.pack(">I", $i)
sys.stdout.buffer.write(salt)
PY
done
ls -1 /tmp/bridgelet-batch-preflight/ | wc -l
```

Sanity-check the output:

```bash
# Every file should be exactly 32 bytes and start with 28 zero bytes.
for f in /tmp/bridgelet-batch-preflight/salt-*.bin; do
  size=$(stat -c %s "$f")
  head4=$(xxd -p -l 28 "$f")
  [ "$size" = "32" ] && [ "$head4" = "$(printf '0%.0s' {1..28})" ] || echo "BAD: $f"
done
```

If any file is the wrong size or has a non-zero prefix, **stop**; the rest of
this runbook is meaningless until the salt derivation matches the contract.

---

## Step 2 — Compute the Expected Addresses

The exact format of a Soroban contract address is `Strkey-encoded Contract`
(`C...`). The deterministic-address algorithm is identical for all `deploy_v2`
invocations performed by the same deploying contract against the same wasm
hash with the same init args, so it is reproducible offline from the *contract
source itself*.

`soroban-cli` does not currently expose a stable "predict address" verb in
22.x, so do **not** rely on shell-level flag combinations here — instead,
compute the candidate addresses from a tiny helper that mirrors the SDK's
deterministic-address routine, or call it directly from your operator
service. The rule is: **the address you compute must be the address the
network would compute given the same inputs**. If your helper cannot produce
a result, treat that as a **no-go** and stop.

Sketch of a self-contained helper, derived from the contract source so it
stays correct even as the SDK surface evolves:

```python
# pip install stellar-sdk (or use the host's vendored copy)
from stellar_sdk import Address
from stellar_sdk.contract import Contract
import hashlib, struct

FACTORY  = "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4"  # factory address
WASM     = open("path/to/ephemeral_account.wasm", "rb").read()
WASM_HASH = hashlib.sha256(WASM).hexdigest()  # 32 bytes / 64 hex chars
INIT_ARGS = b""  # batch_initialize deploys with empty args

def predict_address(index: int) -> str:
    salt = bytearray(32)
    salt[28:32] = struct.pack(">I", index)
    # The exact deterministic-address routine used by with_current_contract(salt)
    # is implemented by the host SDK's Contract helper; this is its public
    # contract: same factory + same wasm hash + same salt + same init_args =>
    # same C... address.
    return Contract.predict_address(
        deployer=FACTORY, salt=bytes(salt), wasm_hash=WASM_HASH, init_args=INIT_ARGS
    )

with open("/tmp/bridgelet-batch-preflight/indexes.txt") as f:
    indexes = [int(line.strip()) for line in f if line.strip()]

addresses = [(i, predict_address(i)) for i in indexes]
with open("/tmp/bridgelet-batch-preflight/addresses.tsv", "w") as out:
    for i, addr in addresses:
        out.write(f"{i}\t{addr}\n")
```

Adapt the helper invocation to whatever host SDK version the operator is
running. Replace `predict_address` with whatever the SDK actually calls the
deterministic-address routine — the contract-source-derived salt above is the
canonical input.

Now deduplicate in case of accidental collisions **inside** the requested
range:

```bash
awk -F'\t' '{print $2}' /tmp/bridgelet-batch-preflight/addresses.tsv \
  | sort -u | wc -l
# Expectation: the line count must equal the number of input indexes.
# If it is smaller, two different indices map to the same address — stop
# and re-check the index encoding.
```

---

## Step 3 — Check the Ledger for Existing Contracts

For each candidate address, verify the ledger does not already contain an
installed contract at that address. `soroban-cli` 22.x does not expose a
stable `contract read` verb, so check via the RPC `getLedgerEntries`
request or via the host SDK equivalent — whatever is in use at the
operation site.

Sketch using the host SDK (adapt to the version actually deployed):

```python
# pip install stellar-sdk
import requests
import sys

RPC_URL = "https://soroban-testnet.stellar.org"  # or mainnet RPC
ADDRESSES = [line.strip() for line in open(
    "/tmp/bridgelet-batch-preflight/addresses.tsv").read().splitlines()]
ADDRESSES = [line.split("\t")[1] for line in ADDRESSES if line]

def ledger_key_for_contract(address):
    # Stellar contract storage is keyed by a LedgerKey whose shape depends
    # on the contract; the safe general check is to ask the RPC for the
    # entry shape used by Soroban-installed contracts and treat "found" as
    # "already deployed". This skeleton leaves the precise key shape to the
    # host SDK; the semantics are what matter.
    return {"contractData": {"contract": address, "key": "CONTRACT_INSTANCE"}}

collisions = []
for addr in ADDRESSES:
    resp = requests.post(RPC_URL, json={
        "jsonrpc": "2.0", "id": 1,
        "method": "getLedgerEntries",
        "params": [ledger_key_for_contract(addr)],
    }).json()
    entries = resp.get("result", {}).get("entries", [])
    if any(e.get("state") is not None for e in entries):
        collisions.append(addr)

print(f"collisions: {len(collisions)}")
for c in collisions:
    print(f"COLLISION: {c}")
sys.exit(1 if collisions else 0)
```

If the SDK / RPC surface you have deployed does not behave the way this
sketch assumes, **stop and escalate** — do not interpret unknown response
shapes as "no collision". Silence is not proof of absence.

Repeat the check **once** after a short delay (≥ 10 seconds) for any address
that did not collide on the first pass, in case the ledger response was
cached at the RPC endpoint. **Do not skip this step.**

---

## Step 4 — Decide Go / No-Go

| Condition | Decision |
| :--- | :--- |
| Every candidate address is unused and the index-to-address mapping is one-to-one | **Go.** Submit `batch_initialize` as planned. |
| One or more candidate addresses already contain a contract | **No-go.** Stop. Report the offending indices in the ticket. Do **not** re-number the requests, do **not** retry, do **not** "skip" the colliding index. Open an incident. |
| The candidate addresses themselves are well-formed (`C...`) but the read tool cannot resolve them | **Treat as a no-go** until the RPC tooling is confirmed healthy. Do not assume silence equals "no contract". |
| Two different indices produced the same address | **No-go.** Suspect a bug in your salt-derivation reproduction. Re-derive from `contracts/account_factory/src/lib.rs` and re-run from step 1 before retrying. |

When in doubt, choose **no-go**. The cost of a false negative (collision in
production) is far higher than the cost of an extra pre-flight.

---

## Step 5 — Run the Companion Checklist

Once the salt uniqueness pre-flight is signed off, move to the
[`SweepController initialization checklist`](../checklists/sweep-controller-initialization-checklist.md)
to verify the wider go-live posture (destination mode, signer custody, and the
authorized-controller binding of the accounts this batch is about to spin up).
The two are intentionally designed to be executed in sequence.

---

## See Also

- `bridgelet-audit/README.md` — folder index.
- `bridgelet-audit/checklists/sweep-controller-initialization-checklist.md` —
  closely related; both must be green before re-pointing production traffic
  through a new factory / controller pair.
- `bridgelet-audit/runbooks/verify-claim-vs-execute-sweep-nonce-state.md` —
  orthogonal pre-flight covering the destination-lock verification path.

---

## Related Issues

- **#290** — this runbook.
- **#295** — SweepController initialization checklist.
- **#288** — Verifying nonce state before trusting `update_authorized_destination`'s lock
  (sibling runbook).
