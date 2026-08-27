use soroban_sdk::contracterror;

// Issue #248: error codes are namespaced per contract. See
// contracts/ephemeral_account/src/errors.rs for the full namespace map.
// This contract owns 2000-2999. Note: 2011 is intentionally skipped here,
// preserving the original enum's internal gap.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    /// The provided account address is not a valid contract or does not exist.
    InvalidAccount = 2000,
    /// A SEP-41 token transfer failed during sweep execution.
    TransferFailed = 2001,
    /// Signature verification failed — the signature does not match the
    /// authorized signer's public key for the given message.
    ///
    /// NOTE: Despite its name, this variant is currently only reachable
    /// for high-level auth logic errors (e.g. "not yet initialized" /
    /// "no creator set").  True Ed25519 signature failures panic via
    /// `env.crypto().ed25519_verify()` rather than returning this
    /// variant.  Retained for backward compatibility with off-chain
    /// error-code consumers. (#412)
    AuthorizationFailed = 2002,
    /// The source account does not hold sufficient balance of the token
    /// being transferred.
    InsufficientBalance = 2003,
    /// The ephemeral account is not in a state that permits sweeping
    /// (e.g., no payment has been recorded yet).
    AccountNotReady = 2004,
    /// The ephemeral account has passed its expiry ledger and can no
    /// longer be swept via the normal path.
    AccountExpired = 2005,
    /// A sweep has already been executed for this account.  Replay of
    /// sweep is forbidden.
    AccountAlreadySwept = 2006,
    /// The Ed25519 signature provided does not match the expected format
    /// or length.
    InvalidSignature = 2007,
    /// The cryptographic signature verification primitive returned a
    /// failure (distinct from `AuthorizationFailed` which covers
    /// higher-level auth logic errors).
    ///
    /// NOTE: Currently unreachable — `env.crypto().ed25519_verify()`
    /// panics on failure rather than returning a Result.  This variant
    /// is retained for forward compatibility if a non-panicking verify
    /// API becomes available. (#412)
    SignatureVerificationFailed = 2008,
    /// No authorized signer has been configured on this SweepController
    /// instance.  `initialize()` must be called first.
    AuthorizedSignerNotSet = 2009,
    /// The provided nonce does not match the expected on-chain nonce,
    /// indicating a stale or replayed signature.
    ///
    /// NOTE: Currently unreachable — the nonce is embedded in the signed
    /// message and verified implicitly by Ed25519 signature verification,
    /// which panics on mismatch rather than returning this variant.
    /// Retained for forward compatibility. (#412)
    InvalidNonce = 2010,
    // NOTE: discriminant 2011 is intentionally unused.  It was removed
    // during a previous refactor and the gap is preserved to avoid
    // breaking any off-chain tooling that references the exact numeric
    // codes above and below. (#413)
    /// The destination address does not match the authorized destination
    /// configured for this controller (locked mode).
    UnauthorizedDestination = 2012,
    /// The caller is not the contract admin / creator and cannot perform
    /// this privileged operation.
    NotAdmin = 2013,
    /// An arithmetic overflow or underflow was detected during amount
    /// calculation.
    Overflow = 2014,
    /// The fee estimation input is invalid (e.g., zero amount or unknown
    /// asset).
    InvalidEstimateInput = 2015,
    /// The signer update time-lock has not yet elapsed.  The new signer
    /// cannot take effect until the required number of ledgers have passed.
    TimeLockNotElapsed = 2016,
    /// No pending signer update exists to be applied.
    NoPendingSignerUpdate = 2017,
    /// The contract has not been initialized yet.
    NotInitialized = 2018,
}
