use crate::errors::Error;
use crate::storage;
use soroban_sdk::{xdr::ToXdr, Address, BytesN, Env};

/// Ed25519 public key length in bytes.
const ED25519_PUBLIC_KEY_LEN: usize = 32;
/// Ed25519 signature length in bytes.
const ED25519_SIGNATURE_LEN: usize = 64;

/// Construct the message to be signed for sweep authorization.
///
/// The off-chain signer must produce an Ed25519 signature over this exact
/// byte sequence.  Changing any component (destination, nonce, contract id)
/// produces a different hash, which invalidates the signature — this is the
/// core replay-prevention mechanism.
///
/// Message format: SHA256(account || destination || nonce_be64 || contract_id)
///
/// # Security notes
/// - The nonce is included to bind each signature to exactly one sweep
///   operation.  After a successful sweep the nonce increments, so the
///   same signature cannot be replayed.
/// - The contract_id (sweep controller address) is included to bind the
///   signature to this specific contract instance, preventing cross-
///   contract replay attacks.
/// - The account is included to bind the signature to a specific ephemeral
///   account, preventing cross-account replay attacks. (#29)
/// - No hardcoded keys, addresses, or magic values are used.  All inputs
///   are derived from on-chain state or caller arguments.
///
/// # Arguments
/// * `env` - Soroban environment
/// * `account` - Ephemeral account the sweep authorizes
/// * `destination` - Destination wallet address
/// * `contract_id` - The sweep controller contract address
///
/// # Returns
/// BytesN<32> containing the SHA-256 hash of the message components
fn construct_sweep_message(
    env: &Env,
    account: &Address,
    destination: &Address,
    contract_id: &Address,
) -> BytesN<32> {
    // ── 1. Read current nonce from persistent storage ──────────────────
    // The nonce starts at 0 and increments after every successful sweep.
    // Including it in the signed message ensures each signature is
    // single-use.
    let nonce = storage::get_sweep_nonce(env);

    // ── 2. Concatenate message components ──────────────────────────────
    // We build a byte buffer containing:
    //   [account_xdr | destination_xdr | nonce_big_endian | contract_id_xdr]
    //
    // Using XDR serialization for addresses ensures a canonical
    // byte representation that is identical across all Soroban
    // environments and SDK versions.
    let mut message = soroban_sdk::Bytes::new(env);

    // Ephemeral account address (XDR-serialized) — binds the signature
    // to a specific account so it cannot be replayed on another account.
    let account_bytes = account.to_xdr(env);
    message.append(&account_bytes);

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

/// Validate that a bytesN<32> value has the correct length for an
/// Ed25519 public key.  This is a defence-in-depth check — Soroban's
/// `BytesN<32>` type already enforces length at the type level, but
/// explicit validation provides clearer error messages.
fn validate_signer_key(signer: &BytesN<32>) -> Result<(), Error> {
    // BytesN<32> is always 32 bytes by construction in Soroban, so this
    // is technically always true.  The check exists for documentation
    // purposes and as a safeguard against future type changes.
    if signer.to_array().len() != ED25519_PUBLIC_KEY_LEN {
        return Err(Error::InvalidSignature);
    }
    Ok(())
}

/// Validate that a signature has the correct length for Ed25519.
fn validate_signature_length(signature: &BytesN<64>) -> Result<(), Error> {
    if signature.to_array().len() != ED25519_SIGNATURE_LEN {
        return Err(Error::InvalidSignature);
    }
    Ok(())
}

/// Verify sweep authorization signature using Ed25519.
///
/// This function verifies that the provided signature was created by the
/// authorized signer using the private key corresponding to the authorized
/// public key stored in contract instance storage.
///
/// ## Auth check breakdown
///
/// 1. **Validate signature format** — Ensure the signature is exactly 64
///    bytes (Ed25519 signature length).
///
/// 2. **Signer existence check** — `get_authorized_signer()` returns
///    `None` if `initialize()` was never called.  We return
///    `AuthorizedSignerNotSet` rather than panicking so the caller gets
///    a recoverable error.
///
/// 3. **Validate signer key format** — Ensure the stored key is exactly
///    32 bytes (Ed25519 public key length).
///
/// 4. **Message reconstruction** — We rebuild the exact same byte sequence
///    the off-chain signer should have signed, using the *current* nonce.
///    If the nonce has advanced since the signature was produced (e.g.,
///    another sweep was executed), verification will fail — this is the
///    replay-prevention guarantee.
///
/// 5. **Ed25519 verification** — `env.crypto().ed25519_verify()` performs
///    constant-time comparison of the signature against the public key and
///    message hash.  **On failure it panics (aborts the transaction), it
///    does NOT return an error.** We do *not* catch the panic here because
///    a failed signature check should abort the entire transaction (no
///    partial state changes).
///
/// ## Return type caveat
///
/// This function returns `Result<(), Error>` for ergonomics, but in
/// practice it can only return:
/// - `Ok(())` — signature valid
/// - `Err(AuthorizedSignerNotSet)` — signer key not configured
/// - `Err(InvalidSignature)` — format validation failed
///
/// A bad Ed25519 signature will **panic** via `ed25519_verify()` rather
/// than producing `Err(AuthorizationFailed)` or `Err(SignatureVerificationFailed)`.
/// Callers that need to observe signature failures as typed errors should
/// use a try-call wrapper or handle the panic at the transaction level. (#411)
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
    account: &Address,
    destination: &Address,
    signature: &BytesN<64>,
) -> Result<(), Error> {
    // Step 1: Validate signature format
    validate_signature_length(signature)?;

    // ── Step 2: Retrieve the authorized signer's public key ────────────
    // This key was stored by the contract creator during initialize().
    // If it was never set, the contract is in an invalid state and we
    // return a specific error rather than panicking on None unwrap.
    let authorized_signer =
        storage::get_authorized_signer(env).ok_or(Error::AuthorizedSignerNotSet)?;

    // Step 3: Validate signer key format
    validate_signer_key(&authorized_signer)?;

    // ── Step 4: Get the current contract's address ─────────────────────
    // Used as a component in the signed message to bind the signature
    // to this specific contract instance (prevents cross-contract replay).
    let contract_id = env.current_contract_address();

    // ── Step 5: Reconstruct the expected signed message ────────────────
    // The off-chain signer should have signed:
    //   SHA256(account || destination || nonce || contract_id)
    // using the current nonce at the time of signing.
    let message = construct_sweep_message(env, account, destination, &contract_id);

    // ── Step 6: Ed25519 signature verification ─────────────────────────
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
