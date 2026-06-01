use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    AuthorizedSignerNotSet = 3,
    InvalidSignature = 4,
    SignatureVerificationFailed = 5,
}
