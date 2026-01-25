use crate::errors::Error;
use crate::storage;
use soroban_sdk::{xdr::ToXdr, Address, Bytes, BytesN, Env};

/// Construct the message to be signed for sweep authorization
///
/// Message format: hash(destination + nonce + contract_id)
///
/// # Arguments
/// * `env` - Soroban environment
/// * `destination` - Destination wallet address
/// * `contract_id` - The sweep controller contract address
///
/// # Returns
/// BytesN<32> containing the hash of the message components
fn construct_sweep_message(
    env: &Env,
    destination: &Address,
    contract_id: &Address,
) -> BytesN<32> {
    // Get current nonce
    let nonce = storage::get_sweep_nonce(env);

    // Construct the message by concatenating:
    // - destination (serialized as bytes)
    // - nonce (as u64, 8 bytes)  
    // - contract_id (serialized as bytes)
    
    // Get XDR bytes for addresses
    let dest_bytes = destination.to_xdr(env);
    let contract_bytes = contract_id.to_xdr(env);
    
    // Build nonce bytes (big-endian u64) as BytesN<8> then convert to Bytes
    let nonce_array = [
        ((nonce >> 56) & 0xFF) as u8,
        ((nonce >> 48) & 0xFF) as u8,
        ((nonce >> 40) & 0xFF) as u8,
        ((nonce >> 32) & 0xFF) as u8,
        ((nonce >> 24) & 0xFF) as u8,
        ((nonce >> 16) & 0xFF) as u8,
        ((nonce >> 8) & 0xFF) as u8,
        (nonce & 0xFF) as u8,
    ];
    let nonce_bytes_n = BytesN::from_array(env, &nonce_array);
    let nonce_bytes: Bytes = nonce_bytes_n.into();
    
    // Build message by concatenating bytes
    let mut message = Bytes::new(env);
    
    // Copy bytes into message one by one
    let mut idx = 0u32;
    for i in 0..dest_bytes.len() {
        message.set(idx, dest_bytes.get(i).unwrap());
        idx += 1;
    }
    for i in 0..nonce_bytes.len() {
        message.set(idx, nonce_bytes.get(i).unwrap());
        idx += 1;
    }
    for i in 0..contract_bytes.len() {
        message.set(idx, contract_bytes.get(i).unwrap());
        idx += 1;
    }

    // Hash the message using SHA256 and convert to BytesN<32>
    env.crypto().sha256(&message).into()
}

/// Verify sweep authorization signature using Ed25519
///
/// This function verifies that the provided signature was created by the authorized signer
/// using the private key corresponding to the authorized public key.
///
/// # Arguments
/// * `env` - Soroban environment
/// * `account` - Ephemeral account address (used as context)
/// * `destination` - Destination wallet address
/// * `signature` - Ed25519 signature (64 bytes)
///
/// # Returns
/// Ok(()) if signature is valid, Error otherwise
pub fn verify_sweep_auth(
    env: &Env,
    _account: &Address,
    destination: &Address,
    signature: &BytesN<64>,
) -> Result<(), Error> {
    // Get the authorized signer public key from storage
    let authorized_signer = storage::get_authorized_signer(env)
        .ok_or(Error::AuthorizedSignerNotSet)?;

    // Get the sweep controller contract address
    let contract_id = env.current_contract_address();

    // Construct the message that should have been signed
    let message = construct_sweep_message(env, destination, &contract_id);

    // Verify the Ed25519 signature
    // ed25519_verify expects:
    // - &BytesN<32> for public key
    // - &Bytes for message (the hash)
    // - &BytesN<64> for signature
    // Convert message from BytesN<32> to Bytes
    let message_bytes: Bytes = message.into();
    
    // ed25519_verify returns () and panics on failure
    // In Soroban, panics are caught by the execution environment
    // We'll call it directly - if it panics, the contract execution will fail
    env.crypto().ed25519_verify(&authorized_signer, &message_bytes, signature);
    
    Ok(())
}

/// Increment the nonce after successful authorization
///
/// This should be called after successful verification to prevent replay attacks.
///
/// # Arguments
/// * `env` - Soroban environment
pub fn increment_nonce(env: &Env) {
    storage::increment_sweep_nonce(env);
}

/// Authorization context for sweep operations
pub struct AuthContext {
    pub account: Address,
    pub destination: Address,
    pub signature: BytesN<64>,
}

impl AuthContext {
    pub fn new(account: Address, destination: Address, signature: BytesN<64>) -> Self {
        Self {
            account,
            destination,
            signature,
        }
    }

    pub fn verify(&self, env: &Env) -> Result<(), Error> {
        verify_sweep_auth(env, &self.account, &self.destination, &self.signature)
    }
}
