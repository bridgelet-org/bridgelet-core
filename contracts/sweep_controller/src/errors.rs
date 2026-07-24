use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    /// The provided account address is not a valid contract or does not exist.
    InvalidAccount = 1,
    /// A SEP-41 token transfer failed during sweep execution.
    TransferFailed = 2,
    /// Signature verification failed — the signature does not match the
    /// authorized signer's public key for the given message.
    AuthorizationFailed = 3,
    /// The source account does not hold sufficient balance of the token
    /// being transferred.
    InsufficientBalance = 4,
    /// The ephemeral account is not in a state that permits sweeping
    /// (e.g., no payment has been recorded yet).
    AccountNotReady = 5,
    /// The ephemeral account has passed its expiry ledger and can no
    /// longer be swept via the normal path.
    AccountExpired = 6,
    /// A sweep has already been executed for this account.  Replay of
    /// sweep is forbidden.
    AccountAlreadySwept = 7,
    /// The Ed25519 signature provided does not match the expected format
    /// or length.
    InvalidSignature = 8,
    /// The cryptographic signature verification primitive returned a
    /// failure (distinct from `AuthorizationFailed` which covers
    /// higher-level auth logic errors).
    SignatureVerificationFailed = 9,
    /// No authorized signer has been configured on this SweepController
    /// instance.  `initialize()` must be called first.
    AuthorizedSignerNotSet = 10,
    /// The provided nonce does not match the expected on-chain nonce,
    /// indicating a stale or replayed signature.
    InvalidNonce = 11,
    /// The destination address does not match the authorized destination
    /// configured for this controller (locked mode).
    UnauthorizedDestination = 13,
    /// The caller is not the contract admin / creator and cannot perform
    /// this privileged operation.
    NotAdmin = 14,
    /// An arithmetic overflow or underflow was detected during amount
    /// calculation.
    Overflow = 15,
    /// The fee estimation input is invalid (e.g., zero amount or unknown
    /// asset).
    InvalidEstimateInput = 16,
    /// The signer update time-lock has not yet elapsed.  The new signer
    /// cannot take effect until the required number of ledgers have passed.
    TimeLockNotElapsed = 17,
    /// No pending signer update exists to be applied.
    NoPendingSignerUpdate = 18,
    /// The contract has not been initialized yet.
    NotInitialized = 19,
}
