# Testnet Contract Access Model

## Status

> **No Bridgelet contract is currently deployed.** The canonical
> `testnet/registry/contract-status.md` records all four as undeployed with
> two `TBD` and two `N/A` IDs and Last Verified `Never`. This document
> describes the access model **specified by the contract source and by the
> public Stellar testnet**, not an observation of a live deployment. Nothing
> here should be read as evidence that any contract is currently callable.

## The rule

**Anyone can call the deployed testnet contracts.** There is no caller
allowlist, no membership check, and no network-level restriction on who may
invoke them. A funded testnet identity — which Friendbot provides for free —
is enough to submit a transaction to any Bridgelet contract.

| Layer | Restriction on *calling* | Basis |
|---|---|---|
| Stellar testnet | None. Public, read-without-authentication network. | `testnet/policies/data-retention.md` |
| Soroban contract entrypoint | No allowlist in any of the four contracts. | Contract source; no allowlist type or caller list exists |
| `scripts/deploy-testnet.sh` | Deployment is restricted by who holds the deployer key. That is a deployment concern, not a calling concern. | `testnet/policies/reset-policy.md` § Authority |
| Function body | Varies by function — see the next section. | Contract source |

This is a property of the **deployment environment**, not a defect in the
contracts. The same source could be deployed somewhere with a different
access policy; on testnet there is none.

## Permissionless calling is not unauthenticated behaviour

The two ideas are easy to conflate and must be kept separate:

- **Permissionless calling** means anyone may *submit* an invocation.
- **Authorization inside a function** decides whether that invocation
  *succeeds* and what it is allowed to change.

Many functions do check authorization. Some deliberately do not. The
distinction is per function, not per contract:

| Function | Who may call | What still gates success |
|---|---|---|
| `EphemeralAccount::initialize` | Anyone | `creator.require_auth()`; one-time guard; valid expiry |
| `EphemeralAccount::record_payment` | **Anyone** | No `require_auth()`. Any caller can append to any initialized account — see abuse pattern P7 |
| `EphemeralAccount::expire` | **Anyone** | Deliberately permissionless so funds cannot be stranded; one-shot and irreversible — see P8 |
| `EphemeralAccount::upgrade` | Anyone may submit | Per-instance `admin.require_auth()`; no timelock or multi-sig |
| `EphemeralAccount::sweep` | Anyone may submit | `authorized_controller.require_auth()`; its `auth_signature` parameter is **not** cryptographically verified |
| `SweepController::initialize` | Anyone | `creator.require_auth()`; one-time guard; sets signer and optional destination |
| `SweepController::execute_sweep` | Anyone may submit | Real Ed25519 verification over destination + current nonce + controller ID, plus destination validation |
| `SweepController::claim` | Anyone may relay | Recipient signs a Soroban auth entry; relayer pays fees |
| `SweepController::update_authorized_destination` | Anyone may submit | `creator.require_auth()`; blocked once `get_nonce() > 0` — see P4 |
| `ReserveContract::set_base_reserve` | Anyone may submit | `admin.require_auth()` |
| `AccountFactory::initialize` | **Anyone** | No `require_auth()` and no one-time guard — see P1b |
| `AccountFactory::batch_initialize` | Anyone | `creator.require_auth()` per batch |

The practical reading: **"anyone can call" does not mean "anyone can change
anything"**, but it does mean that functions without an internal check can
be changed by anyone. Those cases are catalogued in
[`../security/known-testnet-abuse-patterns.md`](../security/known-testnet-abuse-patterns.md)
and are the first things to suspect when the deployment behaves unexpectedly.

## Practical implications

### One shared nonce space per SweepController instance

A `SweepController` has a **single global nonce**, not a nonce per
`EphemeralAccount`. It starts at `0` at `initialize()` and increments by one
after every successful `execute_sweep()`; `claim()` does not increment it.
The signature digest binds the *current* nonce from storage, so a signature
built over a stale value fails with an opaque trap rather than a named error.

Because calling is permissionless, every integrator using the shared
controller shares that counter with every other caller:

```bash
stellar contract invoke --send no \
  --id "<shared-sweep-controller-id>" --network testnet \
  --source "<funded-reader>" -- get_nonce
```

A high or fast-moving value is normal multi-tenant operation, not evidence of
an attack. Only a party holding the sweep-signing key can advance it with a
valid sweep, but any party can submit a failed attempt. Re-read `get_nonce()`
immediately before signing and serialise sign-and-submit per controller. See
[`../config/nonce-tracking.md`](../config/nonce-tracking.md) and abuse
patterns P2/P3.

A redeploy resets the nonce to `0`; any locally cached value becomes wrong.

### Shared demo accounts are sweepable by anyone with the right auth

The long-lived accounts recorded in
[`../registry/known-test-accounts.md`](../registry/known-test-accounts.md)
are documentation fixtures, not private accounts. Any party that can satisfy
their authorization can sweep them:

- where the account's `authorized_controller` is the shared
  `SweepController`, a party holding the configured Ed25519 signing key can
  produce a valid signature over the current nonce, destination, and
  controller ID and call `execute_sweep` on **any** account wired to that
  controller;
- where an account was initialized with a key as its `authorized_controller`,
  whoever holds that key can satisfy `require_auth()` directly;
- two related state changes need no authorization at all: `record_payment()`
  can append a payment to anyone's account, and `expire()` can permanently
  foreclose a sweep once the expiry ledger has passed.

So a "known" account is known to everyone. Do not use one as storage for
anything you would miss, and do not assume it will still be unswept when a
walkthrough is run. The maintenance rule in that registry — never sweep these
accounts — is a convention, not an enforced restriction.

### Controller configuration and destination are public

The shared deployment's parameters are public information:

- `deployments/testnet.json` publishes `authorizedSigner`, `creatorAddress`,
  `reserveAdminAddress`, `recoveryAddress`, and the WASM hash in its `config`
  object;
- the deploy initializes the controller with `authorized_destination = null`
  (unlocked mode), as recorded in
  [`../docs/faq.md`](../docs/faq.md);
- there are no direct getters for the creator, signer, or destination, so
  their values are inferred from the `initialize` transaction and from the
  `dest_auth` / `dest_upd` events in
  [`../registry/event-topics.md`](../registry/event-topics.md);
- events have no access control. Anyone can read every topic and payload, so
  a destination change is observable by everyone even though only the
  creator can make it.

Repointing the destination is **not** permissionless: it requires
`creator.require_auth()` and closes permanently after the first successful
sweep. Reading it is. Do not treat the shared destination as a secret or as a
stable property of your integration; re-check it before a run.

### No isolation between integrators

The deployment is multi-tenant. One party's actions degrade the experience
of every other party, often silently:

| Action by another party | Effect on your run |
|---|---|
| A successful sweep through the shared controller | Your cached nonce is stale; your signature traps |
| `record_payment()` on your account | Unexpected payment in `get_info()`; possible sweep failure |
| `expire()` on your account after expiry | Permanent `Expired` status; your sweep is foreclosed |
| An in-place `upgrade()` of a shared instance | Behaviour changes with the same ID and no event |
| A large factory batch | Index-slot collisions and `error: None` failures |

These are documented as abuse patterns, but most are ordinary operation on a
shared, permissionless deployment. A test that assumes a private controller,
a private account, or a stable nonce is testing an environment that does not
exist. See [`../security/README.md`](../security/README.md) § Why testnet
needs its own security tracking.

## Integrator guidance

Design your testing around the lack of isolation:

- [ ] Resolve contract IDs at runtime from config; never hard-code them.
- [ ] Re-verify IDs with
      `testnet/registry/verify-contract-live.md` before a run that matters.
- [ ] Deploy your own `EphemeralAccount` instance per run rather than reusing
      a shared one. A deployment ID and a child account are different things.
- [ ] If you need nonce isolation, deploy your own `SweepController` rather
      than sharing one; there is no per-account nonce.
- [ ] Fetch `get_nonce()` immediately before signing and serialise
      submissions. Treat an opaque signature trap as contention first.
- [ ] Use synthetic identities and amounts only. No real keys, real people,
      or real customer data — see
      [`../policies/data-retention.md`](../policies/data-retention.md).
- [ ] Do not rely on the shared destination, signer, or demo accounts being
      unchanged between runs; re-read the observable state.
- [ ] Make runs idempotent and re-creatable; expect to start from zero after
      a reset or a redeploy.
- [ ] Never mistake permissionless calling for permission to bypass
      `require_auth()` or Ed25519 verification.

## What does not follow

Permissionless calling does not make the contracts defenceless. The
authorization checks inside functions still hold:

| Attempted | Result | Why |
|---|---|---|
| Forge a sweep signature | Fails | Real `ed25519_verify` in `SweepController` |
| Replay a spent signature | Fails | Digest uses the current on-chain nonce |
| Redirect a captured sweep | Fails | Destination is bound into the digest |
| Sweep an account whose controller is a different contract | Fails | `authorized_controller.require_auth()` |
| Re-initialise an existing account | Fails | One-time guard |
| Change `expiry_ledger` or `recovery_address` after init | Impossible | No setters exist |
| Repoint the destination after the first sweep | Fails | `nonce > 0` check |
| Remove an injected payment | Impossible | No delete path — which is why P7 is permanent |

The full list, including the two "looks like a hole but is not" cases, is in
[`../security/known-testnet-abuse-patterns.md`](../security/known-testnet-abuse-patterns.md).
One caveat worth carrying forward: `EphemeralAccount::sweep()` currently
accepts an unchecked signature. It is not exploitable for theft because that
contract moves no tokens, but any future change that adds transfers before
adding verification would turn the same call into a drain. Treat edits to it
as security changes.

## Related Documentation

- [`../policies/data-retention.md`](../policies/data-retention.md) — why the
  deployment is public and what must never be written to it
- [`../security/README.md`](../security/README.md) — the shared-deployment
  model and the public-information-only rule
- [`../security/known-testnet-abuse-patterns.md`](../security/known-testnet-abuse-patterns.md)
  — the P1b/P2/P4/P7/P8 mechanisms cited above
- [`../config/nonce-tracking.md`](../config/nonce-tracking.md) — the nonce
  mechanics behind the shared counter
- [`../registry/known-test-accounts.md`](../registry/known-test-accounts.md)
  — the shared demo accounts and their maintenance convention
- [`../registry/admin-addresses.md`](../registry/admin-addresses.md) — the
  public privileged roles, and how destination values are inferred
- [`../registry/event-topics.md`](../registry/event-topics.md) — `dest_auth`
  and `dest_upd`, the observable destination changes
- [`../examples/README.md`](../examples/README.md) — the posture of treating
  snippets as integration guides against a shared deployment
- [`../policies/reset-policy.md`](../policies/reset-policy.md) — what a
  redeploy destroys, including the nonce and shared accounts
- [`versioning-scheme.md`](versioning-scheme.md) — the proposed deployment
  tag; it does not affect who may call

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-11-18 | Initial access-model policy: permissionless calling, shared nonce, public config, no isolation |
