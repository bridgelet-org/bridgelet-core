# Etiquette for the Shared Demo Accounts

`testnet/registry/known-test-accounts.md` describes a few long-lived,
intentionally non-swept `EphemeralAccount` instances used for demos,
screenshots and integration fixtures. They are a **shared, mutable resource**.
This page sets expectations so people don't break each other's demos.

> Status: the registry currently lists these accounts as pending deployment, so
> this etiquette applies once they exist. It is a proposed convention, not an
> enforced rule.

## Principles

1. **Do not sweep, expire, or recover a demo account you did not create or were
   not told to use.** Sweeping is irreversible: an account reaches `Swept (2)`
   once and is done.
2. **Read freely, mutate rarely.** Read-only calls (`get_status`, `get_info`,
   `can_sweep`) are always fine. State-changing calls are not.
3. **Use your own accounts for experiments.** Deploy a fresh `EphemeralAccount`
   (see `testnet/examples/`) instead of "borrowing" a demo account.
4. **Don't drain shared balances.** Do not use the demo/relayer funding for
   unrelated tests, and do not fund-and-drain to exercise rate limits.
5. **Don't change shared configuration** (authorized destination, controller
   settings) on shared controllers; the creator can freeze or change the
   destination (`testnet/examples/update-authorized-destination-flow.md`).

## Reserving a demo account

Before a presentation or recorded demo that needs a specific account:

1. Add a row to the reservations list in the PR/issue or the channel named in
   `testnet/docs/contact-and-ownership.md`: account, owner, start and end (UTC),
   what you will do (read-only, or sweep it).
2. Others must not mutate that account during the window.
3. After the demo, post that it is released and what state it is in. If you swept
   it, say so and note a replacement in `testnet/registry/known-test-accounts.md`.

## Before someone else's scheduled demo

- Avoid sweeping, expiring or re-initializing anything during the 24 hours before
  a reserved window.
- Do not run load tests against the shared controller or relayer during a
  reservation.
- If you find a demo account unexpectedly changed, don't "repair" it; report it.

## Keeping accounts healthy

- Check `is_expired` and the expiry ledger when you reserve; an account that
  expires mid-demo is the most common embarrassing failure.
- Check the relayer/funding account balance before the demo
  (`testnet/monitoring/balance-thresholds.md`).
- Record funding top-ups (`testnet/faucet/funding-audit-log.md` if present).

## If you break one

Say so. Replace it, update the registry, and note the incident in the weekly
summary. Mistakes on testnet are cheap; unreported ones are expensive.
