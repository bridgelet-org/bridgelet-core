# Common Mistake: Treating the `authorized_controller` Gate as Sweep Authorization

> **The `require_auth()` on `authorized_controller` in `EphemeralAccount::sweep()`
> controls *who may call* the function. It does not validate *what was
> authorized*.** Carrying the opposite assumption from testnet experiments into a
> mainnet integration design is one of the most consequential mistakes you can
> make with this codebase.

## What the code actually does

`contracts/ephemeral_account/src/lib.rs`, `sweep(env, destination, auth_signature)`:

1. checks state: initialized, not already `Swept`, payment recorded, not expired;
2. calls `verify_sweep_authorization(&env, &destination, &auth_signature)`, which
   ignores the signature argument and instead requires the stored
   `authorized_controller` address to authorize the call (`authorized_controller.require_auth()`);
3. marks the account swept, records `destination`, emits `swept_mul`, reclaims reserve.

So the only thing enforced is: **the transaction must be authorized by the
address set as `authorized_controller` at `initialize()`.** Nothing checks:

- that an off-chain party approved this particular `destination`;
- that the `auth_signature` bytes are valid, or even non-garbage;
- any nonce or replay protection (that lives in `SweepController`);
- any binding between the signature and the destination.

## The confusion

| Assumption | Reality |
|---|---|
| "`require_auth()` on the controller means the sweep was authorized by my backend's signature" | It means the controller address signed/authorized the call. If that address is a regular account you control, **anyone holding its key** can sweep to **any** destination |
| "A sweep to a wrong destination would be rejected" | Not by `EphemeralAccount`. Destination validation exists in `SweepController` locked mode only |
| "Passing a bad `auth_signature` fails" | It does not fail (see `testnet/security/unverified-signature-path-warning.md`) |
| "The controller is always the `SweepController` contract" | Only if you initialized it that way. The address is whatever you passed to `initialize()` |

## Where real authorization lives

`SweepController::execute_sweep` performs Ed25519 verification over the hash of
`destination + nonce + contract_id` against `authorized_signer`, with a nonce
that prevents replay, and (optionally) enforces a single authorized destination.
`SweepController::claim` requires the **recipient** to authorize the call
(`recipient.require_auth()`). In both, the controller then authorizes the
sub-call into `EphemeralAccount`.

## What to do instead

- **Initialize ephemeral accounts with the `SweepController` contract address as
  `authorized_controller`**, so the only way to reach `sweep()` is through the
  controller's checks.
- Always sweep via `execute_sweep` or `claim`; never expose a direct
  `EphemeralAccount::sweep` call in a product flow.
- Use locked mode when the destination is known up front
  (`testnet/examples/testing-locked-mode.md`).
- Treat the controller address and `authorized_signer` as production secrets and
  rotate/guard them (`testnet/security/admin-key-hygiene.md`).
- In reviews, ask: "if an attacker had the `authorized_controller` key, what could
  they do?" The answer for a direct `sweep()` is: sweep any funded account to any
  address.

## Testnet experiment to see it for yourself

Initialize a throwaway account with a **plain identity** as `authorized_controller`,
record a payment, then call `sweep` with an arbitrary 64-byte value as
`auth_signature`. It will succeed. That success demonstrates the gate, not
signature validation. Do this only on testnet with throwaway keys
(`testnet/security/testnet-vs-mainnet-key-reuse-warning.md`).
