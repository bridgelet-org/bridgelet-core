use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
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
}
