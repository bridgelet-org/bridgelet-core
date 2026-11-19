# Idempotency Keys for Your Application Around These Contracts

A recommendation for how a **consuming application** should handle its own
idempotency keys when calling the contracts. It complements, and is distinct
from, the contracts' own protection: `SweepController` uses a **nonce** to
prevent replay of signed sweeps (`testnet/examples/idempotency-considerations.md`,
`testnet/config/nonce-tracking.md`). Account creation and payment recording have
no such on-chain replay protection, so the application must supply it.

> Recommendation only; not verified against a live deployment. Behaviour is read
> from `contracts/ephemeral_account/src/lib.rs`.

## What the chain gives you, and what it does not

| Operation | On-chain protection | Retry hazard |
|---|---|---|
| `SweepController::execute_sweep` | Nonce, plus the account moves to `Swept`; a second attempt fails | Low, but you must re-read state before re-signing (nonce changes) |
| `SweepController::claim` | Account state (`AlreadySwept`), recipient auth | Low |
| `EphemeralAccount::initialize` | `AlreadyInitialized (1)` on a second call to the same instance | A retried *deployment* creates a **new** contract ID, so it is **not** idempotent |
| Deploying an account (new instance) | None | Duplicate deployment on retry |
| `record_payment(amount, asset)` | `DuplicateAsset (13)` for the same asset; `TooManyPayments (14)` | A retry for a different amount/asset would be recorded as a second payment |
| Classic Horizon payment into the account address | None; each submission is a new payment (sequence number only prevents resubmitting the same envelope) | Double payment on blind retry |

## Recommended pattern

1. **Generate an idempotency key per business operation**, not per HTTP request:
   e.g. `create-account:<order-id>`, `record-payment:<order-id>:<asset>`.
   Use a UUID/ULID or a deterministic hash of the business inputs.
2. **Store before you send.** Persist `{key, operation, request-hash, status:
   PENDING, created_at}` in your database **before** building the transaction.
   A unique constraint on `key` makes concurrent duplicates fail fast.
3. **Record the transaction hash** (and, for deployments, the resulting contract
   ID) as soon as you have it, still `PENDING`.
4. **On retry or crash recovery, reconcile before resending:**
   - If a hash is stored, look it up (`getTransaction`). If it succeeded, mark
     `DONE` and return the stored result; if it definitively failed, you may
     retry with a new sequence number; if not found, wait out its time bounds
     (`timeBounds`/`maxTime`) before assuming it will never land.
   - If no hash is stored, check on-chain state for evidence the operation
     already happened (below) before sending.
5. **Return the stored result for a repeated key**, and reject (409) a repeated
   key with a **different** request hash.
6. **Set transaction time bounds** so an abandoned attempt becomes
   un-landable, making the "did it land?" question finite.
7. **Keep keys for at least as long as clients can retry** (and beyond the
   transaction time bounds).

## Per-operation reconciliation

| Operation | Evidence it already happened |
|---|---|
| Account creation | Your stored contract ID from a prior attempt; `get_info` succeeds on it. Because redeploying yields a new ID, **bind the key to the stored ID** and never deploy twice for one key |
| Initialize | `get_info` returns data (otherwise `NotInitialized (2)`); `created` event for that contract |
| Record payment | `get_info.payments` contains that asset with that amount; `payment`/`multi_pay` event |
| Classic payment (Horizon) | The transaction hash exists on Horizon; the account's recorded payment/balance changed |
| Sweep / claim | `get_status == Swept (2)`; `swept_mul` and `sweep` events; controller nonce |

## Anti-patterns

- Using the HTTP request ID or a timestamp as the key (it changes on retry).
- Retrying deployment on timeout without checking whether the first one landed.
- Treating `DuplicateAsset` or `AlreadyInitialized` as a failure to alert on; in a
  retry path they often mean "already done"; verify the existing state matches the
  intended one before reporting success.
- Reusing a key across different intents.

## Testnet caveat

A testnet reset destroys the on-chain evidence your reconciliation relies on
(`testnet/policies/reset-policy.md`); expect keys to refer to contracts that no
longer exist and design the "stored ID no longer resolves" path explicitly
(`testnet/runbooks/contract-not-found-error.md`).
