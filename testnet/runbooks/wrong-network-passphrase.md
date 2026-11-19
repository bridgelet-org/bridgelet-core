# Runbook: Wrong Network Passphrase

## Symptom

A transaction you built and signed is rejected at submission even though the
account, sequence number and fee look right. This is easy to mistake for a
contract-level authorization failure; it is not one.

## Why it happens

A Stellar transaction signature covers the transaction **and the network
passphrase** (hashed into the network ID). Sign with the wrong passphrase and
every signature is invalid for the network you submit to. Typical causes:
copy-pasting a mainnet config (`Public Global Stellar Network ; September 2015`)
into a testnet SDK setup, or the reverse, or typing the testnet passphrase
without the exact spacing.

The correct testnet value is in `testnet/docs/network-config.md`:

```text
Test SDF Network ; September 2015
```

## How to recognize it

| Where you see it | Signature | What it means |
|---|---|---|
| Soroban RPC `sendTransaction` | status `ERROR` with a transaction result of `txBAD_AUTH` (base64 `errorResultXdr`; decode with `stellar xdr decode --type TransactionResult`) | Envelope signature does not verify |
| Horizon transaction submit | `tx_bad_auth` in `extras.result_codes.transaction` | Same |
| SDK exception | message mentioning bad auth / bad signature | Same |

How it differs from a contract failure:

| Wrong passphrase | Contract authorization failure |
|---|---|
| Rejected at the **transaction** layer, before the contract runs | Surfaces as a contract error or `HostError`, e.g. `Error(Contract, #N)` or `Error(Auth, ...)` |
| No contract events, no state change, no fee charged for the contract call | Simulation usually already shows the failure |
| **Simulation succeeds** (simulation does not check envelope signatures) | Simulation fails |
| Contract error numbers (see `testnet/examples/error-handling.md`) never appear | Error numbers appear |

The giveaway: **simulation passes but submission fails with a bad-auth code.**
If the failure is a numbered contract error (for example `Unauthorized (8)` or
`AuthorizationFailed (3)`), look elsewhere
(`testnet/runbooks/failed-sweep-signature.md`).

Soroban authorization **entries** signed for `claim()` also embed the network ID,
so a recipient signing an auth entry with the wrong passphrase produces an
invalid authorization even if the envelope passphrase is right.

## Diagnose

1. Print the passphrase your signing code actually uses, character by character
   (spaces around `;`, no trailing newline, no smart quotes).
2. Ask the RPC what network it serves:
   ```bash
   curl -s https://soroban-testnet.stellar.org -H 'content-type: application/json' \
     -d '{"jsonrpc":"2.0","id":1,"method":"getNetwork"}'
   ```
   The `passphrase` in the response is the value to sign with.
3. Check the stellar-cli network profile: `stellar network ls` and confirm the
   passphrase/RPC pair is the testnet one.
4. Confirm the RPC URL and passphrase are from the **same** network; a testnet
   passphrase with a mainnet RPC (or the reverse) fails the same way.

## Resolve

- Set the passphrase from `testnet/docs/network-config.md` (or the `getNetwork`
  response) in a single shared config, and re-sign. A transaction signed with the
  wrong passphrase must be rebuilt and re-signed; it cannot be fixed in place.
- Do not reuse a sequence number you consumed elsewhere; rebuild from a fresh
  `getAccount`.
- Add a start-up assertion in your integration: call `getNetwork` and compare to
  the configured passphrase, failing fast on mismatch.

> Never debug this by pasting a secret key into a tool or log.
