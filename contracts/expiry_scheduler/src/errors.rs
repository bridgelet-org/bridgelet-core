use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// The contract has already been initialized.
    AlreadyInitialized = 12000,
    /// A state-changing operation was attempted before initialization.
    NotInitialized = 12001,
    /// The caller is neither the registered account nor the admin.
    Unauthorized = 12002,
    /// The account has no scheduled expiry.
    NotRegistered = 12003,
}
