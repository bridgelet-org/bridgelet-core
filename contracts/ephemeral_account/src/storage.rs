use soroban_sdk::{contracttype, Address, Env};

#[derive(Clone, Copy, PartialEq, Eq)]
#[contracttype]
pub enum AccountStatus {
    Active = 0,
    PaymentReceived = 1,
    Swept = 2,
    Expired = 3,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Initialized,
    Creator,
    ExpiryLedger,
    RecoveryAddress,
    PaymentReceived,
    PaymentAmount,
    PaymentAsset,
    Status,
    SweptTo,
}

// Initialization
pub fn is_initialized(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Initialized)
}

pub fn set_initialized(env: &Env, value: bool) {
    env.storage().instance().set(&DataKey::Initialized, &value);
}

// Creator
pub fn set_creator(env: &Env, creator: &Address) {
    env.storage().instance().set(&DataKey::Creator, creator);
}

pub fn get_creator(env: &Env) -> Address {
    env.storage().instance().get(&DataKey::Creator).unwrap()
}

// Expiry ledger
pub fn set_expiry_ledger(env: &Env, ledger: u32) {
    env.storage()
        .instance()
        .set(&DataKey::ExpiryLedger, &ledger);
}

pub fn get_expiry_ledger(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::ExpiryLedger)
        .unwrap()
}

// Recovery address
pub fn set_recovery_address(env: &Env, address: &Address) {
    env.storage()
        .instance()
        .set(&DataKey::RecoveryAddress, address);
}

pub fn get_recovery_address(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::RecoveryAddress)
        .unwrap()
}

// Payment
pub fn has_payment_received(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::PaymentReceived)
}

pub fn set_payment_received(env: &Env, value: bool) {
    env.storage()
        .instance()
        .set(&DataKey::PaymentReceived, &value);
}

pub fn set_payment_amount(env: &Env, amount: i128) {
    env.storage()
        .instance()
        .set(&DataKey::PaymentAmount, &amount);
}

pub fn get_payment_amount(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::PaymentAmount)
        .unwrap_or(0)
}

pub fn set_payment_asset(env: &Env, asset: &Address) {
    env.storage().instance().set(&DataKey::PaymentAsset, asset);
}

pub fn get_payment_asset(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::PaymentAsset)
        .unwrap()
}

// Status
pub fn set_status(env: &Env, status: AccountStatus) {
    env.storage().instance().set(&DataKey::Status, &status);
}

pub fn get_status(env: &Env) -> AccountStatus {
    env.storage()
        .instance()
        .get(&DataKey::Status)
        .unwrap_or(AccountStatus::Active)
}

// Swept to
pub fn set_swept_to(env: &Env, address: &Address) {
    env.storage().instance().set(&DataKey::SweptTo, address);
}

pub fn get_swept_to(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::SweptTo)
}
