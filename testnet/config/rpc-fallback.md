# Soroban RPC Fallback Endpoints for Testnet

The primary Bridgelet testnet RPC is the SDF-hosted
`https://soroban-testnet.stellar.org`. It sometimes has downtime or
degraded performance. This page lists alternative public Soroban RPC
endpoints for **Stellar Testnet** so integrators aren't completely blocked
while the primary is down.

> **Best effort, no SLA.** These endpoints are run by third parties, not by
> Bridgelet or the SDF. They can change, add rate limits, start requiring an
> API key, or go away without notice. Before relying on one, always run the
> [verification check](#verify-an-endpoint-before-using-it).

---

## Endpoints

| Provider | Testnet Soroban RPC URL | API key | Notes |
|---|---|---|---|
| SDF (**primary**) | `https://soroban-testnet.stellar.org` | No | Default in `.env.example` and `scripts/deploy-testnet.sh`. `getNetwork` also returns `friendbotUrl`. |
| Gateway.fm | `https://soroban-rpc.testnet.stellar.gateway.fm` | No | Public endpoint. Its `getNetwork` response leaves out `friendbotUrl`; that is expected. |
| Ankr | `https://rpc.ankr.com/stellar_testnet_soroban` | No (public tier) | Public tier is rate-limited. Ankr offers paid tiers with higher limits. |
| Self-hosted | `http://localhost:8000/rpc` | No | Run your own testnet RPC; see [below](#self-hosted-fallback). Doesn't depend on any third party, but has to sync first. |

**Last verified: 2026-11-19.** Each public endpoint above answered
`getNetwork` with the passphrase `Test SDF Network ; September 2015` (protocol
30), and its `getLatestLedger` sequence was within a few ledgers of the SDF
endpoint. If you re-verify these endpoints, update this date.

Other providers (for example Validation Cloud, QuickNode, NOWNodes and
Blockdaemon) offer Stellar testnet RPC behind an account and API key. Use them
if your organization already has an account, but don't commit API-keyed URLs
to this repository.

### Horizon is a separate problem

This page covers **Soroban RPC only**. Horizon (`https://horizon-testnet.stellar.org`)
can't stand in for Soroban RPC, and Soroban RPC can't stand in for Horizon. See
[`../runbooks/rpc-outage.md`](../runbooks/rpc-outage.md) for how to handle a
Horizon outage.

---

## Verify an endpoint before using it

Run both checks against the candidate URL:

```bash
RPC=https://soroban-rpc.testnet.stellar.gateway.fm   # candidate

# 1. Correct network? The passphrase must be exactly "Test SDF Network ; September 2015"
curl -s -X POST "$RPC" -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getNetwork"}'

# 2. Up to date? Compare the sequence with the primary (or with a second fallback)
curl -s -X POST "$RPC" -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getLatestLedger"}'
```

Use the endpoint only if:

- the passphrase matches exactly. A different passphrase means a different
  network, and transactions signed for it will fail on testnet;
- its ledger sequence is no more than a few ledgers (≈5 s each) behind the
  other endpoints, and still advancing when you check again; and
- it works for a read-only call against a Bridgelet contract. See
  [`staleness-check.md`](staleness-check.md) for one.

---

## Using a fallback

### stellar-cli

Register each fallback once under its own name:

```bash
stellar network add testnet-gatewayfm \
  --rpc-url https://soroban-rpc.testnet.stellar.gateway.fm \
  --network-passphrase "Test SDF Network ; September 2015"

stellar network add testnet-ankr \
  --rpc-url https://rpc.ankr.com/stellar_testnet_soroban \
  --network-passphrase "Test SDF Network ; September 2015"
```

During an outage, swap `--network testnet` for `--network testnet-gatewayfm`
(or `testnet-ankr`). Contract IDs, account IDs and transaction hashes are the
same on every endpoint, because they all serve the same network.

### Environment variables

Point `STELLAR_SOROBAN_RPC_URL` at the fallback and leave
`STELLAR_NETWORK_PASSPHRASE` unchanged:

```bash
export STELLAR_SOROBAN_RPC_URL=https://soroban-rpc.testnet.stellar.gateway.fm
```

### In application code (reads)

For **read-only** calls such as `simulateTransaction`, `getLedgerEntries` and
`getLatestLedger`, an ordered list of endpoints with simple failover is safe:

```ts
import { rpc } from "@stellar/stellar-sdk";

const RPC_URLS = [
  "https://soroban-testnet.stellar.org",            // primary
  "https://soroban-rpc.testnet.stellar.gateway.fm", // fallback 1
  "https://rpc.ankr.com/stellar_testnet_soroban",   // fallback 2
];

async function withFallback<T>(fn: (s: rpc.Server) => Promise<T>): Promise<T> {
  let lastErr: unknown;
  for (const url of RPC_URLS) {
    try {
      return await fn(new rpc.Server(url));
    } catch (err) {
      lastErr = err; // network error, timeout, 5xx: try the next endpoint
    }
  }
  throw lastErr;
}

const ledger = await withFallback((s) => s.getLatestLedger());
```

### Writes (`sendTransaction`)

**Never** retry submissions across endpoints automatically. If a
`sendTransaction` call times out:

1. Keep the transaction hash. It's the same on every endpoint.
2. Call `getTransaction` with that hash on a **healthy** endpoint until the
   status is `SUCCESS` or `FAILED`, or until the transaction's time bounds
   have passed and it can no longer be included.
3. Submit a new transaction only after you know the original didn't apply.

On-chain sequence numbers stop the same signed envelope from applying twice.
A *new* transaction built after a blind retry has no such protection, and it
can double-record a payment or use a sweep nonce you didn't intend to.

---

## Caveats when switching endpoints

- **Event and history retention differs.** Each RPC keeps only a limited window
  of ledgers for `getEvents` and `getTransaction`, and the window size depends
  on how the provider configured it. An event cursor or `startLedger` saved
  from one endpoint may be outside the window on another. When you switch, check
  the `oldestLedger` field in the `getLatestLedger`/`getEvents` responses.
- **Lag.** A fallback that is a few ledgers behind can return state that is
  slightly stale. After writing, wait for the write to show up before reading
  it back through a different endpoint.
- **Rate limits.** Public tiers throttle. Back off on HTTP 429 instead of
  hammering a fallback.
- **Testnet resets apply to every endpoint.** A fallback can't bring back
  contracts lost in an SDF testnet reset. If every endpoint says a contract is
  missing, follow [`staleness-check.md`](staleness-check.md), not this page.

---

## Self-hosted fallback

If you don't want to depend on third parties, run a local testnet RPC node
with the Stellar quickstart image (requires Docker):

```bash
stellar container start testnet
# or, without stellar-cli:
docker run --rm -d -p 8000:8000 --name stellar-testnet stellar/quickstart --testnet --enable rpc
```

The RPC is served at `http://localhost:8000/rpc`. A new node has to catch up
with the network before it can answer queries. Start it **before** you need it,
and run the [verification checks](#verify-an-endpoint-before-using-it) until
the ledger sequence matches the public endpoints.

---

## Related

- [`../runbooks/rpc-outage.md`](../runbooks/rpc-outage.md): full outage runbook (pausing automation, in-flight transactions, recovery)
- [`staleness-check.md`](staleness-check.md): check that cached contract IDs still exist
- [`../docs/network-config.md`](../docs/network-config.md): primary network values
- [Stellar status page](https://status.stellar.org/)
