use soroban_sdk::contracterror;

// Namespace map (1000-wide blocks):
//   fee_sponsor        -> 7000-7999
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 7000,
    NotInitialized = 7001,
    InvalidAmount = 7002,
    InsufficientDeposit = 7003,
    ExceedsCap = 7004,
    InsufficientRemaining = 7005,
    NotSponsor = 7006,
    NotAuthorized = 7007,
    AccountNotSponsored = 7008,
    AlreadySponsored = 7009,
}
