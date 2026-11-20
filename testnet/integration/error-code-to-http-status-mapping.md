# Suggested HTTP Status Mapping for Contract Errors

For teams that wrap these contract calls behind their own REST API. This is a
**starting convention** so integrations don't each invent an inconsistent
mapping. It is a recommendation, not something the contracts or any SDK
enforce, and you may choose different codes.

> Derived from `contracts/ephemeral_account/src/errors.rs` and
> `contracts/sweep_controller/src/errors.rs`. For what each error means and what
> to do about it, see `testnet/examples/error-handling.md` (the authoritative
> handling guide); this page adds only the HTTP layer. Not verified against a live
> deployment.

## Principles

- **Return a stable machine-readable `code`** in the body (e.g. `ACCOUNT_EXPIRED`)
  in addition to the HTTP status, and keep the raw numeric contract error as a
  field for debugging. Do not make clients parse messages.
- **4xx means the request cannot succeed as sent**; **5xx / 503 means retry may
  help**. Contract errors are mostly 4xx.
- Distinguish **contract errors** (a `Error(Contract, #N)` outcome) from
  **infrastructure failures** (RPC down, timeouts, bad passphrase), which map to
  5xx and never to the contract-error rows.
- Remember the contract numbers collide between contracts (for example `1`,
  `3`, `6`, `7`): always map by **(contract, number)**.

## `EphemeralAccount` errors

| # | Error | Meaning (short) | HTTP | Body `code` |
|---|---|---|---|---|
| 1 | `AlreadyInitialized` | `initialize` called twice | 409 Conflict | `ALREADY_INITIALIZED` |
| 2 | `NotInitialized` | Account has not been initialized | 404 Not Found (or 409 if you created it) | `ACCOUNT_NOT_INITIALIZED` |
| 3 | `PaymentAlreadyReceived` | Payment already recorded where only one was allowed | 409 Conflict | `PAYMENT_ALREADY_RECEIVED` |
| 4 | `InvalidAmount` | Amount is not positive | 422 Unprocessable Entity (400 if you treat it as malformed) | `INVALID_AMOUNT` |
| 5 | `InvalidExpiry` | Expiry ledger not valid | 422 | `INVALID_EXPIRY` |
| 6 | `NotExpired` | Expiry/recover attempted early | 409 Conflict (or 425 Too Early) | `ACCOUNT_NOT_EXPIRED` |
| 7 | `AlreadySwept` | Sweep already executed | 409 Conflict | `ALREADY_SWEPT` |
| 8 | `Unauthorized` | Caller not authorized | 403 Forbidden | `UNAUTHORIZED` |
| 9 | `InvalidSignature` | Signature invalid | 403 Forbidden (or 401 if you model signatures as credentials) | `INVALID_SIGNATURE` |
| 10 | `NoPaymentReceived` | Nothing recorded to sweep | 409 Conflict | `NO_PAYMENT_RECEIVED` |
| 11 | `AccountExpired` | Past expiry ledger | **410 Gone** | `ACCOUNT_EXPIRED` |
| 12 | `InvalidStatus` | State does not allow the operation | 409 Conflict | `INVALID_ACCOUNT_STATUS` |
| 13 | `DuplicateAsset` | Asset already has a payment | 409 Conflict | `DUPLICATE_ASSET` |
| 14 | `TooManyPayments` | Payment limit reached | 422 | `TOO_MANY_PAYMENTS` |
| 15 | `NotUpgradeAdmin` | Caller isn't upgrade admin | 403 Forbidden | `NOT_UPGRADE_ADMIN` |

## `SweepController` errors

| # | Error | Meaning (short) | HTTP | Body `code` |
|---|---|---|---|---|
| 1 | `InvalidAccount` | Account not in a valid state/not recognised | 404 Not Found or 409 | `INVALID_ACCOUNT` |
| 2 | `TransferFailed` | Transfer did not complete | 502 Bad Gateway (or 500) | `TRANSFER_FAILED` |
| 3 | `AuthorizationFailed` | Not the creator/already initialized/auth failed | 403 Forbidden | `AUTHORIZATION_FAILED` |
| 4 | `InsufficientBalance` | Not enough balance | 422 (or 402) | `INSUFFICIENT_BALANCE` |
| 5 | `AccountNotReady` | No payment yet / not sweepable | 409 Conflict | `ACCOUNT_NOT_READY` |
| 6 | `AccountExpired` | Account expired | **410 Gone** | `ACCOUNT_EXPIRED` |
| 7 | `AccountAlreadySwept` | Already swept or destination frozen | 409 Conflict | `ALREADY_SWEPT` |
| 8 | `InvalidSignature` | Malformed signature | 400 Bad Request / 403 | `INVALID_SIGNATURE` |
| 9 | `SignatureVerificationFailed` | Ed25519 check failed | 403 Forbidden | `SIGNATURE_VERIFICATION_FAILED` |
| 10 | `AuthorizedSignerNotSet` | Controller not configured | 500 / 503 (operator problem, not the client's) | `CONTROLLER_NOT_CONFIGURED` |
| 11 | `InvalidNonce` | Stale/incorrect nonce | 409 Conflict (client should re-fetch nonce and re-sign) | `INVALID_NONCE` |
| 13 | `UnauthorizedDestination` | Destination differs from the locked one | 403 Forbidden | `UNAUTHORIZED_DESTINATION` |

(There is no `12` in `sweep_controller/src/errors.rs`.)

## Non-contract failures

| Condition | HTTP | Retry? |
|---|---|---|
| Simulation failed for reasons other than a contract error (resource limits) | 502 / 500 | Maybe, after fixing fee/resources |
| Soroban RPC/Horizon unreachable or timeout | 503 Service Unavailable (`Retry-After`) | Yes, with backoff (`testnet/runbooks/rpc-outage.md`) |
| `tx_bad_seq`, `TRY_AGAIN_LATER` | 503 or 409 | Yes, with fresh sequence |
| `tx_bad_auth` (passphrase) | 500 | No; fix config (`testnet/runbooks/wrong-network-passphrase.md`) |
| Contract not found | 404 or 503 | No until fixed (`testnet/runbooks/contract-not-found-error.md`) |
| Rate limit on your own API | 429 | Yes |

## Idempotency note

For `ALREADY_SWEPT` and `ALREADY_INITIALIZED`, check on-chain state before
returning an error: the original request may have actually succeeded and the
caller is retrying. See `testnet/examples/idempotency-considerations.md`.

## Using this table

Keep it in one place in your codebase (a lookup keyed by contract and number) and
cover it with a test that every error variant has a mapping, so a new contract
error cannot fall through to a generic 500 unnoticed.
