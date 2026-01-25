use soroban_sdk::{contracttype, BytesN, Env, Address};

/// Data keys for contract storage
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Authorized signer public key (BytesN<32> for Ed25519)
    AuthorizedSigner,
    /// Current sweep nonce to prevent replay attacks
    SweepNonce,
    /// Authorized destination address (optional, if set, sweeps can only go to this address)
    AuthorizedDestination,
    /// Creator address (the address that initialized the contract)
    Creator,
}

/// Set the authorized signer public key
///
/// # Arguments
/// * `env` - Soroban environment
/// * `signer` - Ed25519 public key (32 bytes)
pub fn set_authorized_signer(env: &Env, signer: &BytesN<32>) {
    env.storage().instance().set(&DataKey::AuthorizedSigner, signer);
}

/// Get the authorized signer public key
///
/// # Arguments
/// * `env` - Soroban environment
///
/// # Returns
/// The authorized signer's Ed25519 public key, or None if not set
pub fn get_authorized_signer(env: &Env) -> Option<BytesN<32>> {
    env.storage().instance().get(&DataKey::AuthorizedSigner)
}

/// Initialize the sweep nonce to 0
///
/// # Arguments
/// * `env` - Soroban environment
pub fn init_sweep_nonce(env: &Env) {
    env.storage().instance().set(&DataKey::SweepNonce, &0u64);
}

/// Get the current sweep nonce
///
/// # Arguments
/// * `env` - Soroban environment
///
/// # Returns
/// The current sweep nonce (incremented after each successful sweep)
pub fn get_sweep_nonce(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::SweepNonce)
        .unwrap_or(0u64)
}

/// Increment the sweep nonce after a successful authorization
///
/// # Arguments
/// * `env` - Soroban environment
pub fn increment_sweep_nonce(env: &Env) {
    let current_nonce = get_sweep_nonce(env);
    env.storage()
        .instance()
        .set(&DataKey::SweepNonce, &(current_nonce + 1));
}

/// Set the authorized destination address
///
/// # Arguments
/// * `env` - Soroban environment
/// * `destination` - Authorized destination address
pub fn set_authorized_destination(env: &Env, destination: &Address) {
    env.storage()
        .instance()
        .set(&DataKey::AuthorizedDestination, destination);
}

/// Get the authorized destination address
///
/// # Arguments
/// * `env` - Soroban environment
///
/// # Returns
/// The authorized destination address, or None if not set (flexible mode)
pub fn get_authorized_destination(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::AuthorizedDestination)
}

/// Check if an authorized destination is set
///
/// # Arguments
/// * `env` - Soroban environment
///
/// # Returns
/// true if authorized destination is set (locked mode), false otherwise (flexible mode)
pub fn has_authorized_destination(env: &Env) -> bool {
    env.storage()
        .instance()
        .has(&DataKey::AuthorizedDestination)
}

/// Set the creator address (the address that initialized the contract)
///
/// # Arguments
/// * `env` - Soroban environment
/// * `creator` - Creator address
pub fn set_creator(env: &Env, creator: &Address) {
    env.storage().instance().set(&DataKey::Creator, creator);
}

/// Get the creator address
///
/// # Arguments
/// * `env` - Soroban environment
///
/// # Returns
/// The creator address, or None if not set
pub fn get_creator(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::Creator)
}
