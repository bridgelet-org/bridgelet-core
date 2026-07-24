use soroban_sdk::contracterror;

// Issue #248: error codes are namespaced per contract to avoid ambiguity
// when a raw numeric code is surfaced without its originating contract ID.
// Namespace map (1000-wide blocks):
//   ephemeral_account -> 1000-1999
//   sweep_controller   -> 2000-2999
//   reserve_contract   -> 3000-3999
//   account_factory    -> 4000-4999 (reserved for future use)
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1000,
    NotInitialized = 1001,
    PaymentAlreadyReceived = 1002,
    InvalidAmount = 1003,
    InvalidExpiry = 1004,
    NotExpired = 1005,
    AlreadySwept = 1006,
    Unauthorized = 1007,
    InvalidSignature = 1008,
    NoPaymentReceived = 1009,
    AccountExpired = 1010,
    InvalidStatus = 1011,
    DuplicateAsset = 1012,
    TooManyPayments = 1013,
    NotUpgradeAdmin = 1014,
}
