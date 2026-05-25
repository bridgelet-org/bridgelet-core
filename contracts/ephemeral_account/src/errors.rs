use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    PaymentAlreadyReceived = 3,
    InvalidAmount = 4,
    InvalidExpiry = 5,
    NotExpired = 6,
    AlreadySwept = 7,
    Unauthorized = 8,
    InvalidSignature = 9,
    NoPaymentReceived = 10,
    AccountExpired = 11,
    InvalidStatus = 12,
    DuplicateAsset = 13,
    TooManyPayments = 14,
}
