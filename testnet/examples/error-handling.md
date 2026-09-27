# Worked Example: Mapping Contract Errors to Integrator Handling

## Overview

A raw Soroban contract error reaches a client as a bare integer wrapped in a
host-error string. This document takes each `Error` variant in
`contracts/*/src/errors.rs`, states the condition that fires it, and — the part
that matters — **what an integrator should do about it**: retry, re-fetch state
and re-sign, stop, or escalate.

> **This is the handling companion to
> [`docs/api-reference.md`](../../docs/api-reference.md), not a second source
> of truth.** That document owns the per-contract `### Error Codes` tables
> (`EphemeralAccount` around line 369, `SweepController` around line 557). It
> is cross-referenced here, not edited and not duplicated as authority. Where
> this document says something the API reference does not — reachability of a
> variant, the missing `#[repr(u32)]`, what the code does *not* do — it is
> because it was read out of the source.

## Status

**Derived from source; unverified against a live deployment.** All four
contracts are recorded as undeployed in
`testnet/registry/contract-status.md` (dated 2026-09-24, `Last Verified:
Never`). Which variants *fire* has been determined by reading the contract
bodies, not by observing failures. Treat the "what to do" column as a reasoned
starting point and adjust it once you can watch a live deployment.

---

## How a contract error actually reaches your client

When a contract function returns `Err(e)`, the host surfaces it as a contract
error carrying the `#[contracterror]` discriminant. The rendering this repo
relies on is visible in the contract tests themselves, which assert on the
string directly:

```rust
// contracts/ephemeral_account/src/test.rs
#[should_panic(expected = "Error(Contract, #1)")]   // AlreadyInitialized
#[should_panic(expected = "Error(Contract, #11)")]  // AccountExpired
#[should_panic(expected = "Error(Contract, #13)")]  // DuplicateAsset
```

```rust
// contracts/reserve_contract/src/test.rs
#[should_panic(expected = "Error(Contract, #4)")]   // AlreadyInitialized
#[should_panic(expected = "Error(Contract, #5)")]   // NotInitialized
```

So a client sees something of the shape `Error(Contract, #7)` — typically
embedded in a larger `HostError:` string from the CLI or the RPC result, not as
a bare integer. Two properties follow, and both are load-bearing:

1. **The integer is scoped to the contract that produced it.** `2` means
   `NotInitialized` from `EphemeralAccount`, `TransferFailed` from
   `SweepController`, and `ReserveNotSet` from `ReserveContract`. A decoder
   that receives only an integer **cannot** interpret it without also knowing
   which contract was invoked. This is the single most important reason the
   error-code question below is not a formality.
2. **Some failures are not contract errors at all** and carry no `#[contracterror]`
   discriminant. See
   [Errors that are not contract errors](#errors-that-are-not-contract-errors).

A related in-band case: `EphemeralAccount::simulate_sweep` does **not** return
`Err`. It is

```rust
pub fn simulate_sweep(env: Env, destination: Address) -> (Vec<Payment>, u32)
```

and signals failure by putting the discriminant in the second tuple element
(`0` on success, or `2` / `7` / `10` / `11`). That `u32` is the same
`EphemeralAccount` discriminant space — but it arrives as a *return value*, so
a client that only matches on thrown errors will miss it entirely.

---

## Three things that will burn you

### 1. `UnauthorizedDestination` is `13`. Code `12` does not exist.

`contracts/sweep_controller/src/errors.rs` numbers `InvalidNonce = 11` and then
goes straight to `UnauthorizedDestination = 13`. There is no `12`. The gap is
real in the source and must not be "fixed" by renumbering.

```rust
// contracts/sweep_controller/src/errors.rs — verbatim
InvalidNonce = 11,
UnauthorizedDestination = 13,
```

Consequences:

- A hand-written decoder must map `12` to "unknown", never to the variant
  before or after it.
- A decoder that does `for (i, v) in variants.iter().enumerate() { code = i }`
  will report `UnauthorizedDestination` as `12` and silently mis-map it.
- Code generated from an assumed-contiguous table will be wrong here even if it
  is right everywhere else. Prefer an explicit `match` on the literal
  discriminant.

Note that `EphemeralAccount` *does* have a code `12` — `InvalidStatus`. So the
same integer means `InvalidStatus` in one contract and nothing at all in
another. That is the scoping problem in miniature.

### 2. `SweepController`'s enum has no `#[repr(u32)]`

What is actually in the tree:

| Contract | `errors.rs` | `#[repr(u32)]`? | Derives |
|---|---|---|---|
| `ephemeral_account` | yes | **yes** | `Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord` |
| `reserve_contract` | yes | **yes** | `Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord` |
| `sweep_controller` | yes | **no** | `Copy, Clone, Debug, Eq, PartialEq` |

For the two enums with `#[repr(u32)]`, the explicit discriminants are also
what the Rust type itself guarantees: the discriminant is the wire value, and
`Error::AccountExpired as u32` is `11` by construction. That is a real,
checkable guarantee — the unit tests in
`contracts/ephemeral_account/src/test.rs` assert exactly this
(`assert_eq!(Error::AccountExpired as u32, 11)` and so on).

For `SweepController` the discriminants are written out explicitly, so the
values are unambiguous *as written in the source today*, but the `as u32`
cast is not something the type system is enforcing, and the enum is not
declared with a stable layout. State it that way rather than claiming a
guarantee that is not there: **"the values are 1–11 and 13, read from
`contracts/sweep_controller/src/errors.rs`"**, not "the ABI guarantees 1–11 and
13".

If the enum is ever reordered or a variant inserted, only the `#[repr(u32)]`
enums are protected from silently changing meaning. If the values matter to you
across a redeploy, pin them with a test on your side, or ask for
`#[repr(u32)]` to be added — that is a contract change, outside this
directory's scope.

### 3. The changelog contradicts the source about namespacing

`testnet/docs/changelog.md`'s **Unreleased** section states:

> Error codes are namespaced per contract: EphemeralAccount uses 1000–1999,
> SweepController 2000–2999, and ReserveContract 3000–3999.

**There is no such namespacing in the source on `main`.** The enums are 1–15,
1–11 plus 13, and 1–6 respectively, and there is no offset helper anywhere in
`contracts/shared/` — that crate contains only `types.rs` and its re-exports
(`AccountInfo`, `AccountStatus`, `Payment`, `PaginationParams`, `NO_CURSOR`,
and the factory result types). Grep the tree for a 1000/2000/3000 offset and
you will not find one.

Two further documents repeat the claim in a form that makes it *look*
implemented:

- `testnet/docs/getting-started.md` lists `Error(Contract, #1000)` for
  `AlreadyInitialized`, `#1001` for `NotInitialized`, `#1003` for
  `InvalidAmount`, and so on. Those are the source codes with a flat `+999`
  offset, and they are internally consistent — but they are not what the code
  does.
- The same table maps `#1007` to `Unauthorized`, while the changelog cites
  `#1007` as the network-passphrase rejection. Those are two different meanings
  for the same number in two files in this tree.

**What to do:** document what the code does, and treat the namespaced scheme as
**not yet implemented** until it exists in `errors.rs` and in
`contracts/shared/`. This document does not resolve the contradiction by
picking a side; it records both and flags the source as authoritative for
today's behaviour. If you have already written client code against the
namespaced scheme, it will not work against a build of the current source.

> Two smaller gaps in the same area, for completeness:
> `docs/api-reference.md`'s `EphemeralAccount` error table stops at code `14`
> and omits `NotUpgradeAdmin` (`15`), which does exist in
> `contracts/ephemeral_account/src/errors.rs`.

---

## `EphemeralAccount` errors

Source: `contracts/ephemeral_account/src/errors.rs`, `#[repr(u32)]`,
discriminants 1–15.

| Code | Variant | Fires when | What to do |
|---:|---|---|---|
| 1 | `AlreadyInitialized` | `initialize()` called on an instance that is already initialized | **Stop.** Deploy a fresh instance. There is no reset and no second `initialize` |
| 2 | `NotInitialized` | Any guarded call on an instance where `initialize` never ran — `record_payment`, `sweep`, `sweep_claim`, `expire`, `reclaim_reserve`, `recover`, `get_info`, `get_info_paginated`, `upgrade` | **Stop and fix the account ID.** Not a transient condition; retrying cannot change it. Confirm the deployment per `testnet/registry/verify-contract-live.md` |
| 3 | `PaymentAlreadyReceived` | **Never constructed anywhere in the crate.** It is declared and nothing returns it | No handling. `docs/api-reference.md` marks it deprecated in favour of `DuplicateAsset` (13) |
| 4 | `InvalidAmount` | `record_payment` with `amount <= 0`; also the overflow guards in `reclaim_reserve_to`, `expire` and `recover` | **Stop.** Fix the argument. Never retry unchanged |
| 5 | `InvalidExpiry` | `initialize()` with `expiry_ledger <= current_ledger` | **Re-fetch state, then re-submit.** Re-read the latest ledger, recompute with a margin, and call again. This is the one code `1`–`15` that a retry can fix |
| 6 | `NotExpired` | `expire()` or `recover()` called before `expiry_ledger` | **Stop and wait.** Do not retry on a timer loop; poll `is_expired()` instead |
| 7 | `AlreadySwept` | `sweep()` or `sweep_claim()` on an account whose status is `Swept` | **Terminal, but benign.** For a duplicate submission this is the *expected* result — see [`idempotency-considerations.md`](idempotency-considerations.md). Read `get_info().swept_to` to find where the funds went |
| 8 | `Unauthorized` | `verify_sweep_authorization` when no `authorized_controller` is stored; `sweep_claim` in the same situation; `recover(caller)` when `caller` is neither creator nor recovery address | **Stop and escalate.** Either the call is being made outside the controller, or the wrong key signed. `EphemeralAccount` has no real signature check — see `testnet/security/unverified-signature-path-warning.md` |
| 9 | `InvalidSignature` | **Never constructed anywhere in the crate** | No handling. The `auth_signature` argument to `sweep()` is ignored entirely |
| 10 | `NoPaymentReceived` | `sweep()` or `sweep_claim()` on an account with no recorded payment | **Stop.** Record the payment first. If you believe a payment exists, the account ID or the `record_payment` call is wrong — `record_payment` does not verify or transfer the underlying asset |
| 11 | `AccountExpired` | `sweep()` or `sweep_claim()` when `is_expired()` is `true` | **Terminal. Route to recovery**, do not re-sweep. See [`testnet/runbooks/expired-account-recovery-check.md`](../runbooks/expired-account-recovery-check.md) |
| 12 | `InvalidStatus` | `expire()` or `recover()` when status is already `Swept` or `Expired`; `reclaim_reserve()` when the status is neither | **Terminal, benign for a duplicate.** The one-shot transition is already consumed |
| 13 | `DuplicateAsset` | `record_payment` for an asset that already has a recorded payment | **Terminal for that asset.** Note this is what you get, *not* code `3`. If a duplicate `record_payment` was the intent, the second entry is redundant; the original is unchanged |
| 14 | `TooManyPayments` | `record_payment` when 10 distinct assets are already recorded | **Stop.** The cap is 10. Sweep, or use a fresh account |
| 15 | `NotUpgradeAdmin` | `upgrade()` when no admin is stored in instance storage | **Stop and escalate.** An admin-less instance is an operational problem, not a caller error |

Two behavioural details that catch integrators out:

- **`record_payment` returns `DuplicateAsset` (13) and `TooManyPayments` (14) —
  never `PaymentAlreadyReceived` (3).** The "one payment per asset" rule is
  enforced by asset, not by account. A second call with a *different* asset
  succeeds and moves status along. Up to 10 distinct assets are accepted.
- **`record_payment` has no `require_auth()`** in the current source, and
  neither `sweep()` nor `expire()` is auth-gated beyond what the sweep path
  checks. `testnet/docs/getting-started.md` states that only the instance's
  `authorized_controller` can call `record_payment`; the current source does
  not enforce that. Do not build an access-control assumption on either
  statement — verify against the deployed WASM.

---

## `SweepController` errors

Source: `contracts/sweep_controller/src/errors.rs`, **no `#[repr(u32)]`**,
discriminants 1–11 then 13.

| Code | Variant | Fires when | What to do |
|---:|---|---|---|
| 1 | `InvalidAccount` | **Never constructed in the crate.** It appears only in doc comments | No handling. If you receive it, something other than this source produced it |
| 2 | `TransferFailed` | A SEP-41 `TokenClient::transfer()` inside `execute_transfers` returned an error, mapped by `.map_err(\|_\| Error::TransferFailed)` — the underlying reason is discarded | **Re-fetch state, then retry at most once.** First check `get_info` / `can_sweep`: a partial transfer attempt is not visible here. The mapping discards the cause, so a repeat that fails the same way needs escalation, not another retry |
| 3 | `AuthorizationFailed` | `initialize()` called twice; `update_authorized_destination` when no creator is stored. A wrong `creator` auth surfaces as a host auth error, **not** as this code | **Stop.** For `initialize` it means the controller is already configured. For `update_authorized_destination` it means the controller was never initialized |
| 4 | `InsufficientBalance` | **Never constructed.** `docs/api-reference.md` marks it "reserved for future use" | No handling |
| 5 | `AccountNotReady` | After the account's `sweep()` succeeded, `info.payment_received` is false, or the summed payment amount is `0` | **Re-fetch state, then stop.** Re-read `get_info`; if the account now reports `Swept` the sweep actually completed and this is a post-hoc observation artifact. If it still reports not-ready, escalate |
| 6 | `AccountExpired` | **Never constructed in the controller.** The expiry guard lives in `EphemeralAccount` and returns *its* code `11` | Expect `Error(Contract, #11)`, not `#6`. Do not write a handler that only knows the controller's numbering |
| 7 | `AccountAlreadySwept` | `update_authorized_destination` when the controller nonce is `> 0` | **Terminal.** The destination is immutable once any sweep has succeeded. This is really a statement about the nonce, not about any one account — the nonce is global to the controller |
| 8 | `InvalidSignature` | **Never constructed** | No handling |
| 9 | `SignatureVerificationFailed` | **Never constructed — and cannot be.** `verify_sweep_auth` calls `env.crypto().ed25519_verify(...)`, which **traps** on a bad signature rather than returning an error. There is no `Err` to convert | **Recognise the trap instead.** A rejected signature produces no contract error at all. See [Errors that are not contract errors](#errors-that-are-not-contract-errors) |
| 10 | `AuthorizedSignerNotSet` | `verify_sweep_auth` when no signer is in storage | **Stop and escalate.** The controller is not initialized. A successful sweep is impossible until it is |
| 11 | `InvalidNonce` | **Never constructed.** There is no nonce validation beyond what `ed25519_verify` enforces implicitly | No handling |
| 13 | `UnauthorizedDestination` | `validate_destination` when the controller has an `authorized_destination` and the supplied destination differs; also when the stored destination fails to read back | **Stop and fix the destination.** Re-sign for the correct destination — the digest binds the destination, so a signature for the wrong one would not have verified anyway. There is no on-chain getter for `authorized_destination`; the source is the `initialize` / `update_authorized_destination` call |

**The reachability column is the important one.** Six of the twelve variants
can never be produced by the current source: `1`, `4`, `6`, `8`, `9`, `11`.
That includes `SignatureVerificationFailed`, which both
`docs/api-reference.md` and `testnet/runbooks/failed-sweep-signature.md` list as
the expected symptom of a bad signature. It is not. The real symptom is a trap.

---

## `ReserveContract` errors

Source: `contracts/reserve_contract/src/errors.rs`, `#[repr(u32)]`,
discriminants 1–6, each with a doc comment in the source.

| Code | Variant | Fires when | What to do |
|---:|---|---|---|
| 1 | `InvalidAmount` | `set_base_reserve` with `amount <= 0` | **Stop.** Fix the argument. Note this is a stroop value: 1 XLM is `10_000_000` |
| 2 | `ReserveNotSet` | `require_base_reserve()` before any value has been stored | **Re-fetch state, then escalate to the admin.** Read via `has_base_reserve()` or the `Option`-returning `get_base_reserve()` instead, as the source's own doc comment advises. If you need a value, the admin must call `set_base_reserve` |
| 3 | `Unauthorized` | **Never constructed.** `set_base_reserve` returns `NotInitialized` (5) when no admin is stored, and a wrong signer produces a host auth error | No handling. If your SDK surfaces `Unauthorized` here, check the auth failure path instead |
| 4 | `AlreadyInitialized` | `initialize(admin)` called when an admin is already stored | **Stop.** The contract may only be initialized once, deliberately, to prevent admin takeover |
| 5 | `NotInitialized` | `set_base_reserve` before `initialize` | **Stop.** The admin must initialize first |
| 6 | `AmountTooLarge` | `set_base_reserve` with `amount > MAX_RESERVE_STROOPS` | **Stop.** The ceiling is `100_000_000_000` stroops = 10,000 XLM. This exists to catch XLM-vs-stroops mistakes, so re-check your units before retrying |

`ReserveContract` also returns `Option` from `get_base_reserve()` and
`get_admin()`, and `bool` from `has_base_reserve()`. Those do not error, and
that is deliberate: `get_base_reserve` "must" be handled for the unset case so
nobody silently consumes a zero. Prefer the `Option` form to
`require_base_reserve` in integration code.

Nothing in the current source reads `ReserveContract` from
`EphemeralAccount` — the account uses a hardcoded
`BASE_RESERVE_STROOPS = 1_000_000_000` — so a successful `set_base_reserve`
does not change sweep behaviour today.

---

## `AccountFactory`: no error enum exists

There is no `contracts/account_factory/src/errors.rs`, and the contract defines
no `#[contracterror]` enum. "Mapping its error variants" is therefore a
different exercise, and there are three distinct failure surfaces:

| Surface | What the caller sees | What to do |
|---|---|---|
| `batch_initialize(creator, requests)` | A `Vec<AccountInitResult>` where each entry has `account_address`, `success: bool`, `error: Option<Bytes>` — and on failure `error` is **always `None`** | **Cannot be automated from the result.** You learn *that* an account failed, not *why*. Route to `testnet/runbooks/account-factory-batch-failure.md` |
| `batch_initialize_paginated(creator, requests, params)` | The same `AccountInitResult` items inside `PaginatedAccountInitResultResponse`, with the same dropped `error` | Same as above |
| `initialize(ephemeral_account_wasm_hash)` | The storage `.unwrap()` on the WASM hash **panics** if the contract was never initialized | Not a contract error. It is a host-level panic, and the fix is to call `initialize` first |

The dropped error detail is confirmed in three places: the source sets
`error: None` in both the success and failure arms of the `match` in
`batch_initialize` and again in `batch_initialize_paginated`; the main
`README.md` records it as "Implemented, error detail dropped"; and
`testnet/registry/contract-status.md` lists it as a known issue. `docs/fee-model.md`
notes the operational consequence: monitoring must account for silent failures.

Practical handling: after a batch, for each `AccountInitResult` with
`success: false`, read the account directly with `EphemeralAccount::get_info`.
If that returns `Ok`, initialization actually succeeded and the batch result is
wrong. If it errors with `NotInitialized` (code `2`), initialization genuinely
did not happen and the reason is not recoverable from the factory. The likely
causes — an `expiry_ledger` in the past (`InvalidExpiry`, 5) or an
auth-related failure — have to be diagnosed out of band.

> `batch_initialize` also passes the **creator** as both
> `authorized_controller` and `admin` for every account it creates. Accounts
> from this path are not wired to your `SweepController` unless you
> re-initialize, and `initialize` is one-shot. Check the addresses before
> planning a sweep on a factory-created account.

---

## Handling code

Rust is the house choice for this tree: `docs/api-reference.md` has "Rust SDK
Integration" sections, `testnet/integration/type-mapping.md` maps types for
the Rust SDK, and the contracts are Rust. The example below is written against
`soroban-sdk` 22.0.0, the version every contract in this workspace pins.

**The shape to copy is an explicit `match` on the literal discriminant, per
contract.** Not an index into a table, and not a `from_code` that assumes
contiguity.

```rust
// Illustrative integration wrapper. Not shipped in this repo.
use soroban_sdk::Error as SdkError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BridgeletError {
    Account(EphemeralAccountError),
    Controller(SweepControllerError),
    Reserve(ReserveContractError),
    /// A failure that is not a #[contracterror] at all: an auth failure,
    /// a host trap, an unreachable endpoint. Keep these separate.
    NonEnum { diagnostic: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum EphemeralAccountError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    PaymentAlreadyReceived = 3,
    InvalidAmount = 4,
    InvalidExpiry = 5,
    NotExpired = 6,
    AlreadySwept = 7,
    Unauthorized = 8,
    InvalidSignature = 9,
    NoPaymentReceived = 10,
    AccountExpired = 11,
    InvalidStatus = 12,
    DuplicateAsset = 13,
    TooManyPayments = 14,
    NotUpgradeAdmin = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SweepControllerError {
    InvalidAccount = 1,
    TransferFailed = 2,
    AuthorizationFailed = 3,
    InsufficientBalance = 4,
    AccountNotReady = 5,
    AccountExpired = 6,
    AccountAlreadySwept = 7,
    InvalidSignature = 8,
    SignatureVerificationFailed = 9,
    AuthorizedSignerNotSet = 10,
    InvalidNonce = 11,
    // No 12. Do not renumber, and do not assume contiguity.
    UnauthorizedDestination = 13,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ReserveContractError {
    InvalidAmount = 1,
    ReserveNotSet = 2,
    Unauthorized = 3,
    AlreadyInitialized = 4,
    NotInitialized = 5,
    AmountTooLarge = 6,
}

impl EphemeralAccountError {
    /// Decode a bare `Error(Contract, #N)` discriminant.
    ///
    /// `None` means "not a code this build can produce" — which includes 12 for
    /// SweepController and any code in the 1000-2999 range the changelog
    /// describes but the source does not implement.
    pub fn from_code(code: u32) -> Option<Self> {
        Some(match code {
            1 => Self::AlreadyInitialized,
            2 => Self::NotInitialized,
            3 => Self::PaymentAlreadyReceived, // declared, never constructed
            4 => Self::InvalidAmount,
            5 => Self::InvalidExpiry,
            6 => Self::NotExpired,
            7 => Self::AlreadySwept,
            8 => Self::Unauthorized,
            9 => Self::InvalidSignature,       // declared, never constructed
            10 => Self::NoPaymentReceived,
            11 => Self::AccountExpired,
            12 => Self::InvalidStatus,
            13 => Self::DuplicateAsset,
            14 => Self::TooManyPayments,
            15 => Self::NotUpgradeAdmin,
            _ => return None,
        })
    }

    pub fn is_retryable_unchanged(self) -> bool {
        matches!(self, Self::InvalidExpiry) // only re-fetch-and-recompute fixes this
    }

    /// Terminal, but the requested end state may already hold.
    pub fn is_benign_duplicate(self) -> bool {
        matches!(self, Self::AlreadySwept | Self::InvalidStatus | Self::DuplicateAsset)
    }
}
```

Classify once, at the boundary, so the rest of the integration never parses
strings:

```rust
/// `invoking` is the contract you called, not the one you hoped you called.
pub fn classify(invoking: &str, err: SdkError) -> BridgeletError {
    match err {
        // The only branch that carries a #[contracterror] discriminant.
        SdkError::Contract(code) => match invoking {
            "EphemeralAccount" => EphemeralAccountError::from_code(code)
                .map(BridgeletError::Account)
                // An unrecognised code: report it, never guess.
                .unwrap_or_else(|| unmapped("EphemeralAccount", code)),

            // Explicit arms, with the gap at 12 left visible on purpose.
            "SweepController" => match code {
                1 => ctrl(SweepControllerError::InvalidAccount),
                2 => ctrl(SweepControllerError::TransferFailed),
                3 => ctrl(SweepControllerError::AuthorizationFailed),
                4 => ctrl(SweepControllerError::InsufficientBalance),
                5 => ctrl(SweepControllerError::AccountNotReady),
                6 => ctrl(SweepControllerError::AccountExpired),
                7 => ctrl(SweepControllerError::AccountAlreadySwept),
                8 => ctrl(SweepControllerError::InvalidSignature),
                9 => ctrl(SweepControllerError::SignatureVerificationFailed),
                10 => ctrl(SweepControllerError::AuthorizedSignerNotSet),
                11 => ctrl(SweepControllerError::InvalidNonce),
                13 => ctrl(SweepControllerError::UnauthorizedDestination),
                // There is no 12. Anything else is unknown.
                _ => unmapped("SweepController", code),
            },

            "ReserveContract" => match code {
                1 => res(ReserveContractError::InvalidAmount),
                2 => res(ReserveContractError::ReserveNotSet),
                3 => res(ReserveContractError::Unauthorized),
                4 => res(ReserveContractError::AlreadyInitialized),
                5 => res(ReserveContractError::NotInitialized),
                6 => res(ReserveContractError::AmountTooLarge),
                _ => unmapped("ReserveContract", code),
            },

            other => unmapped(other, code),
        },

        // Everything else has no discriminant: auth failure, host trap,
        // WASM/VM error, or an unreachable endpoint surfacing as a transport
        // failure. Never decode a code out of these.
        other => BridgeletError::NonEnum {
            diagnostic: format!("{other:?}"),
        },
    }
}

fn ctrl(e: SweepControllerError) -> BridgeletError {
    BridgeletError::Controller(e)
}

fn res(e: ReserveContractError) -> BridgeletError {
    BridgeletError::Reserve(e)
}

fn unmapped(contract: &str, code: u32) -> BridgeletError {
    BridgeletError::NonEnum {
        diagnostic: format!("{contract}: unmapped contract error code {code}"),
    }
}

/// `true` means the failure IS a contract error and a discriminant is present.
pub fn is_contract_error(err: &SdkError) -> bool {
    matches!(err, SdkError::Contract(_))
}
```

> The snippet above is illustrative and deliberately not a production
> implementation. It exists to show three properties worth copying: each
> contract gets its **own explicit arm**, so the `SweepController` gap at `12`
> cannot be papered over; an unrecognised code becomes `NonEnum` with the
> number preserved rather than a guessed variant; and `NonEnum` is a distinct
> state, so a host trap can never be mistaken for a contract error. If you
> write this for real, prefer the contract crates' own
> `TryFrom<Error>` implementations where they exist — the generated
> `try_*` client methods already return
> `Result<Result<T, ConversionError>, Result<Error, InvokeError>>` — and keep
> the unmapped arm. **Not compiled or tested in this repository.**

The retry decision, in one place:

```rust
pub enum Action {
    /// The requested end state already holds. Reconcile and move on.
    TreatAsSuccess,
    /// Re-read state, recompute, submit again. Bounded.
    RetryAfterRefetch,
    /// Do not resubmit unchanged. Read state and pick a different action.
    Stop,
    /// Operator problem. Page someone.
    Escalate,
}

pub fn decide(e: &BridgeletError) -> Action {
    match e {
        // A duplicate submission landed on an already-terminal account.
        BridgeletError::Account(x) if x.is_benign_duplicate() => Action::TreatAsSuccess,
        BridgeletError::Account(x) if x.is_retryable_unchanged() => Action::RetryAfterRefetch,
        // AccountExpired / NotExpired / NotInitialized / InvalidAmount /
        // Unauthorized / NoPaymentReceived / TooManyPayments / NotUpgradeAdmin
        BridgeletError::Account(_) => Action::Stop,

        // Never constructed in the current source. If one arrives, it came
        // from a different build than the one this table was written from.
        BridgeletError::Controller(SweepControllerError::SignatureVerificationFailed)
        | BridgeletError::Controller(SweepControllerError::InvalidNonce)
        | BridgeletError::Controller(SweepControllerError::InvalidSignature)
        | BridgeletError::Controller(SweepControllerError::InsufficientBalance)
        | BridgeletError::Controller(SweepControllerError::InvalidAccount)
        | BridgeletError::Controller(SweepControllerError::AccountExpired) => Action::Escalate,

        BridgeletError::Controller(SweepControllerError::TransferFailed)
        | BridgeletError::Controller(SweepControllerError::AccountNotReady) => Action::RetryAfterRefetch,
        BridgeletError::Controller(SweepControllerError::AccountAlreadySwept) => Action::TreatAsSuccess,
        BridgeletError::Controller(_) => Action::Stop,

        BridgeletError::Reserve(ReserveContractError::ReserveNotSet) => Action::RetryAfterRefetch,
        BridgeletError::Reserve(_) => Action::Stop,

        // A trap, an auth failure, or an unmapped code: never blind-retry.
        BridgeletError::NonEnum { .. } => Action::Escalate,
    }
}
```

And the trap handler, which is not optional on a real integration — a failure
where `is_contract_error` is `false` must never be reported to an operator as
`Error(Contract, #N)`, because there is no `N`.

---

## Errors that are not contract errors

A document that maps only enum variants will mislead anyone who has only ever
seen a trap. On this codebase, traps are at least as likely as enum variants.

### `ed25519_verify` traps, and it is usually the nonce

`SweepController::verify_sweep_auth` calls
`env.crypto().ed25519_verify(&authorized_signer, &message.into(), signature)`
and ignores nothing — but `ed25519_verify` **traps** on failure. There is no
`Err`, so there is no `SignatureVerificationFailed` (9) to catch, no
discriminant to match, and no diagnostic. The whole transaction aborts with a
bare host-function error.

The trap means the digest did not match. In practice the most common cause is
**nonce contention, not a signing bug**: the signature was built over a nonce
that is no longer current, because the controller's nonce is global and
advances on every successful sweep, including other integrators'. On a shared
testnet deployment this is the single most common cause.
`testnet/security/README.md` and
`testnet/runbooks/failed-sweep-signature.md` both say so.

Order of investigation: read `get_nonce()` → compare with the nonce you signed
→ if they differ, re-read and re-sign → if they match, the destination,
controller ID, signer key, or XDR encoding is wrong. See
`testnet/config/nonce-tracking.md` and
`testnet/tooling/nonce-inspector-spec.md`.

### The signature-verification asymmetry

Two functions, two very different meanings:

| | `SweepController::verify_sweep_auth` | `EphemeralAccount::verify_sweep_authorization` |
|---|---|---|
| Verifies the signature | **Yes** — real `ed25519_verify` | **No** — `auth_signature` is bound to `_signature` and discarded |
| Failure mode | Host trap, no error code | None from the signature at all |
| Actual authorization | Ed25519 signature over `SHA256(destination ‖ nonce ‖ controller_id)` | `authorized_controller.require_auth()` — whichever address was stored at `initialize()` |

So `EphemeralAccount`'s `InvalidSignature` (9) does not mean what
`SweepController`'s `SignatureVerificationFailed` (9) means — the first is
never produced, and the second is never produced either. Passing garbage to
`EphemeralAccount::sweep()` is not rejected; it is ignored. That is the live
footgun in `testnet/security/unverified-signature-path-warning.md`, and it is
why a successful direct `sweep()` is not evidence that any signature was
checked.

### `Error(Auth, InvalidAction)`

An authorization failure has no `#[contracterror]` discriminant. It appears as
`Error(Auth, InvalidAction)` in the tree's own troubleshooting table
(`testnet/docs/getting-started.md`). It means the signer did not authorise the
call — the `creator` for `initialize`, the stored `admin` for
`ReserveContract::set_base_reserve` or `EphemeralAccount::upgrade`, the
`recipient` for `SweepController::claim`, or the creator for
`update_authorized_destination`. Route it to **Stop**, never to a retry loop:
the signature will not become valid on its own.

### `Error(Contract, #1007)` — the network-passphrase footgun

`testnet/docs/changelog.md`'s Unreleased section states that `initialize`
enforces a network-passphrase check, and that the expected value on `main` is
currently the Standalone passphrase, which "would reject testnet calls with
`Error(Contract, #1007)`".

Two honest notes:

- That check **does not exist in the current source**. `EphemeralAccount::initialize`
  in `contracts/ephemeral_account/src/lib.rs` validates only
  `expiry_ledger` and the `initialized` flag. So `#1007` cannot be produced by
  this build's `initialize`.
- `testnet/docs/getting-started.md` maps `#1007` to `Unauthorized` instead. The
  two files disagree about what the same number means, which is another
  consequence of the un-namespaced numbering described above.

Practical advice that survives both readings: use the **Testnet** passphrase
`Test SDF Network ; September 2015` exactly, as
`testnet/docs/network-config.md` specifies, and treat a passphrase-shaped
rejection as a configuration problem rather than a contract error.

> Two smaller inconsistencies found while verifying this table:
> `testnet/docs/network-config.md` says
> `bridgelet_shared::passphrase::TESTNET_PASSPHRASE` exposes the passphrase
> constant, but `contracts/shared/src/lib.rs` has no `passphrase` module and
> re-exports only types. `testnet/integration/type-mapping.md` gives the
> passphrase as `Test SDF Network ; November 2015`, against network-config's
> `September 2015`. Use the `network-config.md` value.

### `Error(WasmVm, InvalidAction)`

Not a contract error at all — a WASM/VM-level failure, mentioned in
`scripts/build.sh` in the context of `reference-types not enabled`. It points at
the build, not at your call. **Stop and escalate.**

### When to treat a failure as "the node is not answering"

A confusing Soroban error is frequently an unreachable or unhealthy RPC
endpoint rather than anything about your transaction. Check `getHealth` and
`getLatestLedger` before you start decoding anything. There is a ready-made
collection of read-only probes for exactly this in
[`postman-collection.json`](postman-collection.json).

---

## Common pitfalls

| Pitfall | Symptom | Why | Fix |
|---|---|---|---|
| Decoding a bare integer without knowing the contract | "code 2" means three different things | Discriminants are contract-scoped | Carry the invoking contract ID through the error path |
| Assuming contiguous codes | `UnauthorizedDestination` reported as `12` | `SweepController` skips `12` | Explicit `match` on literals; `None` for unknown |
| Building a `from_code` with `.enumerate()` | Silent mis-maps the day an enum is reordered | Index ≠ discriminant | `match` on the literal; add a test |
| Handling the changelog's 1000–2999 scheme | Every code is unknown | The namespacing is not implemented | Treat it as not-yet-implemented; re-check `errors.rs` |
| Matching on `SignatureVerificationFailed` | Never fires; a bad signature traps | `ed25519_verify` traps | Handle the trap; read `get_nonce()` first |
| Treating an `ed25519_verify` trap as a signing bug | Hours lost re-deriving a correct signature | It is usually nonce contention | `get_nonce()`, compare, re-sign |
| Blind-retrying `TransferFailed` | Repeated fees, same cause | The underlying token error is discarded by `.map_err` | Re-fetch state; escalate on a second failure |
| Trusting `AccountInitResult.error` | You think you know why a batch account failed | It is always `None` | Diagnose with `get_info`; use the batch-failure runbook |
| Assuming `record_payment` rejects a second call | Unexpected `DuplicateAsset` (13) | The rule is one payment *per asset*, and there is no `require_auth` | Up to 10 distinct assets; code `3` is never returned |

---

## Related Documentation

- [`docs/api-reference.md`](../../docs/api-reference.md) — the authoritative
  per-contract `### Error Codes` tables this document accompanies
- [`contracts/ephemeral_account/src/errors.rs`](../../contracts/ephemeral_account/src/errors.rs)
- [`contracts/sweep_controller/src/errors.rs`](../../contracts/sweep_controller/src/errors.rs)
- [`contracts/reserve_contract/src/errors.rs`](../../contracts/reserve_contract/src/errors.rs)
- [`README.md`](README.md) — the parent example index and placeholder preamble
- [`checking-account-status.md`](checking-account-status.md) — the read-only
  pre-flight that avoids most of these failures paying for themselves
- [`idempotency-considerations.md`](idempotency-considerations.md) — which of
  these codes a duplicate submission actually produces
- [`postman-collection.json`](postman-collection.json) — raw RPC probes, useful
  when an error is really an unreachable node
- `testnet/runbooks/failed-sweep-signature.md` — the signature trap, in detail
- `testnet/runbooks/nonce-desync.md` — local cache and chain disagree
- `testnet/runbooks/account-factory-batch-failure.md` — diagnosing a batch
  with no error detail
- [`testnet/security/unverified-signature-path-warning.md`](../security/unverified-signature-path-warning.md)
  — the ignored `auth_signature`
- [`testnet/security/README.md`](../security/README.md) — traps that are
  actually abuse or contention, not bugs
- `testnet/docs/changelog.md` — the Unreleased section that contradicts the
  source on error-code namespacing
- [`testnet/integration/type-mapping.md`](../integration/type-mapping.md) —
  how contract types map into the Rust SDK
- `contracts/account_factory/src/lib.rs` — the dropped `error: None`

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-09-26 | Initial error-to-handling map for `EphemeralAccount`, `SweepController`, `ReserveContract`, and the `AccountFactory` no-enum case |
