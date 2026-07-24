use crate::errors::Error;
use crate::storage;
use soroban_sdk::{xdr::ToXdr, Address, BytesN, Env};

/// Construct the message to be signed for sweep authorization.
///
/// The off-chain signer must produce an Ed25519 signature over this exact
/// byte sequence.  Changing any component (destination, nonce, contract id)
/// produces a different hash, which invalidates the signature — this is the
/// core replay-prevention mechanism.
///
/// Message format: SHA256(destination || nonce_be64 || contract_id)
///
/// # Security notes
/// - The nonce is included to bind each signature to exactly one sweep
///   operation.  After a successful sweep the nonce increments, so the
///   same signature cannot be replayed.
/// - The contract_id (sweep controller address) is included to bind the
///   signature to this specific contract instance, preventing cross-
///   contract replay attacks.
/// - No hardcoded keys, addresses, or magic values are used.  All inputs
///   are derived from on-chain state or caller arguments.
///
/// # Arguments
/// * `env` - Soroban environment
/// * `destination` - Destination wallet address
/// * `contract_id` - The sweep controller contract address
///
/// # Returns
/// BytesN<32> containing the SHA-256 hash of the message components
fn construct_sweep_message(env: &Env, destination: &Address, contract_id: &Address) -> BytesN<32> {
    // ── 1. Read current nonce from persistent storage ──────────────────
    // The nonce starts at 0 and increments after every successful sweep.
    // Including it in the signed message ensures each signature is
    // single-use.
    let nonce = storage::get_sweep_nonce(env);

    // ── 2. Concatenate message components ──────────────────────────────
    // We build a byte buffer containing:
    //   [destination_xdr | nonce_big_endian | contract_id_xdr]
    //
    // Using XDR serialization for addresses ensures a canonical
    // byte representation that is identical across all Soroban
    // environments and SDK versions.
    let mut message = soroban_sdk::Bytes::new(env);

    // Destination address (XDR-serialized)
    let dest_bytes = destination.to_xdr(env);
    message.append(&dest_bytes);

    // Nonce as 8-byte big-endian integer — manually encoded to avoid
    // pulling in additional dependencies and to ensure byte-level
    // determinism.
    message.push_back(((nonce >> 56) & 0xFF) as u8);
    message.push_back(((nonce >> 48) & 0xFF) as u8);
    message.push_back(((nonce >> 40) & 0xFF) as u8);
    message.push_back(((nonce >> 32) & 0xFF) as u8);
    message.push_back(((nonce >> 24) & 0xFF) as u8);
    message.push_back(((nonce >> 16) & 0xFF) as u8);
    message.push_back(((nonce >> 8) & 0xFF) as u8);
    message.push_back((nonce & 0xFF) as u8);

    // Sweep controller contract address (XDR-serialized) — binds the
    // signature to this specific contract instance.
    let contract_bytes = contract_id.to_xdr(env);
    message.append(&contract_bytes);

    // ── 3. Hash with SHA-256 ──────────────────────────────────────────
    // The Ed25519 verify primitive expects a 32-byte message hash.
    env.crypto().sha256(&message).into()
}

/// Verify sweep authorization signature using Ed25519.
///
/// This function verifies that the provided signature was created by the
/// authorized signer using the private key corresponding to the authorized
/// public key stored in contract instance storage.
///
/// ## Auth check breakdown
///
/// 1. **Signer existence check** — `get_authorized_signer()` returns
///    `None` if `initialize()` was never called.  We return
///    `AuthorizedSignerNotSet` rather than panicking so the caller gets
///    a recoverable error.
///
/// 2. **Message reconstruction** — We rebuild the exact same byte sequence
///    the off-chain signer should have signed, using the *current* nonce.
///    If the nonce has advanced since the signature was produced (e.g.,
///    another sweep was executed), verification will fail — this is the
///    replay-prevention guarantee.
///
/// 3. **Ed25519 verification** — `env.crypto().ed25519_verify()` performs
///    constant-time comparison of the signature against the public key and
///    message hash.  It returns `()` on success or panics on failure.
///    We do *not* catch the panic here because a failed signature check
///    should abort the entire transaction (no partial state changes).
///
/// ## No hardcoded keys
///
/// The authorized signer public key is read from contract instance
/// storage, which was set during `initialize()` by the contract creator.
/// There are no hardcoded keys, backdoors, or bypass mechanisms anywhere
/// in this module.
///
/// # Arguments
/// * `env` - Soroban environment
/// * `account` - Ephemeral account address (context for the sweep)
/// * `destination` - Destination wallet address
/// * `signature` - Ed25519 signature (64 bytes)
///
/// # Returns
/// Ok(()) if signature is valid
///
/// # Panics
/// Panics (aborts transaction) if Ed25519 verification fails.
/// This is intentional — a failed signature check indicates either
/// tampering or a bug, and should not allow partial execution.
pub fn verify_sweep_auth(
    env: &Env,
    _account: &Address,
    destination: &Address,
    signature: &BytesN<64>,
) -> Result<(), Error> {
    // ── Step 1: Retrieve the authorized signer's public key ────────────
    // This key was stored by the contract creator during initialize().
    // If it was never set, the contract is in an invalid state and we
    // return a specific error rather than panicking on None unwrap.
    let authorized_signer =
        storage::get_authorized_signer(env).ok_or(Error::AuthorizedSignerNotSet)?;

    // ── Step 2: Get the current contract's address ─────────────────────
    // Used as a component in the signed message to bind the signature
    // to this specific contract instance (prevents cross-contract replay).
    let contract_id = env.current_contract_address();

    // ── Step 3: Reconstruct the expected signed message ────────────────
    // The off-chain signer should have signed:
    //   SHA256(destination || nonce || contract_id)
    // using the current nonce at the time of signing.
    let message = construct_sweep_message(env, destination, &contract_id);

    // ── Step 4: Ed25519 signature verification ─────────────────────────
    // This performs constant-time cryptographic verification.
    // On failure it panics, which aborts the entire Soroban transaction.
    // We do NOT catch this panic — a failed signature check is a hard
    // failure that should never allow partial state progression.
    env.crypto()
        .ed25519_verify(&authorized_signer, &message.into(), signature);
    Ok(())
}

/// Increment the nonce after successful authorization.
///
/// Must be called *after* `verify_sweep_auth()` succeeds but *before*
/// any external contract calls, so that a re-entrant call within the
/// same transaction would see the incremented nonce and fail.
///
/// # Arguments
/// * `env` - Soroban environment
pub fn increment_nonce(env: &Env) {
    storage::increment_sweep_nonce(env);
}

/// Authorization context for sweep operations.
///
/// Bundles the three components needed for authorization verification
/// and provides a single `verify()` entry point.
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

    /// Verify the sweep authorization signature.
    ///
    /// Delegates to `verify_sweep_auth()` which performs Ed25519
    /// verification against the stored authorized signer public key.
    pub fn verify(&self, env: &Env) -> Result<(), Error> {
        verify_sweep_auth(env, &self.account, &self.destination, &self.signature)
    }
}
