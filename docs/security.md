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
*   **Mechanism**: Ed25519 Signatures + Soroban Auth
*   **Flow**:
    1.  Off-chain SDK generates a signature covering `hash(destination + nonce + contract_id)`.
    2.  Caller invokes `SweepController.execute_sweep`.
    3.  `SweepController` verifies the Ed25519 signature against the stored `authorized_signer`.
    4.  `SweepController` calls `EphemeralAccount.sweep`.
    5.  `EphemeralAccount` validates its internal state and transitions to `Swept`.

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
2.  **Token Transfers**: The actual logic to move tokens (calling `token.transfer`) is currently **commented out** or not fully integrated in `SweepController`. The contracts currently manage *state* and *authorization* but do not yet move funds.

### Other Limitations
*   **Asset Limit**: The `EphemeralAccount` supports recording up to 10 distinct assets.
*   **Gas Management**: Users/Integrators are responsible for providing sufficient gas for sweep operations.
*   **Trust Assumption**: The system assumes the `authorized_signer` private key is kept secure off-chain.

## Best Practices for Integrators

1.  **Use SweepController**: Always use `SweepController::execute_sweep` to perform sweeps. Never call `EphemeralAccount::sweep` directly, as it currently lacks active signature verification.
2.  **Verify Expiry**: When creating accounts, ensure `expiry_ledger` provides enough buffer for network latency and confirmation times.
3.  **Monitor Events**: Listen for `AccountCreated` and `PaymentReceived` events to trigger off-chain workflows.
4.  **Key Management**: Securely manage the Ed25519 private key used for generating sweep signatures. Use a hardware security module (HSM) or secure enclave if possible.
5.  **Recovery**: Monitor for expired accounts and trigger `expire()` to reclaim funds to the recovery address.
