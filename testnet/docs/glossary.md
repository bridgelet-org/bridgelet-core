# Testnet Glossary

A short, testnet-focused glossary of the domain terms you will meet in
bridgelet-core contract interfaces, events and deployment docs. It is meant as
an onboarding aid: start here, then move on to the
[Testnet Overview](./overview.md), the [FAQ](./faq.md) and the
[API reference](../../docs/api-reference.md).

Definitions reflect the **current MVP**, including its known stubs. In
particular, real Ed25519 verification lives only in `sweep_controller`.

## How to read an event

Soroban contract events carry a list of **topics** and a **data** value. The
first topic is the event name (a short symbol such as `created`); any further
topics are indexed fields you can filter on, and the data is the event payload.
The topic names below are the ones emitted on testnet:

| Topic | Event | Contract | Payload (data) |
|---|---|---|---|
| `created` | `AccountCreated` | EphemeralAccount | `creator`, `expiry_ledger` |
| `payment` | `PaymentReceived` | EphemeralAccount | `amount`, `asset` |
| `multi_pay` | `MultiPaymentReceived` | EphemeralAccount | multiple payments |
| `swept_mul` | `SweepExecutedMulti` | EphemeralAccount | `destination`, `payments` |
| `expired` | `AccountExpired` | EphemeralAccount | `recovery_address`, `total_amount`, `reserve_amount` |
| `reserve` | `ReserveReclaimed` | EphemeralAccount | `destination`, `amount`, `sweep_id`, `fully_reclaimed`, `remaining_reserve` |
| `sweep` | `SweepCompleted` | SweepController | `ephemeral_account`, `destination`, `amount` |
| `dest_auth` | `DestinationAuthorized` | SweepController | `destination` |
| `dest_upd` | `DestinationUpdated` | SweepController | `old_destination`, `new_destination` |

Note that `sweep` (`SweepCompleted`) is emitted by `SweepController`, not by
`EphemeralAccount`. See the [README events list](../../README.md#events) for the
source definitions.

## A

**AccountCreated**
Event (topic `created`) emitted when an ephemeral account is initialized. Its
data carries `creator` and `expiry_ledger`. See [ephemeral account](#ephemeral-account).

**AccountExpired**
Event (topic `expired`) emitted when an ephemeral account passes its
`expiry_ledger`. Data includes `recovery_address`, `total_amount` and
`reserve_amount`. See [expiry_ledger](#expiry_ledger).

**AccountFactory**
One of the four contracts in the workspace. It batch-deploys and initializes
many `ephemeral_account` instances from a stored WASM hash in one transaction.
It is **not deployed or wired** by the supported testnet tooling, and
`batch_initialize` discards per-account error detail. See
[Overview](./overview.md#4-account_factory-not-deployed-by-the-tooling).

**authorized_controller**
The address recorded on an ephemeral account at `initialize()` and checked with
`require_auth()` when the account's sweep functions are called. This - not the
signature argument - is what currently gates a direct `sweep()`.

**authorized_destination**
An optional address on `sweep_controller` that, when set, restricts sweeps to
one pre-approved destination ("locked" mode). The testnet deployment sets it to
`null` (unlocked), so any destination is accepted with a valid signature. The
creator can change it with `update_authorized_destination`.

**authorized_signer**
The raw 32-byte Ed25519 public key stored on `sweep_controller` and used to
verify sweep signatures. In the deployment record this is
`config.authorizedSigner` (64 hex digits). See
[Signature Format](../../docs/SIGNATURE_FORMAT.md).

## B

**base reserve**
The minimum XLM balance a Stellar account must maintain per ledger entry.
bridgelet-core accounts for this when reclaiming reserve during a sweep or
expiry. `reserve_contract` stores an admin-set base reserve (capped at 10,000
XLM), but it is **not wired into** `ephemeral_account` on testnet. See
[reserve](#reserve).

## C

**claim**
`SweepController::claim(recipient, ephemeral_account)` is the gas-free sweep
path: the recipient signs a Soroban auth entry, a [relayer](#relayer) submits
the transaction and pays fees, and the controller calls
`EphemeralAccount::sweep_claim` using `authorize_as_current_contract()`. See
[FAQ](./faq.md#so-how-should-i-sweep-funds-on-testnet).

## D

**DestinationUpdated**
Event (topic `dest_upd`) emitted by `SweepController` when
`update_authorized_destination` changes the locked destination. Data includes
`old_destination` and `new_destination`. See [authorized_destination](#authorized_destination).

## E

**Ed25519**
The signature scheme used for real sweep authorization. `sweep_controller`
uses `env.crypto().ed25519_verify()` over
`SHA256(destination_xdr || nonce_be_u64 || contract_id_xdr)`, with 32-byte
public keys and 64-byte signatures. See
[Signature Format](../../docs/SIGNATURE_FORMAT.md).

**ephemeral account**
A temporary Stellar account governed by the `ephemeral_account` contract. It
accepts a single inbound payment per asset, expires at a given
[ledger sequence](#ledger-sequence), and allows a controller-gated sweep. See
[Overview](./overview.md#1-ephemeral_account-deployed).

**expiry_ledger**
A `u32` ledger sequence passed to `EphemeralAccount::initialize()`. When the
chain reaches this ledger the account is expired; expiration is based on ledger
sequence, not wall-clock time. See [AccountExpired](#accountexpired) and
[recovery_address](#recovery_address).

## L

**ledger sequence**
The increasing number identifying a Stellar ledger. bridgelet-core uses it for
time-based expiry (`expiry_ledger`) instead of timestamps. See
[expiry_ledger](#expiry_ledger).

## N

**nonce**
An 8-byte big-endian counter on `sweep_controller` that starts at 0 and
increments after each successful sweep authorization. It is part of the signed
message and prevents replay of old signatures. Always query `get_nonce()`
before signing. See [Signature Format](../../docs/SIGNATURE_FORMAT.md).

## P

**PaymentReceived**
Event (topic `payment`) emitted by `ephemeral_account` when an inbound payment
is recorded. Data includes `amount` and `asset`. The multi-asset counterpart is
`MultiPaymentReceived` (topic `multi_pay`).

## R

**Recovery address** (`recovery_address`)
A Stellar `G...` address supplied at `EphemeralAccount::initialize()` as the
fallback destination if an account expires without being swept. In the
deployment record it is `config.recoveryAddress`.

**relayer**
An off-chain actor that submits a transaction and pays its fees on behalf of a
recipient using the gas-free `claim()` path. See [claim](#claim).

**require_auth**
A Soroban SDK primitive that traps unless the caller's address authorized the
invocation. It is the mechanism that currently gates `EphemeralAccount::sweep()`;
it performs no signature verification. See
[FAQ](./faq.md#why-does-sweep-accept-a-signature-parameter-it-doesnt-check).

**reserve**
The XLM balance an account must keep to exist on Stellar. bridgelet-core tracks
reserve for reclaim during sweeps/expiry; `reserve_contract` is a standalone,
admin-set store for the base-reserve value that is **not wired into**
`ephemeral_account` on testnet. See [base reserve](#base-reserve).

**ReserveContract**
One of the four contracts in the workspace: an `init`/`get`/`set`/`has` store
for the base-reserve amount (max 10,000 XLM). It is **not deployed or wired**
by the supported testnet tooling. See
[FAQ](./faq.md#is-reserve_contract-live-on-testnet).

**ReserveReclaimed**
Event (topic `reserve`) emitted by `ephemeral_account` when reserve is reclaimed.
Data includes `destination`, `amount`, `sweep_id`, `fully_reclaimed` and
`remaining_reserve`.

## S

**stroop**
The smallest unit of XLM: 1 XLM = 10,000,000 stroops. Amounts in contract
interfaces and events are `i128` stroop values.

**sweep**
The transfer of funds from an [ephemeral account](#ephemeral-account) to a
permanent destination wallet. Always route sweeps through `SweepController`,
never by calling `EphemeralAccount::sweep()` directly. See
[FAQ](./faq.md#so-how-should-i-sweep-funds-on-testnet).

**SweepCompleted**
Event (topic `sweep`) emitted by `SweepController` after it authorizes and marks
an account swept. Data includes `ephemeral_account`, `destination` and `amount`.
The controller does **not** move tokens itself; the SDK performs the custodial
payout via Horizon.

**SweepExecutedMulti**
Event (topic `swept_mul`) emitted by `ephemeral_account` after a multi-asset
sweep. Data includes `destination` and `payments`.

## T

**testnet reset**
A periodic reset of the Stellar public testnet that wipes all contracts and
state. Any stored contract ID can stop resolving after a reset, so verify IDs
on-chain before use. See [Overview](./overview.md#where-contract-ids-live-and-how-far-to-trust-them).

## U

**upgrade**
`EphemeralAccount::upgrade()` is an admin-gated function that replaces the
contract's WASM. Because it can change live behavior, correlate the on-chain
code hash with source as described in [Versioning](./versioning.md).

## W

**WASM hash**
The 32-byte SHA-256 hash of a compiled contract binary. The deployment record
stores `config.ephemeralAccountWasmHash`; comparing it with the on-chain code
hash tells you which source version is live. See
[Versioning](./versioning.md#step-4---verify-the-on-chain-code-hash).
