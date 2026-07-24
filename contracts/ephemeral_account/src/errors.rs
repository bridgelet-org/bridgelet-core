use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    InvalidAmount = 3,
    InvalidExpiry = 4,
    NotExpired = 5,
    AlreadySwept = 6,
    Unauthorized = 7,
    NoPaymentReceived = 8,
    AccountExpired = 9,
    InvalidStatus = 10,
    DuplicateAsset = 11,
    TooManyPayments = 12,
    NotUpgradeAdmin = 13,
}
