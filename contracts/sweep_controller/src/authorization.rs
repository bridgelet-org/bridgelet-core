use crate::errors::Error;
use crate::storage;
use soroban_sdk::{xdr::ToXdr, Address, BytesN, Env};

/// Ed25519 public key length in bytes.
const ED25519_PUBLIC_KEY_LEN: usize = 32;
/// Ed25519 signature length in bytes.
const ED25519_SIGNATURE_LEN: usize = 64;

/// Construct the message to be signed for sweep authorization.
///
/// Message format: SHA256(destination_xdr || nonce_be64 || contract_id_xdr)
///
/// The off-chain signer must produce an Ed25519 signature over this exact
/// byte sequence.  Changing any component produces a different hash.
///
/// # Arguments
/// * `env` - Soroban environment
/// * `destination` - Destination wallet address
/// * `contract_id` - The sweep controller contract address
///
/// # Returns
/// BytesN<32> containing the SHA-256 hash of the message components
fn construct_sweep_message(env: &Env, destination: &Address, contract_id: &Address) -> BytesN<32> {
    let nonce = storage::get_sweep_nonce(env);

    let mut message = soroban_sdk::Bytes::new(env);

    // Destination address (XDR-serialized for canonical representation)
    let dest_bytes = destination.to_xdr(env);
    message.append(&dest_bytes);

    // Nonce as 8-byte big-endian integer
    message.push_back(((nonce >> 56) & 0xFF) as u8);
    message.push_back(((nonce >> 48) & 0xFF) as u8);
    message.push_back(((nonce >> 40) & 0xFF) as u8);
    message.push_back(((nonce >> 32) & 0xFF) as u8);
    message.push_back(((nonce >> 24) & 0xFF) as u8);
    message.push_back(((nonce >> 16) & 0xFF) as u8);
    message.push_back(((nonce >> 8) & 0xFF) as u8);
    message.push_back((nonce & 0xFF) as u8);

    // Sweep controller contract address (binds signature to this instance)
    let contract_bytes = contract_id.to_xdr(env);
    message.append(&contract_bytes);

    env.crypto().sha256(&message).into()
}

/// Validate that a bytesN<32> value has the correct length for an
/// Ed25519 public key.  This is a defence-in-depth check — Soroban's
/// `BytesN<32>` type already enforces length at the type level, but
/// explicit validation provides clearer error messages.
fn validate_signer_key(signer: &BytesN<32>) -> Result<(), Error> {
    // BytesN<32> is always 32 bytes by construction in Soroban, so this
    // is technically always true.  The check exists for documentation
    // purposes and as a safeguard against future type changes.
    if signer.to_buffer().len() != ED25519_PUBLIC_KEY_LEN {
        return Err(Error::InvalidSignature);
    }
    Ok(())
}

/// Validate that a signature has the correct length for Ed25519.
fn validate_signature_length(signature: &BytesN<64>) -> Result<(), Error> {
    if signature.to_buffer().len() != ED25519_SIGNATURE_LEN {
        return Err(Error::InvalidSignature);
    }
    Ok(())
}

/// Verify sweep authorization signature using Ed25519.
///
/// ## Authorization validation steps
///
/// 1. **Validate signature format** — Ensure the signature is exactly 64
///    bytes (Ed25519 signature length).
///
/// 2. **Retrieve authorized signer** — Read the Ed25519 public key from
///    contract instance storage.  Returns `AuthorizedSignerNotSet` if
///    `initialize()` was never called.
///
/// 3. **Validate signer key format** — Ensure the stored key is exactly
///    32 bytes (Ed25519 public key length).
///
/// 4. **Reconstruct signed message** — Build the exact byte sequence the
///    off-chain signer should have signed, using the *current* nonce.
///
/// 5. **Ed25519 verification** — Perform constant-time cryptographic
///    verification.  Panics on failure (aborts the entire transaction).
///
/// # Arguments
/// * `env` - Soroban environment
/// * `account` - Ephemeral account address (context for the sweep)
/// * `destination` - Destination wallet address
/// * `signature` - Ed25519 signature (64 bytes)
///
/// # Returns
/// Ok(()) if signature is valid
pub fn verify_sweep_auth(
    env: &Env,
    _account: &Address,
    destination: &Address,
    signature: &BytesN<64>,
) -> Result<(), Error> {
    // Step 1: Validate signature format
    validate_signature_length(signature)?;

    // Step 2: Retrieve authorized signer from storage
    let authorized_signer =
        storage::get_authorized_signer(env).ok_or(Error::AuthorizedSignerNotSet)?;

    // Step 3: Validate signer key format
    validate_signer_key(&authorized_signer)?;

    // Step 4: Get contract address and reconstruct message
    let contract_id = env.current_contract_address();
    let message = construct_sweep_message(env, destination, &contract_id);

    // Step 5: Ed25519 cryptographic verification (panics on failure)
    env.crypto()
        .ed25519_verify(&authorized_signer, &message.into(), signature);
    Ok(())
}

/// Increment the nonce after successful authorization.
///
/// Must be called *after* verification succeeds to prevent replay within
/// the same transaction or across transactions.
pub fn increment_nonce(env: &Env) {
    storage::increment_sweep_nonce(env);
}

/// Authorization context for sweep operations.
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
