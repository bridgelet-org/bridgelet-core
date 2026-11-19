# Mobile Integration Notes (Preliminary / Placeholder)

> **Status: placeholder.** No mobile app in the Bridgelet organization integrates
> with these contracts today, and nothing in this repository references one. These
> notes list the mobile-specific questions a future integration will hit so it
> isn't a blank slate. Nothing here has been built or tested, and the mobile
> flows are **proposals**, not an existing design.

## Where mobile touches the contracts

The recipient-facing path is the **gas-free `claim()`** flow
(`testnet/config/gas-free-claim-testing.md`,
`testnet/examples/gas-free-claim-walkthrough.md`): the recipient **signs a
Soroban authorization entry** for `SweepController::claim(recipient,
ephemeral_account)`, while a relayer (not the phone) submits the transaction and
pays fees. That is the natural fit for mobile: the phone only needs to *sign*,
not hold XLM for fees or talk to the network directly.

## Considerations

### 1. Deep-linking a claim flow

- A claim link needs to carry (or let the app resolve) the **ephemeral account
  ID**, the **network** (testnet vs mainnet), and optionally the expected
  destination. Use an HTTPS app link (universal link / Android App Link) rather
  than a custom scheme for integrity and fallback to a web page.
- **Never put secret keys, signing seeds or signed payloads in a URL.** URLs end up
  in logs, history and screenshots.
- The app should re-validate everything the link claims against the chain:
  `get_status`, `is_expired`, and `can_sweep` (`testnet/examples/checking-account-status.md`)
  before asking the user to sign.
- Handle the **expired** and **already swept** states with clear user messages
  (`AccountExpired`, `AlreadySwept`; see `testnet/examples/error-handling.md`).

### 2. Wallet-app handoff for signing

- If signing happens in a separate wallet app, the handoff must carry only the
  **payload to sign** and return only the **signature** (the auth entry
  signature), not the key.
- Show the user what they approve: destination (`recipient`), the account and
  assets/amounts from `get_info`.
- Soroban auth entries have an **expiration ledger**; a handoff that takes too long
  makes the signature unusable. Plan for refresh/re-sign.
- The signature covers the **network passphrase**; make sure the wallet is on the
  same network (`testnet/runbooks/wrong-network-passphrase.md`).

### 3. Relayer interaction

- The app sends the signed auth entry to a relayer service you control; the relayer
  builds, fees and submits (`testnet/runbooks/relayer-submission-failures.md`).
- Show progress by transaction status and events; don't rely on the phone
  polling Horizon forever (`testnet/docs/webhook-vs-polling-tradeoffs.md`).
- Tolerate retries and duplicate taps: make the request idempotent
  (`testnet/integration/idempotency-key-recommendation.md`).

### 4. Connectivity and lifecycle

- Mobile networks drop. Persist the in-flight claim state locally and reconcile
  against chain state on resume.
- Handle backgrounding mid-signature and mid-submission.
- Prefer push notifications from your backend for completion over long polling.

### 5. Security on device

- Keep keys in the platform keystore/secure enclave; never log signing material.
- Use separate keys per network; never reuse a mainnet key for testnet
  (`testnet/security/testnet-vs-mainnet-key-reuse-warning.md`).
- Do not rely on the app to enforce authorization: the contract and controller
  are the enforcement (`testnet/security/authorized-controller-confusion.md`).

### 6. Testnet specifics

- Testnet resets can invalidate links and accounts at any time
  (`testnet/policies/reset-policy.md`); label testnet builds clearly and
  don't ship them to production users.
- Contracts may not be deployed/verified at any given time
  (`testnet/registry/contract-status.md`).

## Open questions for the mobile team

- Which wallet(s) or SDKs will sign the Soroban auth entry?
- Link format and where it is resolved?
- Who operates the relayer and its funding?
- Required UX when the account is expired/swept?

## Out of scope

Any mobile code, SDK choice or design; those belong to the mobile repository.
