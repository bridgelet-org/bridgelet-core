use soroban_sdk::{contracttype, Address, Bytes, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Payment {
    pub asset: Address,
    pub amount: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq, Copy)]
#[repr(u32)]
pub enum AccountStatus {
    Active = 0,
    PaymentReceived = 1,
    Swept = 2,
    Expired = 3,
}

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

#[contracttype]
#[derive(Clone, Debug)]
pub struct AccountInitRequest {
    pub expiry_ledger: u32,
    pub recovery_address: Address,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct AccountInitResult {
    pub account_address: Address,
    pub success: bool,
    pub error: Option<Bytes>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SweepPayload {
    pub ephemeral_account: Address,
    pub destination: Address,
    pub nonce: u64,
    pub network_passphrase: Bytes,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetBalance {
    pub asset: Address,
    pub balance: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl ContractVersion {
    pub fn packed(&self) -> u32 {
        ((self.major & 0xFF) << 16) | ((self.minor & 0xFF) << 8) | (self.patch & 0xFF)
    }
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }
    pub fn initial() -> Self {
        Self::new(1, 0, 0)
    }
}
