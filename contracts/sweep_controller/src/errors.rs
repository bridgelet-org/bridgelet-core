use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    InvalidAccount = 1,
    AuthorizationFailed = 2,
    // discriminant 3 previously held TransferFailed (removed — see #233)
    InsufficientBalance = 4,
    AccountNotReady = 5,
    AccountExpired = 6,
    AccountAlreadySwept = 7,
    InvalidSignature = 8,
    SignatureVerificationFailed = 9,
    AuthorizedSignerNotSet = 10,
    InvalidNonce = 11,
    // discriminant 12 intentionally unused
    UnauthorizedDestination = 13,
}
