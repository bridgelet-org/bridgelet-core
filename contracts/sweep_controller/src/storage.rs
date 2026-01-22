use soroban_sdk::{BytesN, Env, Address};

/// Data keys for contract storage
#[derive(Clone)]
pub enum DataKey {
    /// Authorized signer public key (BytesN<32> for Ed25519)
    AuthorizedSigner,
    /// Current sweep nonce to prevent replay attacks
    SweepNonce,
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
