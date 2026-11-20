# Wallet Compatibility for Manual Testnet Signing QA

## Scope

This page is for manual (non-scripted) QA of Bridgelet testnet flows that involve
`authorized_controller` / recipient signing. It answers a practical question:
when a signature-related step fails, is the wallet at fault or the contract?

The status labels below reflect **community reports and common practice, not a
verified compatibility matrix maintained by this repo**. No CI job here tests
wallet integrations, and wallet behavior changes between releases. Treat every
"works" as "reported to work — verify on the version you have", and always
reproduce a suspected contract bug with the CLI/`stellar-cli` before filing it
against the contracts.

> Network: Stellar testnet (`Test SDF Network ; September 2015`)
> Soroban RPC: `https://soroban-testnet.stellar.org`
> Network values in machine-readable form: [`network.toml`](network.toml)

## What "signing" means in these flows

It is important to separate two very different signing operations, because
wallets only help with one of them.

| Flow | Who signs | What is signed | Can a wallet do it? |
|---|---|---|---|
| `execute_sweep` (signed path) | The authorized signer | Ed25519 signature over `SHA256(destination_xdr || nonce_be_u64 || contract_id_xdr)` | Generally **no**. Use `tools/sweep-signer`. This is a custom hash, not a standard transaction or auth entry. |
| `claim` (gas-free path) | The recipient | A Soroban **auth entry** for `claim(recipient, ephemeral_account)` | Sometimes. Wallets that expose a Soroban `signAuthEntry` API can produce this; support varies by wallet. |
| Regular account operations | The account owner | A Stellar **transaction** (funding, setting up a custom network, etc.) | Yes, all wallets listed here can sign standard transactions. |

For the signed `execute_sweep` path, the canonical signer tool in this repo is
`tools/sweep-signer`. It uses `soroban-sdk` itself to XDR-serialize addresses and
to produce a byte-identical signature. Do not expect a browser wallet to output
the raw 64-byte `auth_signature` that `execute_sweep` expects; if a wallet cannot
produce it, that is expected, not a wallet bug.

For the gas-free `claim` path, see
[`gas-free-claim-testing.md`](gas-free-claim-testing.md): the recipient authorizes
via a Soroban auth entry while a relayer submits the transaction and pays fees.

## Compatibility table

Statuses are deliberately conservative. "Community-reported" means people have
reported the capability working for Soroban testnet; it is not an endorsement or
a guarantee for your wallet version.

| Wallet | Form | Testnet account support | Soroban auth-entry signing | Status / known quirks |
|---|---|---|---|---|
| Freighter | Browser extension | Yes (add/switch to Testnet) | Reported supported via its dapp API (`signAuthEntry`-style flows) | Most commonly reported working for Soroban testnet dapps. Verify the connected network is Testnet; a Mainnet connection makes signatures fail with confusing auth errors. |
| xBull | Browser extension | Yes (add/switch to Testnet) | Reported supported via its dapp API | Community-reported Soroban testnet support including auth entries. Confirm the extension is on the same network as the dapp and that the recipient account is funded. |
| Lobstr | Hosted / extension / mobile | Testnet support reported in some clients | Unclear; verify in your client | Account access on testnet is reported, but auth-entry signing support is not consistently documented. If it cannot sign an auth entry, use it only to hold/fund the recipient and sign the claim with a CLI identity. |
| Albedo | Web wallet (keyless, key-based sign-in) | Testnet support reported | Uncertain for Soroban auth entries | Reported for transaction signing via its API. Because keys are not held locally in the same way, auth-entry support should be verified before relying on it for `claim()` QA. |
| StellarTerm | Web client | Custom-network / Testnet support reported | Not a Soroban dapp signer | Primarily a classic transaction and trading client. Useful for checking balances and funding testnet accounts, but do not assume it can produce a Soroban auth entry. |

Notes that apply to all entries:

- Wallet releases change quickly. A capability reported here may have been added,
  removed or renamed in your version. Check the wallet's own documentation and
  test with a trivial auth before attributing a failure to the contract.
- A wallet being able to connect to Testnet is not the same as it being able to
  sign a Soroban authorization. Check both separately.
- The CLI is the reference implementation. If a wallet flow fails, reproduce the
  same call with `stellar-cli` (see
  [`soroban-cli-identity-setup.md`](soroban-cli-identity-setup.md)).

## Adding or switching to Testnet in a wallet

The exact menu labels differ by wallet, so use this as a sequence of checks
rather than a fixed recipe:

1. Open the wallet's network settings (often under Settings > Network, or a
   network selector at the top of the app).
2. Select **Testnet** if it is built in. If you must add it manually, use:
   - Network name: any label you like, e.g. `Testnet`
   - Network passphrase: `Test SDF Network ; September 2015` (the spaces around
     `;` are significant)
   - Soroban RPC URL: `https://soroban-testnet.stellar.org`
   - Horizon URL: `https://horizon-testnet.stellar.org`
   - Friendbot / faucet: `https://friendbot.stellar.org`
3. Save the network and switch the active network to it. In browser-extension
   wallets, also confirm the dapp connection prompt shows Testnet, not Mainnet.
4. Fund the recipient account through Friendbot before signing anything. An
   unfunded account cannot authorize or pay its own way on the ledger.
5. Verify the account in Horizon, for example
   `https://horizon-testnet.stellar.org/accounts/<G...ADDRESS>`. A 404 means the
   account is not funded yet.

If your wallet only lets you pick from a list and has no Testnet option, use a
different wallet for the signing step rather than guessing at custom-network
settings; a wrong passphrase will produce signatures that no testnet contract
accepts.

## Common wallet-specific failure modes

| Symptom | Likely wallet-side cause | What to check |
|---|---|---|
| Signature rejected / `require_auth` fails | Wrong active network (Mainnet passphrase used) | The connection prompt and network settings must both say Testnet with the exact passphrase above |
| Dapp cannot see the account | Wallet locked, extension not connected, or wrong account selected | Unlock the wallet, reconnect the dapp, select the intended recipient account |
| Recipient account not found | Account unfunded or testnet was reset | Fund via Friendbot and re-check in Horizon |
| Wallet signs a transaction but the contract still says "not authorized" | A transaction signature is not a Soroban auth entry | Use a wallet that exposes `signAuthEntry`, or sign the claim with the CLI |
| Signature produced but `execute_sweep` rejects it | Wallet produced a transaction signature, not the custom sweep hash | Use `tools/sweep-signer` for the Ed25519 sweep-authorization signature |
| Works in one wallet, fails in another | Different auth-entry support or different network selection | Reproduce with the CLI; compare the network/passphrase shown in each wallet |
| Everything fails after a testnet reset | Accounts/contracts wiped; wallet may be pointing at an old contract ID | Refund accounts and reload current IDs from `deployments/testnet.json` |

## Troubleshooting checklist

- [ ] Confirm the wallet is on **Testnet** with passphrase
  `Test SDF Network ; September 2015`.
- [ ] Confirm the RPC endpoint is `https://soroban-testnet.stellar.org`.
- [ ] Confirm the recipient account is funded and visible in Horizon.
- [ ] Confirm which signing operation the flow needs: a Soroban auth entry
  (`claim`) or the custom sweep hash (`execute_sweep`, via `tools/sweep-signer`).
- [ ] Confirm the wallet actually supports that operation; a transaction signer
  is not automatically an auth-entry signer.
- [ ] Reproduce the same call with `stellar-cli` to isolate a wallet issue from
  a contract issue.
- [ ] Log the exact error and which wallet/build produced it. Do not assume a
  specific wallet version from a community report.
- [ ] Re-check the contract ID and nonce after any redeploy or testnet reset
  (see [`nonce-tracking.md`](nonce-tracking.md)).

## Related documentation

- [`gas-free-claim-testing.md`](gas-free-claim-testing.md) - recipient + relayer
  `claim()` flow
- [`nonce-tracking.md`](nonce-tracking.md) - nonce-safe signing pre-flight
- [`soroban-cli-identity-setup.md`](soroban-cli-identity-setup.md) - CLI identity
  and network setup
- [`network.toml`](network.toml) - machine-readable testnet network values
- [`docs/SIGNATURE_FORMAT.md`](../../docs/SIGNATURE_FORMAT.md) - the
  `execute_sweep` signature format
- [`tools/sweep-signer/`](../../tools/sweep-signer/) - canonical off-chain signer
