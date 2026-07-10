# Bridgelet Core Architecture

**Version:** 1.1 (corrected against actual `main` source)
**Last Updated:** July 10, 2026
**Status:** MVP — not audited

> **Change from v1.0:** the previous version of this document described a
> two-contract system with `SweepController` marked "Planned (Not Yet
> Implemented)" throughout. That is no longer accurate (and may never have
> matched the actual code). The workspace has **four** contracts, three of
> which are fully implemented, and this revision documents all four as they
> exist in `contracts/` today.

---

## Table of Contents

1. [Introduction](#introduction)
2. [System Architecture](#system-architecture)
3. [Contract Details](#contract-details)
   - [EphemeralAccount](#ephemeralaccount-contract)
   - [SweepController](#sweepcontroller-contract)
   - [ReserveContract](#reservecontract-contract)
   - [AccountFactory](#accountfactory-contract)
4. [Data Flow](#data-flow)
5. [Design Decisions](#design-decisions)
6. [Integration Points](#integration-points)
7. [Limitations](#limitations)

---

## Introduction

### Overview

Bridgelet Core is a suite of Soroban smart contracts that enable secure, single-use ephemeral accounts on the Stellar network. The system enforces business logic restrictions on temporary accounts, ensuring they can only receive tracked payments and be swept to a pre-authorized destination or expired to a recovery address.

### Target Audience

- **SDK Developers** building on top of Bridgelet Core
- **Security Auditors** reviewing contract design
- **Integration Partners** connecting external systems
- **Contract Maintainers** working on the codebase

---

## System Architecture

### Component Overview

The workspace (`Cargo.toml`) declares five members — four deployable contracts plus a shared library:

```
contracts/
├── ephemeral_account/   # Per-transfer temporary account + state machine
├── sweep_controller/    # Signature verification + token transfer execution
├── reserve_contract/    # Standalone base-reserve config store
├── account_factory/     # Batch deployer/initializer for ephemeral_account
└── shared/               # Common types (Payment, AccountStatus, AccountInfo, ...)
```

```
┌──────────────────────────────────────────────────────────────────────┐
│                        Off-chain SDK / Relayer                       │
│   monitors payments (Horizon), signs sweep authorizations, submits   │
│   transactions to invoke the contracts below                         │
└───────────────┬───────────────────────────┬───────────────────────────┘
                │                            │
                ▼                            ▼
   ┌─────────────────────┐        ┌───────────────────────┐
   │   AccountFactory     │───────▶│   EphemeralAccount     │
   │  batch-deploys N     │ deploys│  (one instance per     │
   │  ephemeral accounts  │        │   temporary account)   │
   └─────────────────────┘        └───────────┬────────────┘
                                                │ sweep() / sweep_claim()
                                                ▼
                                    ┌───────────────────────┐
                                    │    SweepController     │
                                    │  verifies Ed25519 sig,  │
                                    │  calls back into the    │
                                    │  account, then executes │
                                    │  SEP-41 token transfers  │
                                    └───────────┬────────────┘
                                                │ TokenClient.transfer()
                                                ▼
                                        SEP-41 token contracts

   ┌─────────────────────┐
   │   ReserveContract     │  standalone config contract — stores an
   │  (admin-set base       │  admin-set base reserve amount; NOT
   │   reserve amount)      │  currently wired into EphemeralAccount's
   └─────────────────────┘  own internal reserve bookkeeping (see
                             Limitations)
```

### Component Responsibilities

#### Bridgelet SDK (Off-Chain, not in this repo)
- Monitors incoming payments via Horizon and calls `record_payment()`
- Holds (or has access to, via HSM/relayer) the Ed25519 private key matching `authorized_signer` on `SweepController`
- Signs sweep-authorization messages and/or submits `claim()` transactions on the recipient's behalf

#### EphemeralAccount (On-Chain, implemented)
- Enforces single-payment-per-asset, expiry, and status transitions
- Gates `sweep()`/`sweep_claim()` behind `authorized_controller.require_auth()`
- Tracks and reclaims an internal base-reserve amount on sweep/expiry

#### SweepController (On-Chain, implemented)
- Independently verifies Ed25519 signatures over `hash(destination + nonce + contract_id)`
- Enforces nonce-based replay protection
- Executes the actual SEP-41 `transfer()` calls for every recorded payment
- Optionally locks all sweeps to one pre-set destination address

#### ReserveContract (On-Chain, implemented, currently standalone)
- Simple `initialize` / `set_base_reserve` / `get_base_reserve` / `has_base_reserve` interface
- Admin-gated writes; bounded to 100,000,000,000 stroops (10,000 XLM)
- No other contract currently reads from this contract on-chain — see Limitations

#### AccountFactory (On-Chain, implemented)
- Stores a WASM hash for `ephemeral_account` at `initialize()`
- `batch_initialize()` deploys and initializes N ephemeral accounts in one transaction using deterministic salts (index-based)
- Swallows individual initialization error detail (returns `success: false, error: None`)

### Network Topology
Same Soroban RPC / Horizon endpoints used across all four contracts; no contract-specific network requirements beyond standard Stellar testnet/mainnet RPC access.

---

## Contract Details

### EphemeralAccount Contract

**Source:** `contracts/ephemeral_account/src/lib.rs`

#### State Machine
`AccountStatus`: `Active (0) → PaymentReceived (1) → Swept (2)`, or `Active → Expired (3)` via `expire()` after `expiry_ledger`.

#### Storage
Creator, status, expiry ledger, recovery address, authorized controller, admin, per-asset payments, swept-to destination, internal reserve-tracking fields (`BASE_RESERVE_STROOPS = 1_000_000_000`).

#### Function Reference (actual signatures)

```rust
fn initialize(
    env: Env,
    creator: Address,
    expiry_ledger: u32,
    recovery_address: Address,
    authorized_controller: Address,
    admin: Address,
) -> Result<(), Error>;

fn record_payment(env: Env, amount: i128, asset: Address) -> Result<(), Error>;

fn sweep(env: Env, destination: Address, auth_signature: BytesN<64>) -> Result<(), Error>;
fn sweep_claim(env: Env, destination: Address) -> Result<(), Error>;

fn is_expired(env: Env) -> bool;
fn expire(env: Env) -> Result<(), Error>;
fn get_status(env: Env) -> AccountStatus;
fn get_info(env: Env) -> Result<AccountInfo, Error>;
fn recover(env: Env, caller: Address) -> Result<(), Error>;
fn upgrade(env: Env, new_wasm_hash: BytesN<32>) -> Result<(), Error>;
fn simulate_sweep(env: Env, destination: Address) -> (Vec<Payment>, u32);

fn get_reserve_remaining(env: Env) -> i128;
fn get_reserve_available(env: Env) -> i128;
fn is_reserve_reclaimed(env: Env) -> bool;
fn get_last_reserve_event(env: Env) -> Option<ReserveReclaimed>;
fn get_reserve_reclaim_event_count(env: Env) -> u32;
```

#### `sweep()` — how authorization actually works today
1. Checks initialized / not-swept / payment-received / not-expired.
2. Calls `verify_sweep_authorization(&env, &destination, &auth_signature)`.
3. **That function ignores both arguments (`_destination`, `_signature`)** and instead does:
   ```rust
   let controller = storage::get_authorized_controller(env).ok_or(Error::Unauthorized)?;
   controller.require_auth();
   Ok(())
   ```
   i.e. it only verifies that the call is authorized as coming from whatever address was stored as `authorized_controller` at `initialize()` time. No Ed25519 check happens inside this contract.
4. Sets status to `Swept` *before* reserve reclaim (reentrancy guard).
5. Emits `SweepExecutedMulti`, then calls `reclaim_reserve_to()`.

Practical implication: an `EphemeralAccount` can only be swept by whatever address you set as `authorized_controller` — normally `SweepController`'s contract address — because that address must satisfy `require_auth()`. The actual signature verification that makes this safe happens one layer up, in `SweepController`.

#### `sweep_claim()`
Identical state checks, but skips the signature path entirely: it requires `controller.require_auth()` directly, no signature parameter at all. Used by `SweepController::claim()`.

#### Errors
`AlreadyInitialized, NotInitialized, PaymentAlreadyReceived, InvalidAmount, InvalidExpiry, NotExpired, AlreadySwept, Unauthorized, InvalidSignature, NoPaymentReceived, AccountExpired, InvalidStatus, DuplicateAsset, TooManyPayments, NotUpgradeAdmin`

Note: `InvalidSignature` (9) exists in the error enum but `sweep()`'s current implementation has no code path that returns it, since the signature is never actually checked in this contract.

---

### SweepController Contract

**Source:** `contracts/sweep_controller/src/lib.rs`, `authorization.rs`, `transfers.rs`

This is where real cryptographic authorization lives.

#### Function Reference

```rust
fn initialize(
    env: Env,
    creator: Address,
    authorized_signer: BytesN<32>,
    authorized_destination: Option<Address>,
) -> Result<(), Error>;

fn execute_sweep(
    env: Env,
    ephemeral_account: Address,
    destination: Address,
    auth_signature: BytesN<64>,
) -> Result<(), Error>;

fn claim(env: Env, recipient: Address, ephemeral_account: Address) -> Result<(), Error>;

fn can_sweep(env: Env, ephemeral_account: Address) -> bool;
fn update_authorized_destination(env: Env, new_destination: Address) -> Result<(), Error>;
```

#### Authorization Flow (implemented, not planned)

`verify_sweep_auth()` in `authorization.rs`:

1. Builds a message: `SHA256(destination.to_xdr() ++ nonce_be_u64 ++ contract_id.to_xdr())`, where `nonce` is this controller's current stored sweep nonce.
2. Calls `env.crypto().ed25519_verify(authorized_signer, message, signature)` — this **traps/panics on an invalid signature** (standard Soroban host-function behavior), so a failed verification aborts the transaction rather than returning an `Err`.
3. On success, `execute_sweep()` increments the nonce (replay protection) *before* calling into `EphemeralAccount`.
4. Uses `env.authorize_as_current_contract()` with a `SubContractInvocation` context so that the downstream `EphemeralAccount::sweep()` call satisfies its `authorized_controller.require_auth()` check.

#### Transfer Mechanism (implemented, not planned)

`transfers::execute_transfers()` iterates every `Payment` returned by `EphemeralAccount::get_info()` and calls `TokenClient::new(env, &payment.asset).transfer(from, destination, &payment.amount)` for each — atomic multi-asset sweep in one call.

#### `claim()` — gas-free path
`recipient.require_auth()` (Soroban native auth on the outer transaction) replaces the Ed25519 signature entirely; the controller then authorizes itself as invoker of `EphemeralAccount::sweep_claim()`. This lets a relayer submit and pay fees while only the recipient signs.

#### Destination locking
If `authorized_destination` was set at `initialize()`, every `execute_sweep`/`claim` call is checked against it (`validate_destination`) and can be changed via `update_authorized_destination()` — but only before any sweep has occurred (`nonce == 0` check).

#### Errors
`InvalidAccount, TransferFailed, AuthorizationFailed, InsufficientBalance, AccountNotReady, AccountExpired, AccountAlreadySwept, InvalidSignature, SignatureVerificationFailed, AuthorizedSignerNotSet, InvalidNonce, UnauthorizedDestination` (discriminant `12` is unused/skipped — likely a removed variant; harmless in Rust but worth a cleanup pass).

---

### ReserveContract Contract

**Source:** `contracts/reserve_contract/src/lib.rs`

A small, standalone admin-config contract:

```rust
fn initialize(env: Env, admin: Address) -> Result<(), Error>;
fn set_base_reserve(env: Env, amount: i128) -> Result<(), Error>;  // admin-gated, bounded to 100_000_000_000 stroops
fn get_base_reserve(env: Env) -> Option<i128>;
fn require_base_reserve(env: Env) -> Result<i128, Error>;
fn has_base_reserve(env: Env) -> bool;
fn get_admin(env: Env) -> Option<Address>;
```

**Not currently integrated:** `EphemeralAccount` computes its own reserve figures internally (`BASE_RESERVE_STROOPS` constant + its own storage), and nothing in the codebase has `EphemeralAccount` call into `ReserveContract` to read a live value. If the intent is for `ReserveContract` to become the single source of truth for the network base reserve, that cross-contract call does not exist yet.

---

### AccountFactory Contract

**Source:** `contracts/account_factory/src/lib.rs`

```rust
fn initialize(env: Env, ephemeral_account_wasm_hash: BytesN<32>);
fn batch_initialize(
    env: Env,
    creator: Address,
    requests: Vec<AccountInitRequest>,
) -> Vec<AccountInitResult>;
```

Deploys a new `ephemeral_account` instance per request via `env.deployer().with_current_contract(salt).deploy_v2(...)`, using an index-derived salt, then calls `try_initialize()` on each. All accounts created this way get `authorized_controller = creator` and `admin = creator` (the factory passes `creator` for both of the last two `initialize` args).

**Known gap:** on a per-account failure, `AccountInitResult.error` is hardcoded to `None` (see inline comment: *"In a real implementation, we'd serialize errors"*). Callers can detect `success: false` but not the cause.

**Not wired into tooling:** not built by `scripts/build.sh`, not deployed by `scripts/deploy-testnet.sh`, not covered by either CI workflow.

---

## Data Flow

### Account Creation
`creator` → `EphemeralAccount::initialize()` (or via `AccountFactory::batch_initialize()` for many at once) → account is `Active`.

### Payment
SDK observes inbound payment via Horizon → calls `record_payment(amount, asset)` → status becomes `PaymentReceived`.

### Sweep (signed path)
SDK/relayer builds `hash(destination ++ nonce ++ sweep_controller_address)`, signs with the private key matching `authorized_signer` → calls `SweepController::execute_sweep(ephemeral_account, destination, signature)` → controller verifies signature, authorizes itself as invoker, calls `EphemeralAccount::sweep()` → controller reads `get_info()`, executes token transfers → `EphemeralAccount` reclaims its internal reserve tracking.

### Sweep (gas-free claim path)
Recipient signs a Soroban auth entry for `SweepController::claim(recipient, ephemeral_account)` → relayer submits and pays fees → controller authorizes itself as invoker of `EphemeralAccount::sweep_claim()` → same transfer/reserve-reclaim tail as above.

### Expiration
Past `expiry_ledger` with no sweep → anyone calls `expire()` (or `recover()`) → funds path returns to `recovery_address`.

---

## Design Decisions

### Why four contracts instead of one or two?
Separating signature verification (`SweepController`) from account state (`EphemeralAccount`) lets the signing/authorization scheme evolve (e.g. multi-sig, threshold signatures) without touching account state logic. `ReserveContract` and `AccountFactory` are separable, optional concerns — but as shipped, `ReserveContract` isn't yet consumed by the others, so the separation hasn't paid off in this MVP.

### Why Soroban native `require_auth()` for the controller gate, instead of doing the Ed25519 check inside `EphemeralAccount` itself?
Keeps signature-scheme changes isolated to `SweepController`. Trade-off: anyone reading only `ephemeral_account/src/lib.rs` in isolation could reasonably (and incorrectly) assume `auth_signature` is verified there — it isn't, and that's the single most important thing to communicate to new contributors or auditors.

### Why is nonce state per-`SweepController`, not per-`EphemeralAccount`?
Because one `SweepController` deployment can service many ephemeral accounts (that's the point of the split), replay protection has to live wherever the signature is actually checked — `SweepController` — not on the individual account.

---

## Integration Points

### SDK Integration
- Monitor payments via Horizon, call `record_payment()`.
- Hold/access the private key matching `SweepController`'s `authorized_signer`, or drive the `claim()` gas-free path instead.
- Never call `EphemeralAccount::sweep()` directly from SDK code with an ad hoc signature — it isn't verified there; always go through `SweepController`.

### Off-Chain Requirements
- Event indexing for `AccountCreated`, `PaymentReceived`/`MultiPaymentReceived`, `SweepExecutedMulti`, `AccountExpired`, `ReserveReclaimed` (from `EphemeralAccount`) and `SweepCompleted`, `DestinationAuthorized`, `DestinationUpdated` (from `SweepController`).
- Payment monitoring service watching Horizon for inbound transfers to each ephemeral account address.

---

## Limitations

1. **No independent signature check inside `EphemeralAccount`.** All cryptographic authorization is centralized in `SweepController`. If you ever call `EphemeralAccount` directly (bypassing `SweepController`), the `auth_signature` parameter is decorative.
2. **`ReserveContract` is unintegrated.** It exists, builds, and has admin-gated setters, but nothing reads from it on-chain yet; `EphemeralAccount` maintains its own separate, hardcoded reserve constant.
3. **`AccountFactory` is undocumented in tooling.** Not built, deployed, or tested by any script or CI workflow in this repo as committed.
4. **Batch-creation error detail is dropped.** `AccountFactory::batch_initialize` reports failure but not cause per account.
5. **CI is currently non-functional as committed.** `test.yml` is fully commented out; `deploy-testnet.yml` runs validation only — its actual deploy/artifact/summary steps are commented out. See the root `README.md` for details.
6. **`scripts/deploy-testnet.sh` has a live bug:** it references `$RESERVE_CONTRACT_ID` in its final output/artifact-writing steps but never assigns it (the script doesn't deploy `reserve_contract` at all), and with `set -euo pipefail` this will abort with an unbound-variable error rather than silently omitting the value.
7. **Not audited.** No `docs/security-audit.md` exists yet ("coming soon" per the previous README); treat this MVP accordingly before moving real value through it.