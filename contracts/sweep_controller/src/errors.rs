use soroban_sdk::contracterror;

// Issue #248: error codes are namespaced per contract. See
// contracts/ephemeral_account/src/errors.rs for the full namespace map.
// This contract owns 2000-2999. Note: 2011 is intentionally skipped here,
// preserving the original enum's internal gap.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    InvalidAccount = 2000,
    TransferFailed = 2001,
    AuthorizationFailed = 2002,
    InsufficientBalance = 2003,
    AccountNotReady = 2004,
    AccountExpired = 2005,
    AccountAlreadySwept = 2006,
    InvalidSignature = 2007,
    SignatureVerificationFailed = 2008,
    AuthorizedSignerNotSet = 2009,
    InvalidNonce = 2010,
    UnauthorizedDestination = 2012,
}
