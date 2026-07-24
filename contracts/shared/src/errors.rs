use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum SharedError {
    NotInitialized = 50,
    AlreadyInitialized = 51,
    Unauthorized = 52,
    Expired = 53,
    NotExpired = 54,
    Overflow = 55,
    InvalidAmount = 56,
}
