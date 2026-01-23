use soroban_sdk::{contracttype, Address, Env, Map, Vec};

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Payment {
    pub asset: Address,
    pub amount: i128,
    pub timestamp: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[contracttype]
pub enum AccountStatus {
    Active = 0,
    PaymentReceived = 1,
    Swept = 2,
    Expired = 3,
}

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
    Payments,
    Status,
    SweptTo,
    BaseReserve,  // New: Track base reserve amount
    ReserveReclaimed,  // New: Track if reserve was reclaimed
}

/// Payment record for tracking individual asset payments
#[derive(Clone)]
#[contracttype]
pub struct Payment {
    pub asset: Address,
    pub amount: i128,
    pub timestamp: u64,
}

const MAX_ASSETS: u32 = 10;

// Base reserve constants (in stroops: 1 XLM = 10,000,000 stroops)
pub const BASE_RESERVE_PER_ENTRY: i128 = 5_000_000; // 0.5 XLM
pub const ACCOUNT_BASE_RESERVE: i128 = 10_000_000; // 1 XLM (2 * 0.5 XLM base reserve)
pub const MIN_BALANCE_FOR_CLOSE: i128 = 1_000_000; // 0.1 XLM for final transaction


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

// Base reserve functions
pub fn set_base_reserve(env: &Env, amount: i128) {
    env.storage()
        .instance()
        .set(&DataKey::BaseReserve, &amount);
}

pub fn get_base_reserve(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::BaseReserve)
        .unwrap_or(0)
}

pub fn set_reserve_reclaimed(env: &Env, reclaimed: bool) {
    env.storage()
        .instance()
        .set(&DataKey::ReserveReclaimed, &reclaimed);
}

pub fn is_reserve_reclaimed(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&DataKey::ReserveReclaimed)
        .unwrap_or(false)
}

/// Calculate base reserve needed for account
/// Account base (1 XLM) + trustlines (0.5 XLM each)
pub fn calculate_base_reserve(num_trustlines: u32) -> i128 {
    ACCOUNT_BASE_RESERVE + (BASE_RESERVE_PER_ENTRY * num_trustlines as i128)
}
