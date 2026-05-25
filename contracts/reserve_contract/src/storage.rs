use soroban_sdk::{contracttype, Address, Env};

/// Storage keys used by the reserve contract.
///
/// Each variant maps to a distinct slot in Soroban's instance storage,
/// ensuring keys never collide with each other.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// The configured base reserve amount, expressed in stroops.
    ///
    /// One XLM equals 10,000,000 stroops.  Storing the value as stroops
    /// avoids floating-point arithmetic inside the contract.
    BaseReserve,

    /// The admin address that is authorised to update the base reserve.
    ///
    /// Set once during [`ReserveContract::initialize`] and immutable
    /// afterwards.
    Admin,
}

// Base Reserve helpers

/// Persist the base reserve amount (in stroops) to contract storage.
///
/// Calling this function a second time silently overwrites the previous
/// value – callers are responsible for validating the amount before
/// invoking this function.
///
/// # Arguments
/// * `env`    – Soroban environment handle.
/// * `amount` – Base reserve in stroops.  Must already be validated as
///              positive by the caller.
pub fn set_base_reserve(env: &Env, amount: i128) {
    env.storage()
        .instance()
        .set(&DataKey::BaseReserve, &amount);
}

/// Read the base reserve amount from contract storage.
///
/// # Returns
/// * `Some(amount)` – the value previously stored via [`set_base_reserve`].
/// * `None`         – the base reserve has never been configured.
pub fn get_base_reserve(env: &Env) -> Option<i128> {
    env.storage().instance().get(&DataKey::BaseReserve)
}

/// Returns `true` if a base reserve has been stored, `false` otherwise.
///
/// Cheaper than calling [`get_base_reserve`] when only the presence of the
/// key matters, not its value.
pub fn has_base_reserve(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::BaseReserve)
}

// Admin helpers

/// Store the admin address.  Intended to be called exactly once during
/// contract initialization.
pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

/// Read the admin address, if set.
pub fn get_admin(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::Admin)
}

/// Returns `true` if an admin has been configured (i.e. contract is initialized).
pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Admin)
}

// TTL management

/// If the remaining TTL drops below this threshold (in ledgers), extend it.
/// ~100 ledgers ≈ ~8 minutes — gives a comfortable buffer.
const INSTANCE_TTL_THRESHOLD: u32 = 100;

/// Extend the instance TTL to this many ledgers.
/// 518 400 ledgers ≈ 30 days (at ~5 s per ledger).
const INSTANCE_TTL_EXTEND_TO: u32 = 518_400;

/// Proactively extend the instance storage TTL so the contract (and all
/// its instance-stored data) does not get archived during periods of
/// inactivity.
///
/// Should be called from **every** public entry-point (reads included)
/// to guarantee the data stays alive as long as anyone interacts with
/// the contract.
pub fn extend_instance_ttl(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
}
