# Soroban Authorization Framework Review — EphemeralAccount

Closes #547.

## Current model: standard-account-based, not custom account contracts

A repo-wide search for `__check_auth` / `CustomAccountInterface` across
all 25 `contracts/*/src/lib.rs` files returns **zero matches**. Every
contract relies exclusively on standard Soroban auth primitives:

- `Address::require_auth()` — `creator` in `initialize()`, `controller`
  in `record_payment()`/`verify_sweep_authorization()`, `admin` in
  `upgrade()`, `caller` in `recover()`.
- `env.authorize_as_current_contract(auth_entries)` with
  `InvokerContractAuthEntry::Contract` —
  `sweep_controller/src/lib.rs::authorize_ephemeral_sweep()` /
  `authorize_ephemeral_sweep_claim()`, letting `SweepController`
  authorize the cross-contract call into `EphemeralAccountContract`.
- A parallel **off-chain Ed25519 path**
  (`verify_sweep_authorization()`, `ephemeral_account/src/lib.rs:567`):
  a raw `BytesN<32>` public key (`authorized_signer`) verified via
  `env.crypto().ed25519_verify()` — the claim/recovery signing model, a
  hand-rolled check, not `__check_auth`.

## Is a custom account contract pattern relevant here?

Not currently. `EphemeralAccountContract` is never a transaction
*source* — it is only ever a *target* of `SweepController`'s authorized
calls, or a validator of a detached Ed25519 signature. `__check_auth`
exists for a contract standing in as a classic-account *signer*, which
isn't this contract's role.

## Forward-looking tradeoffs (decision record)

If a future design wants in-contract multi-signer claim policies
(replacing the single stored `authorized_signer`): **pros** — native
integration with Soroban's auth-entry replay protection, replacing the
hand-rolled Ed25519-over-SHA256 construction; **cons** — `__check_auth`
is a known attack surface if the signed payload isn't scoped correctly
(this repo already defends against replay via `nonce` + `contract_id`
in the message), plus added complexity for a narrow, well-tested flow
with no reported issues.

## Acceptance criteria status

- [x] Current model documented: standard-account-based plus a
      hand-rolled Ed25519 path, not custom-account-contract-based.
- [x] Tradeoffs of a future move documented above.
- [x] No implementation change — current model sufficient; closing as a
      documented evaluation per the issue's own criteria.
