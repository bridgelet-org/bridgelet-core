# Security Model

This document outlines the security considerations, threat model, authorization mechanisms, and best practices for the Bridgelet Core system.

## Threat Model

The Bridgelet Core system is designed to operate in a trust-minimized environment. The following threat vectors have been considered and mitigated:

### 1. Unauthorized Sweeping
*   **Threat**: An attacker attempts to sweep funds from an active Ephemeral Account to their own wallet.
*   **Mitigation**:
    *   **Authorization Signatures**: Sweeping requires a valid Ed25519 signature from an authorized signer. The `SweepController` contract verifies this signature before allowing a sweep.
    *   **Destination Locking**: The `SweepController` can be initialized with a locked `authorized_destination`. If set, funds can *only* be swept to that specific address, regardless of the signer.
    *   **Nonce Protection**: Each sweep operation requires a unique nonce to prevent replay attacks.

### 2. Double Spending / Replay Attacks
*   **Threat**: An attacker attempts to sweep the same account multiple times or replay a valid sweep signature.
*   **Mitigation**:
    *   **State Machine**: The `EphemeralAccount` contract enforces a strict state machine (`Active` -> `PaymentReceived` -> `Swept` | `Expired`). Once the state transitions to `Swept`, subsequent calls fail with `Error::AlreadySwept`.
    *   **Nonces**: The `SweepController` maintains a monotonically increasing nonce. Every signed sweep request must include the current nonce, and the nonce is incremented upon success.

### 3. Expiration Bypass
*   **Threat**: An attacker attempts to keep funds in an ephemeral account indefinitely or sweep them after they should have expired.
*   **Mitigation**:
    *   **Ledger-based Expiration**: Expiration is tied to the Stellar ledger sequence number, providing an objective time source.
    *   **Guard Clauses**: The `sweep` function explicitly checks `is_expired()` and fails if the account has passed its expiry ledger.
    *   **Recovery Mechanism**: After expiration, the `expire()` function allows funds to be recovered to a pre-defined `recovery_address`, preventing funds from being permanently locked.

### 4. Malicious Account Initialization
*   **Threat**: An attacker initializes an account with a past expiry or invalid parameters.
*   **Mitigation**:
    *   **Initialization Checks**: The `initialize` function validates that `expiry_ledger` is in the future.
    *   **One-time Initialization**: The `Initialized` flag prevents re-initialization of an existing contract.

## Authorization Model

Bridgelet Core uses a layered authorization model:

### 1. Contract Initialization
*   **Mechanism**: `require_auth()`
*   **Scope**: The creator address must authorize the initialization of an `EphemeralAccount`.

### 2. Sweep Operations

The system supports two sweep paths, both routed through `SweepController`:

#### 2a. `execute_sweep` — Ed25519 Signature Path
*   **Mechanism**: Ed25519 Signatures + Soroban Auth
*   **Flow**:
    1.  Off-chain SDK generates a signature covering `hash(destination + nonce + contract_id)`.
    2.  Caller invokes `SweepController::execute_sweep`.
    3.  `SweepController` verifies the Ed25519 signature against the stored `authorized_signer`.
    4.  `SweepController` increments the nonce to prevent replay.
    5.  `SweepController` authorizes itself as the invoker of `EphemeralAccount::sweep`.
    6.  `EphemeralAccount::sweep` validates its internal state, transitions to `Swept`, and reclaims the base reserve.
*   **When to use**: When the off-chain signer is available to produce a signature. Suitable for automated sweep pipelines.

#### 2b. `claim` — Soroban Auth Path
*   **Mechanism**: Soroban Authorization Entries
*   **Flow**:
    1.  The recipient signs a Soroban auth entry for `SweepController::claim`.
    2.  Caller (or a relayer) invokes `SweepController::claim(recipient, ephemeral_account)`.
    3.  `SweepController` validates the destination matches the locked destination (if set).
    4.  `SweepController` authorizes itself as the invoker of `EphemeralAccount::sweep_claim`.
    5.  `EphemeralAccount::sweep_claim` validates state, transitions to `Swept`, and reclaims the base reserve.
*   **When to use**: When the recipient is available to sign a Soroban auth entry directly. Suitable for SDK/integration-driven claims where no off-chain signer is needed.

### 2a. Claim Operations
*   **Mechanism**: Soroban Auth (dual authorization)
*   **Flow**:
    1.  Caller invokes `SweepController.claim` with `recipient` and `ephemeral_account`.
    2.  `recipient.require_auth()` ensures the recipient authorizes the claim.
    3.  `SweepController` builds a Soroban auth entry authorizing itself as the invoker of `sweep_claim` on the ephemeral account (`authorize_as_current_contract`).
    4.  `EphemeralAccount.sweep_claim` is called, which verifies the Soroban auth entries and transitions the account to `Swept`.
*   **Note**: This path does not use Ed25519 signatures. Instead, both the recipient and the controller contract must provide Soroban authorization, enabling a relayer/SDK to submit the transaction while the recipient only signs the authorization payload.

### 3. Expiration
*   **Mechanism**: Public (Permissionless)
*   **Scope**: Once the expiry ledger is reached, *anyone* can call `expire()` to return funds to the recovery address. This ensures funds are never stuck due to a missing signer.

## Reentrancy Protection

The system employs the Checks-Effects-Interactions pattern and leverages Soroban's execution model:

1.  **State Updates First**: In `EphemeralAccount::sweep`, the status is updated to `Swept` *before* any external calls or event emissions would typically occur (though currently, no external calls are made).
2.  **Atomic Execution**: Soroban contract invocations are atomic. If any part of the sweep operation fails (e.g., signature verification), the entire transaction reverts, ensuring no partial state changes.
3.  **Single-Threaded**: Soroban executes transactions sequentially for a given contract instance, preventing race conditions.

## Security Guarantees

*   **Single-Use**: An account can effectively be swept only once.
*   **Time-Bounded**: Funds are guaranteed to be either sweepable or recoverable after the expiry ledger.
*   **Auditability**: All critical state transitions (`Created`, `PaymentReceived`, `Swept`, `Expired`) emit on-chain events.

## Known Limitations and Assumptions

### Critical Implementation Gaps (Current Version)
1.  **EphemeralAccount Signature Verification**: The `verify_sweep_authorization` function in `EphemeralAccount` is currently a placeholder ("TODO"). **Do not rely on `EphemeralAccount::sweep` directly for security.** Always route sweeps through `SweepController`, which implements proper Ed25519 verification.
2.  **Token Transfers**: `SweepController` invokes `transfers::execute_transfers` to move tokens from the ephemeral account to the destination after a successful sweep. The transfer logic is fully integrated and active.
1.  **EphemeralAccount Signature Verification**: The `verify_sweep_authorization` function in `EphemeralAccount` is currently a placeholder. It only checks that the caller is the authorized controller — it does **not** verify the Ed25519 signature bytes. **Do not rely on `EphemeralAccount::sweep` directly for security.** Always route sweeps through `SweepController`, which implements proper Ed25519 verification via `execute_sweep`, or through the `claim` path which uses Soroban auth instead of off-chain signatures.

### Other Limitations
*   **Asset Limit**: The `EphemeralAccount` supports recording up to 10 distinct assets.
*   **Gas Management**: Users/Integrators are responsible for providing sufficient gas for sweep operations.
*   **Trust Assumption**: The system assumes the `authorized_signer` private key is kept secure off-chain.

## Best Practices for Integrators

1.  **Use SweepController**: Always use `SweepController` to perform sweeps — either via `execute_sweep` (Ed25519 signature path) or `claim` (Soroban auth path). Never call `EphemeralAccount::sweep` or `EphemeralAccount::sweep_claim` directly.
2.  **Choose the Right Path**: Use `execute_sweep` when you have an off-chain signer producing Ed25519 signatures. Use `claim` when the recipient can sign a Soroban auth entry directly (e.g., via SDK or wallet integration).
3.  **Verify Expiry**: When creating accounts, ensure `expiry_ledger` provides enough buffer for network latency and confirmation times.
4.  **Monitor Events**: Listen for `AccountCreated`, `PaymentReceived`, and `SweepCompleted` events to trigger off-chain workflows.
5.  **Key Management**: Securely manage the Ed25519 private key used for generating sweep signatures. Use a hardware security module (HSM) or secure enclave if possible.
6.  **Recovery**: Monitor for expired accounts and trigger `expire()` to reclaim funds to the recovery address.
