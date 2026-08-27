use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 10000,
    NotInitialized = 10001,
    Unauthorized = 10002,
    CannotRevokeSuperAdmin = 10003,
}