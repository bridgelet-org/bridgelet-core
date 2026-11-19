# Bulk Test-Account Funding Notes

## Status

Practical patterns for funding many test accounts at once. **No
project-specific measured rate limits are recorded in this repository.** The
numbers and rules below are either documented facts about this repo or generic
observations from existing documents; they are not measurements taken against
Friendbot by Bridgelet.

Friendbot is an SDF-operated public service, not a Bridgelet service. Treat it
as a shared, rate-limited, occasionally unreliable resource.

---

## What is known versus observed

| Statement | Status |
|---|---|
| Friendbot is a public Stellar faucet, not operated by Bridgelet | Documented convention |
| The funding target must be a public `G...` account address, never a `C...` contract ID | Documented convention |
| Ledgers close roughly every 5 seconds; an account needs a minimum reserve and must exist before it can pay fees | Documented in `../docs/network-config.md` |
| Friendbot funding takes ~1–5 s and is rate-limited | Generic observation in `../examples/integration-test-harness-notes.md`; not a Bridgelet measurement |
| Limits are "per IP, wait 60 seconds" | Generic guidance in the earlier stub; unverified, not a recorded limit |
| Limits are "per address" | Generic statement in the harness notes; unverified, not a recorded limit |
| No project-specific measured limits are recorded | Fact about this repository |

Do not turn any of the generic statements into a hard-coded sleep, a guaranteed
throughput figure, or a retry budget presented as a measurement. Verify on
Horizon after each request and adapt.

---

## Recommended patterns

### 1. Small reusable pools

Create a small pool (for example 3–5 accounts) once per suite run and reuse
those funded accounts across tests. This removes Friendbot from the hot path;
the harness notes recommend exactly this and show roughly 4 one-time requests
instead of one per test. Reuse the **funded Stellar account**, but deploy a
fresh `EphemeralAccount` contract instance for any test that needs a sweepable
account—instances are single-use.

### 2. Deterministic keys in the CI secret store

To reuse a pool across runs, derive a fixed set of keypairs deterministically
from a seed held **only** in the CI secret store. Inject the derived keys as
short-lived environment variables at job start. Never commit the seed, the
derived secret keys, or a generated key file; keep any address list in a
gitignored file or as public-only configuration. Rotate the seed if a log or
artifact could have captured a derived secret.

### 3. Serial funding with backoff and jitter

Fund accounts one at a time. Do not use parallel workers or a tight retry
loop against a shared faucet. On failure, cap attempts (for example four),
use exponential backoff with a small random jitter, and re-check the account
on Horizon before each retry. A timeout is an unknown outcome, not a failure
to be replayed blindly.

### 4. Preflight balance checks

Read the native XLM balance on Horizon before requesting funds. Skip accounts
already above your re-fund threshold—the starting values in
[`../monitoring/balance-thresholds.md`](../monitoring/balance-thresholds.md)
are `10 XLM` (`100,000,000` stroops) as a re-fund trigger and `50 XLM` as a
working target. Compare balances as integer stroops, not floating-point XLM.
A failed balance read may mean Horizon is down rather than that the account is
unfunded; check [`../runbooks/rpc-outage.md`](../runbooks/rpc-outage.md) before
treating it as a funding need. A low balance on an existing account is not a
Friendbot top-up: Friendbot may reject an existing account, so use a verified
funded pool or an agreed transfer instead.

### 5. Budget latency from the harness notes

Use [`../examples/integration-test-harness-notes.md`](../examples/integration-test-harness-notes.md)
for the latency model: ~5 s ledgers, 5–15 s per state-changing step, and
Friendbot funding listed generically at 1–5 s. Budget around 15 s per
state-changing step, poll for conditions instead of using a fixed sleep, and
keep the live suite small. If Horizon or Soroban RPC is degraded, pause the
dependent jobs rather than raising retry counts.

---

## Safe shell loop

The loop below uses **public addresses only**, runs serially, checks the
balance before and after each request, and backs off with jitter only for
`429` or `5xx` responses. Other rejections stop immediately instead of being
mistaken for throttling. Keep `${FUNDING_ADDRESSES_FILE}` gitignored (it
contains only `G...` values), and keep `jq` available for the stroop
calculation.

```bash
set -euo pipefail

HORIZON="${STELLAR_HORIZON_URL:-https://horizon-testnet.stellar.org}"
FRIENDBOT="https://friendbot.stellar.org"
ADDRESSES_FILE="${FUNDING_ADDRESSES_FILE:-./.testnet-funding-addresses}"
MIN_STROOPS=100000000
MAX_ATTEMPTS=4

native_stroops() {
  curl --silent --show-error --fail "$HORIZON/accounts/$1" \
    | jq -r '[.balances[] | select(.asset_type == "native") | (.balance | tonumber * 10000000 | floor)] | add // 0'
}

fund_one() {
  local addr="$1" attempt=1 delay balance waited
  while [ "$attempt" -le "$MAX_ATTEMPTS" ]; do
    balance=$(native_stroops "$addr" 2>/dev/null || printf 'missing')
    if [ "$balance" != "missing" ] && [ "$balance" -ge "$MIN_STROOPS" ]; then
      printf 'skip, funded: %s (%s stroops)\n' "$addr" "$balance"
      return 0
    fi
    if [ "$balance" != "missing" ]; then
      printf 'low balance, not a Friendbot top-up: %s (%s stroops); use a verified pool or transfer\n' "$addr" "$balance"
      return 1
    fi

    http_code=$(curl --silent --show-error --output /dev/null \
      --write-out '%{http_code}' "$FRIENDBOT/?addr=$addr" || printf '000')
    case "$http_code" in
      2*)
        waited=0
        while [ "$waited" -lt 15 ]; do
          balance=$(native_stroops "$addr" 2>/dev/null || printf 'missing')
          if [ "$balance" != "missing" ]; then
            printf 'funded: %s (%s stroops)\n' "$addr" "$balance"
            return 0
          fi
          sleep 3
          waited=$((waited + 3))
        done
        printf 'success response, no account yet: %s (recheck later)\n' "$addr"
        return 1
        ;;
      429|5[0-9][0-9])
        if [ "$attempt" -eq "$MAX_ATTEMPTS" ]; then
          break
        fi
        delay=$((2 ** attempt + RANDOM % 3))
        printf 'http %s for %s; retry in %ss\n' "$http_code" "$addr" "$delay"
        sleep "$delay"
        attempt=$((attempt + 1))
        ;;
      4*)
        printf 'rejected with http %s, not a throttle; use a verified pool or transfer: %s\n' "$http_code" "$addr"
        return 1
        ;;
      *)
        printf 'no http response for %s; check status before retrying\n' "$addr"
        return 1
        ;;
    esac
  done
  printf 'giving up: %s\n' "$addr"
  return 1
}

while IFS= read -r addr || [ -n "$addr" ]; do
  case "$addr" in
    G*) fund_one "$addr" || true ;;
    *) printf 'skipped non-G address\n' ;;
  esac
done < "$ADDRESSES_FILE"
```

The `|| true` on each account keeps one failure from aborting the whole run;
the loop still prints it, so no failure is silently dropped. Already-funded
addresses are skipped; existing accounts with a low balance are reported for
a verified pool or transfer rather than sent to Friendbot, and are not
retried as if they were throttled. No line contains a secret key.

---

## Cautions

- **No secrets committed.** Never place an `S...` key, seed, or signature in
  the address list, the shell history, logs, or version control. See
  [`../policies/data-retention.md`](../policies/data-retention.md).
- **No contract IDs.** Friendbot funds `G...` accounts only. A `C...` address
  is a contract, not a classic account. The canonical
  [`../registry/contract-status.md`](../registry/contract-status.md) records
  all four Bridgelet contracts as undeployed: two with `TBD` IDs
  (EphemeralAccount, SweepController) and two with `N/A` IDs (ReserveContract,
  AccountFactory), despite conflicting older artifacts. Do not use those IDs
  as funding targets.
- **Reserve and ledger headroom.** Leave enough XLM for the minimum balance,
  ledger entries, and future fees. Wait for confirmation across ledger closes
  before using a freshly funded account as a fee payer.
- **Be a good citizen.** A tight loop or large parallel fan-out against a
  shared public faucet is both impolite and self-defeating. If throttled,
  slow down; see [`troubleshooting.md`](troubleshooting.md).
- **Reset awareness.** After a testnet reset, previously funded accounts and
  IDs are gone. Do not reuse old keys assuming a balance; reconcile first per
  [`../runbooks/rpc-outage.md`](../runbooks/rpc-outage.md).

## Related documents

- [`README.md`](README.md)
- [`troubleshooting.md`](troubleshooting.md)
- [`alternative-funding.md`](alternative-funding.md)
- [`../examples/integration-test-harness-notes.md`](../examples/integration-test-harness-notes.md)
- [`../config/soroban-cli-identity-setup.md`](../config/soroban-cli-identity-setup.md)
- [`../monitoring/balance-thresholds.md`](../monitoring/balance-thresholds.md)
- [`../policies/data-retention.md`](../policies/data-retention.md)

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-11-18 | Initial bulk funding patterns and safe loop |
