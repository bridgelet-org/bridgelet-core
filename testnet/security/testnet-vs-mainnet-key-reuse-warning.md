# ⚠️ Never Reuse a Mainnet-Intended Key on Testnet

> **Use a separate, throwaway keypair for every testnet experiment.** Any key
> you might ever want to hold mainnet funds or authority must never touch
> testnet, in any role.

## Why this matters even though testnet has no real value

Stellar uses the same Ed25519 keypair format on every network. The same secret
produces the **same public address** on testnet and mainnet. If a key is exposed
while you test, that exposure is real for mainnet too.

Testnet workflows routinely expose secrets:

- keys pasted into shell history, `.env` files, CI variables or screenshots;
- identities saved with `stellar keys` on shared or throwaway machines;
- sample configs committed to a repository or sent in a bug report;
- keys shared with a teammate or relayer so they can "just try it".

An attacker who obtains a testnet-exposed key that is also your planned mainnet
key can drain any mainnet account that key controls or impersonate that signer
the day it goes live.

## Keys in this project that this applies to

| Key | Role | Rule |
|---|---|---|
| Account creator / deployer identity | Pays fees, deploys, `creator` of `SweepController` | Testnet-only identity |
| `authorized_signer` (Ed25519 key behind `SweepController`) | Signs sweep authorizations | A **separate** keypair per network. The mainnet signer must never be generated or used for testnet testing |
| Admin / upgrade-admin / `reserve_admin` addresses | Admin actions (`upgrade`, `set_base_reserve`) | Different addresses per network (`testnet/security/admin-key-hygiene.md`) |
| Relayer | Submits `claim()` and pays fees | Testnet-only funded account |
| Recipient/demo keys | Sign auth entries in examples | Throwaway |

## Rules

1. Generate testnet keys **for testnet** (`stellar keys generate <name> --network testnet`)
   and name them clearly, e.g. `testnet-deployer`.
2. Never import a mainnet secret, seed phrase or hardware-wallet account into a
   testnet identity or CI secret.
3. Never put a secret key or signing seed in the repository, an issue, a log, a
   URL, a screenshot, or a chat message. Public keys and contract IDs are fine.
4. Do not reuse a key because it "already has a trustline" or "is already funded".
5. Keep mainnet keys on separate machines/profiles from testnet tooling.
6. If you **may** have used a planned mainnet key on testnet, treat it as
   compromised: generate a new mainnet key and do not use the old one.

## Quick self-check before you start

- [ ] Is this key one I would ever fund on mainnet? If yes, stop and generate a new one.
- [ ] Is the key stored only in a testnet-labelled identity?
- [ ] Is it absent from `deployments/*.json`, `.env`, and docs (only the **public** key belongs in
  `deployments/testnet.json`)?

## Related

- `testnet/security/admin-key-hygiene.md`
- `testnet/security/disclosure-process.md`
- `testnet/security/testnet-threat-model.md`
