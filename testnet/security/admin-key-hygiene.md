# Testnet Admin Key Hygiene

## The premise

"It is only testnet" is not a reason to be careless with the testnet deployer/admin key. The XLM is worthless, but the **authority** is not.

This deployment is **shared**. Other integrators, other projects' CI, and this repo's own documentation examples all depend on the contracts this key controls. A careless or malicious use of the admin key degrades every one of them, and does so silently in the case of `upgrade()`.

> **Scope.** This document sets expectations for **whoever holds the key**. It does not describe, review, or modify the deploy script — secret handling inside `scripts/deploy-testnet.sh` is out of scope here and untouched by this work. What follows is about the human and organisational practices around the key, not the mechanics of the script that consumes it.

---

## What the key can actually do

Enumerating the blast radius first, because "it's just testnet" skips this step and it is the whole point.

### `EphemeralAccount::upgrade(new_wasm_hash)` — the serious one

```rust
pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>) -> Result<(), Error> {
    let admin = storage::get_admin(&env).ok_or(Error::NotUpgradeAdmin)?;
    admin.require_auth();
    env.deployer().update_current_contract_wasm(new_wasm_hash);
    Ok(())
}
```

- **Gated by** `admin`, set per-instance at `initialize()`.
- **Effect** — replaces the contract's code. Immediately and globally, for **every integrator** using that instance. The contract ID does not change, so nothing in a consuming application's address configuration reveals that anything happened.
- **Events** — **none.** No contract event is emitted. `testnet/registry/upgrade-history.md` notes this explicitly: *"No native upgrade event emitted; monitor via transaction history."* Detection therefore depends on someone checking transaction history, which almost nobody does.
- **Reversible?** Only by another `upgrade()` call with a previous hash. There is no timelock, no multi-sig, and no governance delay. In practice, not reversible for anyone who does not notice quickly.
- **Can it be called on any account?** Only on instances whose `admin` is this key. If the key is the admin for many or all testnet accounts, the blast radius is all of them.

This is the single most consequential thing the key can do, and it is the one with the least observability.

### `SweepController::update_authorized_destination(new_destination)` — the sneaky one

```rust
pub fn update_authorized_destination(env: Env, new_destination: Address) -> Result<(), Error> {
    let creator = storage::get_creator(&env).ok_or(Error::AuthorizationFailed)?;
    creator.require_auth();
    let nonce = storage::get_sweep_nonce(&env);
    if nonce > 0 {
        return Err(Error::AccountAlreadySwept);
    }
    // ...
}
```

- **Gated by** the controller's `creator`, not its admin — a different key.
- **Effect** — changes where **every account on that controller** can be swept to. In locked mode, sweeps to any other address then fail with `UnauthorizedDestination`.
- **One-shot window** — permitted only while the nonce is `0`, i.e. **before the first successful sweep or claim on that controller**. Once any sweep has happened it fails with `AccountAlreadySwept` and the destination is permanently fixed.
- **Events** — `DestinationUpdated { old_destination, new_destination }` (topic `dest_upd`) is emitted, so this one *is* observable if anyone is watching.

**This is worth emphasising:** the entire window in which a destination repoint is possible is the window before the first sweep. Early in a deployment's life that is exactly when a stray or careless `initialize()`-adjacent script run, a wrong flag, or a hostile actor has the most opportunity, and the least suspicion.

### `ReserveContract::set_base_reserve(amount)` — low impact today

- **Gated by** `admin`, and bounded to `MAX_RESERVE_STROOPS = 100_000_000_000` (10,000 XLM).
- **Currently inert.** `EphemeralAccount` uses its own hardcoded `BASE_RESERVE_STROOPS = 1_000_000_000` and never reads `ReserveContract`. Changing this value changes nothing observable until the two are wired together — which is an open question the README flags. See `testnet/registry/reserve-contract-status.md`.

Low impact **today**. If the wiring is ever implemented, this becomes a live config knob and the hygiene requirements below apply to it unchanged.

### `AccountFactory::initialize(ephemeral_account_wasm_hash)` — **not gated at all**

```rust
pub fn initialize(env: Env, ephemeral_account_wasm_hash: BytesN<32>) {
    env.storage().instance().set(&DataKey::EphemeralAccountWasmHash, &ephemeral_account_wasm_hash);
}
```

There is **no `require_auth()`** and no one-time guard. **Anyone** can call this and overwrite the WASM hash that every subsequent `batch_initialize` deploys. No admin key is involved; no key at all is required.

This is not a key-hygiene problem — it is a missing-authorisation problem in the contract, and it is tracked in the abuse patterns document. It is listed here because it is the one privileged-sounding entry point that key discipline cannot protect. **Holding the admin key well does nothing about it.**

### Everything else is not privileged

| Operation | Gated by | Note |
|---|---|---|
| `SweepController::initialize` | `creator` | One-time; second call returns `AuthorizationFailed` |
| `EphemeralAccount::initialize` | `creator` | One-time; sets `admin` and `authorized_controller` — get these right, they are immutable |
| `EphemeralAccount::expire` | **nobody** | Deliberately permissionless so funds cannot be stranded |
| `EphemeralAccount::recover` | `creator` or `recovery_address` | |
| `EphemeralAccount::record_payment` | **nobody** | No status or expiry guard — see the abuse patterns doc |
| `SweepController::execute_sweep` / `claim` | Ed25519 signer or recipient auth | |

`expire()` being permissionless is intentional, not a gap. Do not "fix" it.

---

## Expected practices

### 1. One key, one named holder, one documented purpose

- The key should have a **single named owner** recorded somewhere findable. `testnet/registry/admin-addresses.md` is the place for the public address; the *name* of the human or team holding the secret belongs in whatever the project's access records are.
- Its scope is **the shared testnet deployment, and nothing else.** A key used for mainnet, for another environment, or for unrelated tooling should never be the same key.
- If more than one person needs the capability, that is a process problem to solve with access control and audit, not with key sharing.

### 2. Never in git, ever, in any form

- Not in `.env` (check `.gitignore` covers it), not in a shell history file, not in a CI log, not in a config committed to a fork, not in a documentation example.
- `.env.example` holds **placeholders only**. The repo's `testnet/config/README.md` sets the standard this directory follows: *"No secrets, ever... Never put a secret key (`S...`), seed phrase, or any private signing material in this directory or in files derived from these templates."* That standard applies with more force to the admin key than to anything else, precisely because it is only "just" testnet.
- If a key has ever been committed, pushed, or pasted anywhere: **rotate before doing anything else.** Removing it from history does not un-disclose it. Treat it as public from the moment it lands.
- Fork PRs are not private. A secret in a PR body, a diff, or a test fixture is published.

### 3. Generate for this purpose only

- Derive or generate a **dedicated** testnet keypair. Do not reuse a key from a personal wallet, a local sandbox, a different project, or any environment where the same key might be expected to hold value.
- The deployment/admin key is an `S...` Stellar secret that signs transactions and pays fees. The sweep-authorization key is a **separate** Ed25519 seed, and it is signing-only — it does not need to be a funded account. Conflating the two means a leak of the cheap key compromises fee-paying authority, and a leak of the deployer key exposes a funded account. `tools/sweep-signer` exists to keep the signing seed separate.
- Both are secrets. Neither is documented anywhere in this repository, and neither should be.

### 4. Store it as a secret, not as a file

- CI secret store (the repo's workflows already expect a `TESTNET_DEPLOYER_SECRET_KEY` secret) or a password manager. Not a file in a working tree, not a shell variable in a persistent profile, not a `.env` on a shared machine.
- Scope access to the minimum: read it only when a deploy or upgrade is actually happening.
- Access should be auditable — "who read the key, when" should be answerable.

### 5. Upgrade deliberately, or not at all

Given that `upgrade()` emits no event and changes behaviour for everyone:

- **Require an intentional act.** An `upgrade()` should never be a side effect of running a build or deploy script unattended. If the key is loaded in a CI environment, that environment must not be able to call `upgrade()` without a deliberate, separately-authorised step.
- **Record every upgrade in `testnet/registry/upgrade-history.md` immediately**, with date, old hash, new hash, reason, admin address, and transaction hash. The table exists for exactly this.
- **Announce it before doing it**, with enough notice for integrators to react. Note the tension with the other direction: the key should also not be *too* protected to use when a genuine fix is needed. The resolution is that an upgrade is a **deliberate, announced, logged** act — not a casual one, and not an unattended one.
- **Verify after.** Follow `testnet/registry/verify-contract-live.md` rather than assuming the call did what was intended.
- **Behaviour changes are indistinguishable from bugs** if nobody knows an upgrade happened. A wave of "the contracts changed and nobody told me" reports is the predictable outcome of an unlogged upgrade.

### 6. Have a rotation and revocation plan

- Know in advance how the key would be rotated, and who decides. For `EphemeralAccount` there is **no rotation function** — `admin` is fixed at `initialize()` and has no setter. The only path is deploying a new instance and migrating, which changes the contract ID. Plan for that being disruptive.
- For the controller, `authorized_signer` also has **no getter and no rotation function**. A compromised sweep-signing key cannot be rotated on-chain at all; the recovery is a new controller deployment.
- This asymmetry is a reason to treat these keys as long-lived and worth protecting, rather than as disposable testnet artefacts. There is no cheap revocation path for either.

### 7. Review before use

- Any use of the admin key should be able to answer: *what does this call change, who does it affect, how would anyone notice, and how would it be reversed?* If any of those is unclear, do not run it.
- Cross-check privileged calls against `testnet/security/known-testnet-abuse-patterns.md` — the same operations, described from the perspective of someone doing them to cause harm.

### 8. Do not use it against the shared deployment casually

- `expire()` is permissionless and **anyone** can trigger recovery on any expired account. Not an admin-key concern, but worth knowing: holding the admin key is not what protects an account from being expired, and not holding it is not what allows it.
- Do not call `upgrade()` "just to test it." The same function is used for genuine fixes, and its history is the only record that they happened.
- Do not repoint `authorized_destination` on a controller other integrators are using. If the controller is in locked mode, this silently breaks every other integrator's sweeps with `UnauthorizedDestination`.

---

## If the key is exposed

Treat as compromised, in this order:

1. **Stop using it immediately.** Do not "clean up first."
2. **Rotate.** New keypair, new deployment. Remember there is no in-place rotation for `admin` or `authorized_signer`.
3. **Reconstruct what was done with it, from transaction history** — not from assumptions. This is the only record, because `upgrade()` emits no event. Check for `upgrade` invocations and `dest_upd` / `dest_auth` events.
4. **Record the incident** in `testnet/registry/upgrade-history.md` with the full transaction history, however unflattering. An unlogged change is worse than a logged mistake.
5. **Tell integrators** if anything on the shared deployment changed. A silent upgrade discovered by someone else is a credibility problem well beyond the technical one.
6. **Write down how it leaked**, and fix that path. The recurring causes are a committed `.env`, a secret pasted into a PR or an issue, a shell history file, and a CI log at an unprotected log level.

---

## Quick reference

| Rule | Why |
|---|---|
| Single named holder | Accountability; the blast radius is everyone |
| Never in git, in any form | Forks and PRs are public; rotation is the only remedy |
| Dedicated key, not reused | Limits blast radius across environments |
| Secret store, not a file | Auditable, revocable, access-controlled |
| `upgrade()` is deliberate and logged | No event is emitted; the log is the only record |
| Rotation plan exists now | No on-chain rotation for `admin` or `authorized_signer` |
| Recovery key custody is a separate concern | `AccountFactory::initialize` is not gated by any key |

---

## Related documentation

- `testnet/security/README.md` — index for this directory
- `testnet/security/known-testnet-abuse-patterns.md` — the same operations, from an attacker's perspective
- `testnet/security/unverified-signature-path-warning.md` — the other place testnet is mistaken for safer than it is
- `testnet/registry/admin-addresses.md` — public addresses per privileged role
- `testnet/registry/upgrade-history.md` — where every upgrade must be recorded
- `testnet/registry/verify-contract-live.md` — post-change verification
- `testnet/config/README.md` — the no-secrets standard this follows
- `docs/security.md` — design-level security model (different concern)
- `scripts/deploy-testnet.sh` — consumes the key; **out of scope, not modified**
- `tools/sweep-signer/` — the separate, signing-only sweep key

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-26 | Initial testnet admin key hygiene expectations |
