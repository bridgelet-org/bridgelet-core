<!--
Purpose: Explain why EphemeralAccount::sweep accepts an auth_signature parameter that is not verified by the account contract.
Owner: @bridgelet-org
Status: Documentation-only. No contracts, docs, scripts, or tools changes are introduced by this file.
-->

# FAQ: Why Does `sweep()` Accept an `auth_signature` Parameter It Does Not Verify?

| Field | Value |
| :--- | :--- |
| **Category** | Frequently Asked Questions (FAQ) |
| **Related issue** | [#334](https://github.com/bridgelet-org/bridgelet-core/issues/334) |
| **Owner / reviewer** | `@bridgelet-org` |
| **Last reviewed** | `2026-08-26` |

## Quick Answer

In the MVP, `EphemeralAccount::sweep()` keeps the `auth_signature` parameter as a
placeholder for the intended signed-sweep interface. The account contract does not
verify the Ed25519 signature bytes itself. Its authorization check only confirms that
the configured `authorized_controller` authorized the call.

The real Ed25519 verification and replay-protection nonce handling happen in
`SweepController::execute_sweep`. Keeping that work in the controller lets the
controller own the signing and authorization policy while the ephemeral account
handles its lifecycle and funds.

For the full background and trade-offs, see
[`sweep-auth-signature-dead-parameter.md`](../postmortems/sweep-auth-signature-dead-parameter.md).

## What This Means in Practice

Do not treat the `auth_signature` argument on a direct `EphemeralAccount::sweep()`
call as proof that the signature was checked. A direct call bypasses the controller's
Ed25519 verification and nonce checks.

Always route signed sweeps through `SweepController::execute_sweep`. When using the
Soroban authorization path, route claims through `SweepController::claim` instead.
Never call `EphemeralAccount::sweep()` directly from an integration or relayer.

## Related Documents

- [`docs/security.md`](../../docs/security.md) — MVP limitation and integrator best practice.
- [`docs/architecture.md`](../../docs/architecture.md) — separation of responsibilities between the account and controller.
- [`sweep-auth-signature-dead-parameter.md`](../postmortems/sweep-auth-signature-dead-parameter.md) — full postmortem.

## Related Issues

- [#314](https://github.com/bridgelet-org/bridgelet-core/issues/314) — original issue about the unused `auth_signature` parameter.
