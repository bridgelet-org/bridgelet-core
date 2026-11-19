# Multi-Asset Testing on Testnet

EphemeralAccount accepts payments in up to **10 different assets**, and a
sweep covers all of them in one call. Testing that properly takes more than
native XLM. This page lists testnet tokens that are already available and
shows how to issue your own.

---

## First: what the contracts do with an asset

Set up your tests with these points in mind. They follow directly from the
current source:

- **`record_payment(amount, asset)` records metadata only.** `asset` is a
  contract `Address`, and the contract doesn't call it, check its balance, or
  check that it implements SEP-41. Pass the asset's **Stellar Asset Contract
  (SAC) address** (`C...`), so the recorded asset matches what is actually on
  the `G...` account.
- **One payment per asset.** Recording the same asset twice fails with
  `DuplicateAsset`. A different asset can be added after the first one, for up
  to 10 (the 11th fails with `TooManyPayments`).
- **No tokens move on-chain during a sweep.** `SweepController::execute_sweep`
  / `claim` and `EphemeralAccount::sweep` / `sweep_claim` validate the request,
  change the account's status, and emit `SweepExecutedMulti` (topic
  `swept_mul`, one entry per asset) from EphemeralAccount and `SweepCompleted`
  (topic `sweep`) from SweepController. The SDK pays out afterwards with classic Horizon
  payments from the ephemeral `G...` address. "Atomic multi-asset" therefore
  means atomic on-chain *authorization and state*. A multi-asset test should
  check the events and `get_info`, and separately check the SDK's classic
  payouts.
- `SweepCompleted.amount` is the plain sum of every asset's amount, mixing
  units across assets. Use `SweepExecutedMulti` or `get_info().payments` for
  per-asset values.

---

## Known testnet tokens

| Asset | Classic asset (`CODE:ISSUER`) | SAC contract ID | How to get it |
|---|---|---|---|
| XLM (native) | `native` | `CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC` | Friendbot (`stellar keys fund <name> --network testnet`) |
| USDC (Circle) | `USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5` | `CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA` | Add a trustline, then use the [Circle testnet faucet](https://faucet.circle.com) (choose Stellar Testnet) |
| EURC (Circle) | `EURC:GB3Q6QDZYTHWT7E5PVS3W7FUT5GVAFC5KSZFFLPU25GO7VTC3NM2ZTVO` | `CCUUDM434BMZMYWYDITHFXHDMIVTGGD6T2I5UKNX5BSLXLW7HVR4MCGZ` | Add a trustline, then use the Circle testnet faucet |

**Last verified: 2026-11-19**, against testnet Horizon `/assets`. The SAC
contract IDs come from Horizon's `contract_id` field. The USDC issuer's home
domain is `centre.io` and the EURC issuer's is `circle.com`. A testnet reset or
a change by the issuer can change these. Before a test run, re-check with:

```bash
curl -s "https://horizon-testnet.stellar.org/assets?asset_code=USDC&asset_issuer=GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5"
stellar contract id asset --asset USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5 --network testnet
```

`stellar contract id asset` works out the SAC address from the asset and
passphrase offline. It doesn't show whether the SAC has actually been deployed;
Horizon's `contract_id` does.

Third-party faucets rate-limit and can go offline. For CI, or for more than
three assets, issue your own tokens (below). You control them completely, and
they don't depend on any outside service.

### Adding a trustline for USDC or EURC

A classic account can't hold a non-native asset until it has a trustline for
it:

```bash
stellar tx new change-trust \
  --source testnet-recipient \
  --line USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5 \
  --network testnet
```

Add the trustline on **every** account that will hold the asset: the sender,
the ephemeral `G...` account, and the sweep destination. A missing trustline on
the destination is the most common reason a multi-asset payout fails in the
SDK.

---

## Issuing your own test tokens

This gives you as many independent assets as you need. Each one is a classic
asset with a SAC deployed, so both Horizon payments and SEP-41 calls work.

```bash
NET=testnet

# 1. An issuer and a distributor (holder) account
stellar keys generate test-issuer      --network $NET --fund
stellar keys generate test-distributor --network $NET --fund
ISSUER=$(stellar keys address test-issuer)

# 2. Pick asset codes (1-12 alphanumeric chars)
for CODE in TSTA TSTB TSTC; do
  ASSET="$CODE:$ISSUER"

  # 3. Distributor trusts the asset
  stellar tx new change-trust --source test-distributor --line "$ASSET" --network $NET

  # 4. Issuer mints by paying the distributor (amount in stroops: 10,000 units = 100000000000)
  stellar tx new payment --source test-issuer \
    --destination "$(stellar keys address test-distributor)" \
    --asset "$ASSET" --amount 100000000000 --network $NET

  # 5. Deploy the asset's SAC so it has a C... contract address (once per asset)
  stellar contract asset deploy --asset "$ASSET" --source test-distributor --network $NET

  # 6. Print the SAC address to use as record_payment's `asset`
  echo "$CODE => $(stellar contract id asset --asset "$ASSET" --network $NET)"
done
```

Notes:

- Running `contract asset deploy` a second time for the same asset fails
  because the SAC already exists. That's harmless; the ID from
  `contract id asset` stays the same.
- You can check balances through the SAC without submitting anything:
  `stellar contract invoke --id <SAC_ID> --network testnet --source test-distributor --send no -- balance --id <G_OR_C_ADDRESS>`.
- Testnet resets wipe your issuer, assets and SACs. Re-run the script
  afterwards, and **don't hard-code self-issued SAC IDs** in shared fixtures.
  Derive them at setup time instead.
- Never use a test issuer's secret anywhere except testnet, and never commit
  it. See [`README.md`](README.md).

A fully custom SEP-41 contract (not a SAC) is only needed if you are testing
Soroban-native token behavior. Bridgelet's payout path is classic, so SACs over
classic assets are the realistic choice.

---

## Multi-asset test outline

Use 2–3 assets (for example XLM plus two you issued) with one ephemeral account:

1. Fund the ephemeral account's `G...` address with each asset; it needs a
   trustline for each non-native one.
2. Call `record_payment` once per asset with the SAC address and the amount
   actually received:

   ```bash
   stellar contract invoke --id "$EA" --network testnet --source <creator> -- \
     record_payment --amount 50000000 --asset <SAC_ID_TSTA>
   stellar contract invoke --id "$EA" --network testnet --source <creator> -- \
     record_payment --amount 25000000 --asset <SAC_ID_TSTB>
   ```

3. Check `get_payment_count` = 2 and that `get_info` lists both. The first
   payment emits `PaymentReceived`; later ones emit `MultiPaymentReceived`.
4. Negative cases:
   - recording `TSTA` again → `DuplicateAsset`
   - `amount` 0 → `InvalidAmount`
   - an 11th distinct asset → `TooManyPayments`
5. Sweep through SweepController (`execute_sweep` or `claim`). Check:
   - `SweepExecutedMulti` lists every asset with its amount;
   - `get_status` = `2` (Swept);
   - `can_sweep` = `false`, and a second sweep fails with `AlreadySwept`.
6. Separately, check the SDK's classic payouts: the destination's balance for
   each asset went up by the recorded amount (Horizon
   `/accounts/<destination>`).

When testing close to the 10-asset limit, read payments page by page with
EphemeralAccount's `get_info_paginated` / `simulate_sweep_paginated` (see
[`../../docs/api-reference.md`](../../docs/api-reference.md)). For history and
event pagination, see
[`../integration/pagination-and-history.md`](../integration/pagination-and-history.md).

---

## Related

- [`../examples/multi-asset-sweep.md`](../examples/multi-asset-sweep.md)
- [`../runbooks/multi-payment-edge-cases.md`](../runbooks/multi-payment-edge-cases.md)
- [`../faucet/README.md`](../faucet/README.md): XLM funding
- [`soroban-cli-identity-setup.md`](soroban-cli-identity-setup.md)
