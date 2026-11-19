# Setting Up a stellar-cli Identity and Network for Testnet

This guide covers the three `stellar` CLI steps a new contributor needs before
touching the Bridgelet testnet contracts:

1. `stellar keys generate`: create a local identity
2. `stellar keys fund`: fund it from the testnet Friendbot
3. `stellar network add`: register the testnet RPC endpoint and passphrase

Every command is scoped to **Stellar Testnet**. You should not need to read
`scripts/deploy-testnet.sh` to work out the right invocations.

> `soroban-cli` was renamed to `stellar-cli`, and the binary is now `stellar`.
> Old `soroban keys ...` / `soroban network ...` commands map one-to-one onto
> `stellar keys ...` / `stellar network ...`. This guide uses only the new name.

---

## Testnet values used in this guide

| Setting | Value |
|---|---|
| CLI network name | `testnet` (built in to stellar-cli) |
| Network passphrase | `Test SDF Network ; September 2015` |
| Soroban RPC URL | `https://soroban-testnet.stellar.org` |
| Horizon URL | `https://horizon-testnet.stellar.org` |
| Friendbot | `https://friendbot.stellar.org` |

The same values are available in machine-readable form in
[`network.toml`](network.toml). Load that file in code instead of copying the
values from this table.

The passphrase must match **exactly**, including the spaces around `;`.

---

## Prerequisites

- stellar-cli **23.x**. The repo pins `23.4.1`:

  ```bash
  cargo install --locked stellar-cli --version 23.4.1
  stellar --version
  ```

- Network access to the RPC and Friendbot URLs above.

Shell note: the multi-line examples use bash `\` line continuations. In
PowerShell, put the command on one line or use a backtick (`` ` ``) instead.

---

## 1. Check the network config

stellar-cli ships with a built-in network named `testnet`, so
`--network testnet` works on a fresh install:

```bash
stellar network ls
```

`testnet` should appear in the list. If it does, go to step 2.

### Registering testnet explicitly with `stellar network add`

Register the network yourself if you want the RPC URL and passphrase pinned in
your own config, or if your CLI build doesn't list `testnet`. Use a distinct
name so it can't be confused with the built-in entry:

```bash
stellar network add bridgelet-testnet \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015"

stellar network ls
```

From then on, pass `--network bridgelet-testnet` wherever this guide says
`--network testnet`.

To remove the entry later:

```bash
stellar network rm bridgelet-testnet
```

### Making testnet the default (optional)

```bash
stellar network use testnet
```

You can also set the default per shell with an environment variable:

```bash
export STELLAR_NETWORK=testnet          # bash
$env:STELLAR_NETWORK = "testnet"        # PowerShell
```

Even with a default set, passing `--network testnet` explicitly keeps
copy-pasted commands unambiguous.

---

## 2. Generate an identity with `stellar keys generate`

```bash
stellar keys generate bridgelet-dev --network testnet
```

This creates a new random keypair saved under the name `bridgelet-dev`. Read
its public key (the `G...` account ID) with:

```bash
stellar keys address bridgelet-dev
```

List every identity the CLI knows about:

```bash
stellar keys ls
```

### Generating and funding in one step

`--fund` makes the CLI call Friendbot right after generating the key, which
combines this step with step 3:

```bash
stellar keys generate bridgelet-dev --network testnet --fund
```

### Where the secret key lives

By default, the identity is saved as a **plaintext** TOML file in the
stellar-cli config directory: `~/.config/stellar/identity/<name>.toml`, or a
project-local `.stellar/identity/` if one exists in the current directory. It
is **not** encrypted.

- Don't create a project-local `.stellar/` directory inside this repository,
  because it could end up committed.
- On a shared machine, pass `--secure-store` to keep the secret in the OS
  keychain instead.

---

## 3. Fund the identity with `stellar keys fund`

A testnet account doesn't exist on the ledger until something funds it.
Friendbot sends free test XLM:

```bash
stellar keys fund bridgelet-dev --network testnet
```

`stellar keys fund` only works on networks that have a Friendbot (testnet and
futurenet). It fails on mainnet, which is what you want.

### Funding without the CLI

If the CLI's Friendbot call fails, call Friendbot directly with the public key:

```bash
curl "https://friendbot.stellar.org/?addr=$(stellar keys address bridgelet-dev)"
```

### Checking that the account is funded

stellar-cli doesn't have a `keys balance` command. Ask Horizon instead:

```bash
curl -s "https://horizon-testnet.stellar.org/accounts/$(stellar keys address bridgelet-dev)"
```

A funded account returns JSON with a `balances` array that includes a `native`
entry. An unfunded account returns HTTP 404.

If Friendbot is rate-limiting you, see [`../faucet/`](../faucet/README.md) for
alternative funding routes.

---

## 4. Choosing a default identity (optional)

```bash
stellar keys use bridgelet-dev
```

Or set it per shell:

```bash
export STELLAR_ACCOUNT=bridgelet-dev    # bash
$env:STELLAR_ACCOUNT = "bridgelet-dev"  # PowerShell
```

With both defaults set, `--network testnet --source bridgelet-dev` can be left
off later commands.

---

## 5. Smoke test

These checks confirm the network config and the identity work. None of them
submits a transaction.

Check that the RPC endpoint is healthy:

```bash
curl -s -X POST https://soroban-testnet.stellar.org \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'
```

Simulate a read of your own native XLM balance through the native Stellar
Asset Contract. `--send no` means simulate only, never submit:

```bash
stellar contract invoke \
  --id "$(stellar contract id asset --asset native --network testnet)" \
  --network testnet \
  --source bridgelet-dev \
  --send no \
  -- \
  balance --id "$(stellar keys address bridgelet-dev)"
```

If this returns a balance (in stroops: 1 XLM = 10,000,000), your identity,
funding and network config are all working.

To call the Bridgelet contracts themselves, continue with
[`../docs/soroban-cli-quickref.md`](../docs/soroban-cli-quickref.md). Look up
contract IDs in
[`deployment-artifacts/contract-ids.txt`](../../deployment-artifacts/contract-ids.txt),
and verify one with
[`../registry/verify-contract-live.md`](../registry/verify-contract-live.md)
before you rely on it.

---

## 6. One identity per role

Bridgelet flows involve several actors. Give each one its own testnet identity
so that a key leaked in one role doesn't compromise the others:

```bash
stellar keys generate bridgelet-deployer  --network testnet --fund   # deploys/administers contracts
stellar keys generate bridgelet-relayer   --network testnet --fund   # pays fees when submitting claims/sweeps
stellar keys generate bridgelet-recipient --network testnet --fund   # signs as the end recipient
```

Testnet identities are disposable. To regenerate an identity under the same
name, use `--overwrite`:

```bash
stellar keys generate bridgelet-relayer --network testnet --fund --overwrite
```

---

## 7. After a testnet reset

When the SDF resets testnet, every account is wiped. Your local identity files
survive the reset, but the accounts they point to no longer exist on the
ledger. Fund each identity again:

```bash
stellar keys fund bridgelet-dev --network testnet
```

See [`../reset/post-reset-checklist.md`](../reset/post-reset-checklist.md) for
the full checklist.

---

## 8. Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| `Account not found` / Horizon 404 | Identity was never funded, or testnet was reset | `stellar keys fund <name> --network testnet` |
| Friendbot returns 4xx/5xx or times out | Rate limit, or Friendbot is down | Wait and retry, or use [`../faucet/alternative-funding.md`](../faucet/alternative-funding.md) |
| `network ... not found` | Typo in the network name, or a custom name that was never added | `stellar network ls`; re-run `stellar network add` from step 1 |
| `bad auth` / signature rejected | Wrong network passphrase | Use exactly `Test SDF Network ; September 2015`; see [`../runbooks/wrong-network-passphrase.md`](../runbooks/wrong-network-passphrase.md) |
| Can't reach RPC | Network/proxy issue, or an RPC outage | Run the `getHealth` check in step 5; see [`../runbooks/rpc-outage.md`](../runbooks/rpc-outage.md) |
| `identity already exists` | Name already used | Pick a new name or add `--overwrite` |

---

## 9. Security notes

- **Never commit a secret key** (`S...`) or seed phrase, and don't put one in
  any file under `testnet/config/`. See [`README.md`](README.md).
- Treat testnet identities as throwaway, and never reuse a testnet secret on
  mainnet.
- `stellar keys secret <name>` prints the raw secret. Don't run it in shared
  terminals or paste its output into issues or logs.

---

## Reference

- [stellar-cli repository](https://github.com/stellar/stellar-cli)
- [`network.toml`](network.toml): machine-readable testnet network values
- [`../docs/network-config.md`](../docs/network-config.md): network values in prose, with SDK snippets
- [`../docs/soroban-cli-quickref.md`](../docs/soroban-cli-quickref.md): contract invocation shapes
