use bridgelet_shared::{AccountStatus, Payment};
use soroban_sdk::{contracttype, Address, Env, Map};

#[contracttype]
pub enum DataKey {
    Initialized,
    Creator,
    ExpiryLedger,
    RecoveryAddress,
    Payments,
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

// Expiry
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

// Payments
pub fn has_payments(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Payments)
}

pub fn get_all_payments(env: &Env) -> Map<Address, Payment> {
    env.storage()
        .instance()
        .get(&DataKey::Payments)
        .unwrap_or_else(|| Map::new(env))
}

pub fn set_all_payments(env: &Env, payments: &Map<Address, Payment>) {
    env.storage().instance().set(&DataKey::Payments, payments);
}

pub fn add_payment(env: &Env, payment: Payment) {
    let mut payments = get_all_payments(env);
    payments.set(payment.asset.clone(), payment);
    set_all_payments(env, &payments);
}

pub fn get_payment(env: &Env, asset: &Address) -> Option<Payment> {
    let payments = get_all_payments(env);
    payments.get(asset.clone())
}

pub fn get_total_payments(env: &Env) -> u32 {
    get_all_payments(env).len()
}

pub fn has_payment_received(env: &Env) -> bool {
    has_payments(env)
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
