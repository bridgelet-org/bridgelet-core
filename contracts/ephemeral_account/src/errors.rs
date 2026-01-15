use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    PaymentAlreadyReceived = 3,
    Unauthorized = 4,
    AlreadySwept = 5,
    NotExpired = 6,
    InvalidDestination = 7,
    InvalidAmount = 8,
    InvalidExpiry = 9,
    NoPaymentReceived = 10,
    AccountExpired = 11,
    InvalidStatus = 12,
}
