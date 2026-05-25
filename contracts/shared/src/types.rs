use soroban_sdk::{contracttype, Address, Vec};

// Represents a payment received by the ephemeral account.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Payment {
    pub asset: Address,
    pub amount: i128,
    pub timestamp: u64,
}
// The current status of an ephemeral account.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq, Copy)]
#[repr(u32)]
pub enum AccountStatus {
    Active = 0,
    PaymentReceived = 1,
    Swept = 2,
    Expired = 3,
}

/// Account information structure
#[derive(Clone)]
#[contracttype]
pub struct AccountInfo {
    pub creator: Address,
    pub status: AccountStatus,
    pub expiry_ledger: u32,
    pub recovery_address: Address,
    pub payment_received: bool,
    pub payment_count: u32,
    pub payments: Vec<Payment>,
    pub swept_to: Option<Address>,
}
